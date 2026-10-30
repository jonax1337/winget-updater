use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use base64::{Engine, engine::general_purpose::STANDARD};
use tauri::Manager;

use crate::error::{AppError, AppResult};
use crate::models::{ManagerId, PackageRef};

mod http;
mod metadata;
mod online;
mod raster;
mod site_icon;

const INDEX_TTL: Duration = Duration::from_secs(10 * 60);
const ICON_SIZE: u32 = 32;
// Windows limits a file name to 255 characters; the hex encoding doubles the id length.
const MAX_READABLE_ID_LEN: usize = 200;

/// Returns a PNG data URL from the local installation or, failing that, the publisher's website.
pub fn package_icon(
    app: &tauri::AppHandle,
    package: &crate::models::PackageRef,
    name: &str,
) -> crate::error::AppResult<Option<String>> {
    let directory = app
        .path()
        .app_cache_dir()
        .map_err(storage_error)?
        .join("icons");
    fs::create_dir_all(&directory).map_err(storage_error)?;
    let cache = directory.join(cache_filename(package));
    if let Some(cached) = read_cached_icon(&cache)? {
        return Ok(cached);
    }
    let png = find_icon(package, name)?;
    fs::write(cache, png.as_deref().unwrap_or_default()).map_err(storage_error)?;
    Ok(png.map(|bytes| data_url(&bytes)))
}

fn storage_error(error: impl std::fmt::Display) -> AppError {
    AppError::Storage(format!("Icon-Cache: {error}"))
}

fn data_url(bytes: &[u8]) -> String {
    format!("data:image/png;base64,{}", STANDARD.encode(bytes))
}

fn cache_filename(package: &PackageRef) -> String {
    let manager = package.manager.as_str();
    let readable: String = package
        .id
        .bytes()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    // The "h" prefix cannot occur in hex, so a hash never collides with a readable name.
    let id = if readable.len() <= MAX_READABLE_ID_LEN {
        readable
    } else {
        format!("h{:016x}", fnv1a_64(package.id.as_bytes()))
    };
    format!("{manager}_{id}.png")
}

// Hand-rolled because `DefaultHasher` may change between Rust versions and orphan cached files.
fn fnv1a_64(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;
    bytes.iter().fold(OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
}

// The outer option distinguishes a cache miss from a cached negative lookup.
fn read_cached_icon(path: &Path) -> AppResult<Option<Option<String>>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(storage_error(error)),
    };
    if !bytes.is_empty() {
        return Ok(Some(Some(data_url(&bytes))));
    }
    let modified = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map_err(storage_error)?;
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default();
    Ok((age < INDEX_TTL).then_some(None))
}

#[derive(Clone, Debug)]
struct InstalledApp {
    normalized_name: String,
    display_icon: String,
    install_location: PathBuf,
}

struct RegistryIndex {
    entries: Vec<InstalledApp>,
    refreshed_at: Instant,
}

static REGISTRY_INDEX: OnceLock<Mutex<RegistryIndex>> = OnceLock::new();

fn normalize_name(name: &str) -> String {
    name.chars()
        .flat_map(char::to_lowercase)
        .filter(|character| character.is_alphanumeric())
        .collect()
}

fn match_rank(candidate: &str, query: &str) -> u8 {
    if query.is_empty() || candidate.is_empty() {
        0
    } else if candidate == query {
        3
    } else if candidate.starts_with(query) {
        2
    } else if candidate.contains(query) {
        1
    } else {
        0
    }
}

fn matching_apps(entries: &[InstalledApp], package: &PackageRef, name: &str) -> Vec<InstalledApp> {
    let name = normalize_name(name);
    let id = normalize_name(package.id.rsplit('.').next().unwrap_or(&package.id));
    let mut matches: Vec<_> = entries
        .iter()
        .filter_map(|entry| {
            let rank = match_rank(&entry.normalized_name, &name)
                .max(match_rank(&entry.normalized_name, &id));
            (rank > 0).then_some((rank, entry))
        })
        .collect();
    matches.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    matches
        .into_iter()
        .map(|(_, entry)| entry.clone())
        .collect()
}

fn registry_matches(package: &PackageRef, name: &str) -> AppResult<Vec<InstalledApp>> {
    let index = REGISTRY_INDEX.get_or_init(|| {
        Mutex::new(RegistryIndex {
            entries: read_registry(),
            refreshed_at: Instant::now(),
        })
    });
    let mut index = index
        .lock()
        .map_err(|_| AppError::Other("Icon-Registry-Cache gesperrt".into()))?;
    if index_stale(index.refreshed_at, Instant::now()) {
        index.entries = read_registry();
        index.refreshed_at = Instant::now();
    }
    Ok(matching_apps(&index.entries, package, name))
}

fn index_stale(refreshed_at: Instant, now: Instant) -> bool {
    now.saturating_duration_since(refreshed_at) >= INDEX_TTL
}

#[cfg(test)]
#[test]
fn index_expires_at_ttl() {
    let refreshed = Instant::now();
    assert!(!index_stale(
        refreshed,
        refreshed + INDEX_TTL - Duration::from_nanos(1)
    ));
    assert!(index_stale(refreshed, refreshed + INDEX_TTL));
    assert!(index_stale(
        refreshed,
        refreshed + INDEX_TTL + Duration::from_secs(1)
    ));
}

#[cfg(windows)]
fn read_registry() -> Vec<InstalledApp> {
    use winreg::{RegKey, enums::*};
    const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    const WOW_UNINSTALL: &str = r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";
    let mut entries = Vec::new();
    for (hive, path) in [
        (HKEY_LOCAL_MACHINE, UNINSTALL),
        (HKEY_LOCAL_MACHINE, WOW_UNINSTALL),
        (HKEY_CURRENT_USER, UNINSTALL),
    ] {
        let Ok(root) =
            RegKey::predef(hive).open_subkey_with_flags(path, KEY_READ | KEY_WOW64_64KEY)
        else {
            continue;
        };
        for key in root.enum_keys().flatten() {
            let Ok(key) = root.open_subkey_with_flags(key, KEY_READ | KEY_WOW64_64KEY) else {
                continue;
            };
            let Ok(name) = key.get_value::<String, _>("DisplayName") else {
                continue;
            };
            entries.push(InstalledApp {
                normalized_name: normalize_name(&name),
                display_icon: key.get_value("DisplayIcon").unwrap_or_default(),
                install_location: PathBuf::from(expand_environment(
                    &key.get_value::<String, _>("InstallLocation")
                        .unwrap_or_default(),
                )),
            });
        }
    }
    entries
}

#[cfg(not(windows))]
fn read_registry() -> Vec<InstalledApp> {
    Vec::new()
}

#[derive(Debug, PartialEq)]
struct IconSource {
    path: PathBuf,
    index: i32,
}

fn parse_display_icon(value: &str) -> Option<IconSource> {
    let value = value.trim().trim_matches('"').trim();
    let (path, index) = match value.rsplit_once(',') {
        Some((path, index)) => match index.trim().parse::<i32>() {
            Ok(index) => (path.trim(), index),
            Err(_) => (value, 0),
        },
        None => (value, 0),
    };
    let path = path.trim_matches('"').trim();
    if path.is_empty() {
        return None;
    }
    Some(IconSource {
        path: PathBuf::from(expand_environment(path)),
        index,
    })
}

fn expand_environment(value: &str) -> String {
    let mut expanded = String::new();
    let mut rest = value;
    while let Some(start) = rest.find('%') {
        expanded.push_str(&rest[..start]);
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('%') else {
            expanded.push_str(&rest[start..]);
            return expanded;
        };
        let variable = &after_start[..end];
        expanded.push_str(&std::env::var(variable).unwrap_or_else(|_| format!("%{variable}%")));
        rest = &after_start[end + 1..];
    }
    expanded.push_str(rest);
    expanded
}

fn first_executable(directory: &Path) -> Option<IconSource> {
    let mut paths: Vec<_> = fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        })
        .collect();
    paths.sort();
    paths
        .into_iter()
        .next()
        .map(|path| IconSource { path, index: 0 })
}

fn is_plain_path_segment(id: &str) -> bool {
    !id.is_empty()
        && id.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
        && !matches!(id, "." | "..")
}

fn scoop_root() -> Option<PathBuf> {
    std::env::var_os("SCOOP")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join("scoop")))
}

fn scoop_current_directory(id: &str) -> Option<PathBuf> {
    // Package IDs must not escape the Scoop apps directory.
    is_plain_path_segment(id).then_some(())?;
    Some(scoop_root()?.join("apps").join(id).join("current"))
}

fn scoop_source(id: &str) -> Option<IconSource> {
    first_executable(&scoop_current_directory(id)?)
}

fn find_icon(package: &PackageRef, name: &str) -> AppResult<Option<Vec<u8>>> {
    if let Some(png) = find_local_icon(package, name)? {
        return Ok(Some(png));
    }
    Ok(online::download_icon(package))
}

fn find_local_icon(package: &PackageRef, name: &str) -> AppResult<Option<Vec<u8>>> {
    match package.manager {
        ManagerId::Npm | ManagerId::Pip => Ok(None),
        ManagerId::Scoop => scoop_source(&package.id)
            .map(|source| extract_png(&source))
            .transpose()
            .map(Option::flatten),
        ManagerId::Winget | ManagerId::Chocolatey => registry_icon(package, name),
    }
}

fn registry_icon(package: &PackageRef, name: &str) -> AppResult<Option<Vec<u8>>> {
    for entry in registry_matches(package, name)? {
        if let Some(source) = parse_display_icon(&entry.display_icon)
            && let Some(png) = extract_png(&source)?
        {
            return Ok(Some(png));
        }
        if let Some(source) = first_executable(&entry.install_location)
            && let Some(png) = extract_png(&source)?
        {
            return Ok(Some(png));
        }
    }
    Ok(None)
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::UI::Shell::ExtractIconExW;
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::core::PCWSTR;

    struct OwnedIcon(HICON);
    impl Drop for OwnedIcon {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyIcon(self.0);
            }
        }
    }

    struct OwnedBitmap(HBITMAP);
    impl Drop for OwnedBitmap {
        fn drop(&mut self) {
            unsafe {
                let _ = DeleteObject(HGDIOBJ(self.0.0));
            }
        }
    }

    struct OwnedDc(HDC);
    impl Drop for OwnedDc {
        fn drop(&mut self) {
            unsafe {
                let _ = DeleteDC(self.0);
            }
        }
    }

    pub(super) fn extract(source: &IconSource) -> Option<Vec<u8>> {
        let icon = load_icon(source)?;
        let mut info = ICONINFO::default();
        unsafe {
            GetIconInfo(icon.0, &mut info).ok()?;
        }
        let color = OwnedBitmap(info.hbmColor);
        let mask = OwnedBitmap(info.hbmMask);
        let dc = OwnedDc(unsafe { CreateCompatibleDC(None) });
        if dc.0.is_invalid() {
            return None;
        }
        let (width, height, mut pixels) = if color.0.is_invalid() {
            monochrome_pixels(dc.0, mask.0)?
        } else {
            bitmap_pixels(dc.0, color.0)?
        };
        let alpha_present = pixels.as_chunks::<4>().0.iter().any(|pixel| pixel[3] != 0);
        let mask_pixels = if alpha_present {
            None
        } else {
            bitmap_pixels(dc.0, mask.0)
        };
        convert_bgra(
            &mut pixels,
            mask_pixels.as_ref().map(|(_, _, bytes)| bytes.as_slice()),
        );
        Some(resize_rgba(&pixels, width, height))
    }

    fn load_icon(source: &IconSource) -> Option<OwnedIcon> {
        let path: Vec<u16> = source
            .path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let path = PCWSTR(path.as_ptr());
        if source
            .path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("ico"))
        {
            let handle = unsafe {
                LoadImageW(
                    None,
                    path,
                    IMAGE_ICON,
                    ICON_SIZE as i32,
                    ICON_SIZE as i32,
                    LR_LOADFROMFILE,
                )
            }
            .ok()?;
            return Some(OwnedIcon(HICON(handle.0)));
        }
        let mut large = HICON::default();
        let count = unsafe { ExtractIconExW(path, source.index, Some(&mut large), None, 1) };
        if count == 0 || large.is_invalid() {
            return None;
        }
        Some(OwnedIcon(large))
    }

    fn bitmap_pixels(dc: HDC, bitmap: HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
        let mut object = BITMAP::default();
        if unsafe {
            GetObjectW(
                HGDIOBJ(bitmap.0),
                size_of::<BITMAP>() as i32,
                Some((&mut object as *mut BITMAP).cast()),
            )
        } == 0
        {
            return None;
        }
        let width = u32::try_from(object.bmWidth).ok()?;
        let height = object.bmHeight.unsigned_abs();
        if width == 0 || height == 0 || width > 1024 || height > 2048 {
            return None;
        }
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0; width as usize * height as usize * 4];
        let rows = unsafe {
            GetDIBits(
                dc,
                bitmap,
                0,
                height,
                Some(pixels.as_mut_ptr().cast()),
                &mut info,
                DIB_RGB_COLORS,
            )
        };
        (rows == height as i32).then_some((width, height, pixels))
    }

    fn monochrome_pixels(dc: HDC, bitmap: HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
        let (width, double_height, mask) = bitmap_pixels(dc, bitmap)?;
        let height = double_height / 2;
        if height == 0 {
            return None;
        }
        let offset = width as usize * height as usize * 4;
        let mut pixels = mask[offset..offset * 2].to_vec();
        for (pixel, and_mask) in pixels
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(mask[..offset].as_chunks::<4>().0.iter())
        {
            pixel[3] = if and_mask[0] == 0 { 255 } else { 0 };
        }
        Some((width, height, pixels))
    }
}

fn convert_bgra(pixels: &mut [u8], mask: Option<&[u8]>) {
    let alpha_present = pixels.as_chunks::<4>().0.iter().any(|pixel| pixel[3] != 0);
    for (index, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        pixel.swap(0, 2);
        if !alpha_present {
            pixel[3] = mask
                .and_then(|mask| mask.get(index * 4))
                .map_or(255, |value| if *value == 0 { 255 } else { 0 });
        }
    }
}

fn resize_rgba(pixels: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut resized = vec![0; (ICON_SIZE * ICON_SIZE * 4) as usize];
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let source = (((y * height / ICON_SIZE) * width + x * width / ICON_SIZE) * 4) as usize;
            let target = ((y * ICON_SIZE + x) * 4) as usize;
            resized[target..target + 4].copy_from_slice(&pixels[source..source + 4]);
        }
    }
    resized
}

fn extract_png(source: &IconSource) -> AppResult<Option<Vec<u8>>> {
    #[cfg(windows)]
    {
        native::extract(source)
            .map(|pixels| encode_png(&pixels))
            .transpose()
    }
    #[cfg(not(windows))]
    {
        let _ = source;
        Ok(None)
    }
}

fn encode_png(pixels: &[u8]) -> AppResult<Vec<u8>> {
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, ICON_SIZE, ICON_SIZE);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|error| AppError::Other(format!("Icon-PNG: {error}")))?;
        writer
            .write_image_data(pixels)
            .map_err(|error| AppError::Other(format!("Icon-PNG: {error}")))?;
    }
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn icons_extract_system_icon_as_32_pixel_png() {
        let root = std::env::var_os("SystemRoot").expect("Windows system directory");
        let source = IconSource {
            path: PathBuf::from(root).join("System32/shell32.dll"),
            index: 0,
        };
        let png = extract_png(&source)
            .expect("PNG encoding")
            .expect("system icon");
        let reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .expect("valid PNG");
        assert_eq!((reader.info().width, reader.info().height), (32, 32));
        assert_eq!(reader.info().color_type, png::ColorType::Rgba);
    }

    fn package_with_id(id: String) -> PackageRef {
        PackageRef {
            manager: ManagerId::Winget,
            id,
        }
    }

    #[test]
    fn icons_cache_filename_keeps_short_ids_readable() {
        assert_eq!(
            cache_filename(&package_with_id("a.B".into())),
            "winget_612e42.png"
        );
    }

    #[test]
    fn icons_cache_filename_hashes_long_ids_to_a_stable_short_name() {
        let first = cache_filename(&package_with_id("a".repeat(150)));
        let second = cache_filename(&package_with_id("b".repeat(150)));
        assert!(first.len() < 255);
        assert_eq!(first, cache_filename(&package_with_id("a".repeat(150))));
        assert_ne!(first, second);
    }

    #[test]
    fn icons_fnv1a_matches_reference_vectors() {
        assert_eq!(fnv1a_64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a_64(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn icons_normalize_names_without_punctuation() {
        assert_eq!(
            normalize_name("Microsoft Visual Studio Code (User)"),
            "microsoftvisualstudiocodeuser"
        );
        assert_eq!(normalize_name("ÄÖ-123"), "äö123");
    }

    #[test]
    fn icons_matching_prefers_exact_then_prefix_then_contains() {
        assert!(match_rank("code", "code") > match_rank("codeuser", "code"));
        assert!(match_rank("codeuser", "code") > match_rank("visualcode", "code"));
        assert_eq!(match_rank("code", ""), 0);
        assert_eq!(match_rank("other", "code"), 0);
    }

    #[test]
    fn icons_matching_uses_winget_id_suffix() {
        let entries = vec![InstalledApp {
            normalized_name: "wsl".into(),
            display_icon: String::new(),
            install_location: PathBuf::new(),
        }];
        let package = PackageRef {
            manager: ManagerId::Winget,
            id: "Microsoft.WSL".into(),
        };
        assert_eq!(
            matching_apps(&entries, &package, "Windows Subsystem for Linux").len(),
            1
        );
    }

    #[test]
    fn icons_parse_quoted_executable_and_negative_resource_index() {
        assert_eq!(
            parse_display_icon(r#"  "C:\Program Files\app.exe",-42 "#),
            Some(IconSource {
                path: PathBuf::from(r"C:\Program Files\app.exe"),
                index: -42
            })
        );
    }

    #[test]
    fn icons_parse_direct_ico_and_paths_with_commas() {
        assert_eq!(
            parse_display_icon(r#""C:\icons\a,b.ico""#),
            Some(IconSource {
                path: PathBuf::from(r"C:\icons\a,b.ico"),
                index: 0
            })
        );
        assert_eq!(parse_display_icon(r"C:\icons\app.exe,0").unwrap().index, 0);
        assert_eq!(
            parse_display_icon(r#""C:\icons\app.exe,3""#).unwrap().index,
            3
        );
        assert_eq!(parse_display_icon(" "), None);
    }

    #[test]
    fn icons_bgra_preserves_alpha_and_uses_legacy_mask() {
        let mut transparent = [3, 2, 1, 0, 6, 5, 4, 0];
        convert_bgra(&mut transparent, Some(&[0, 0, 0, 0, 255, 255, 255, 0]));
        assert_eq!(transparent, [1, 2, 3, 255, 4, 5, 6, 0]);
        let mut alpha = [30, 20, 10, 128];
        convert_bgra(&mut alpha, None);
        assert_eq!(alpha, [10, 20, 30, 128]);
    }
}
