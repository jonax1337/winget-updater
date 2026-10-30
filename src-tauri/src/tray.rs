use tauri::{
    AppHandle, Manager,
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::engine;
use crate::error::{AppError, AppResult};

const TRAY_ID: &str = "main-tray";
const LIGHT_TRAY: Image<'static> = tauri::include_image!("icons/tray-light.png");
const DARK_TRAY: Image<'static> = tauri::include_image!("icons/tray-dark.png");

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let scan = MenuItem::with_id(app, "scan", "Check now", true, None::<&str>)?;
    let update = MenuItem::with_id(app, "update", "Update all", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &scan, &update, &separator, &quit])?;
    let builder = TrayIconBuilder::with_id(TRAY_ID)
        .icon(tray_icon())
        .menu(&menu)
        .tooltip(tooltip(0))
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_menu(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        });
    builder.build(app)?;
    Ok(())
}

fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        "open" => show_main_window(app),
        "scan" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = engine::scan(&app).await {
                    engine::report_error(&app, &error);
                }
            });
        }
        "update" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = engine::update_all(&app).await {
                    engine::report_error(&app, &error);
                }
            });
        }
        "quit" => {
            let run = engine::get_run_state(app);
            if run.scanning || run.updating {
                engine::report_error(
                    app,
                    &AppError::Other(
                        "Wait for the current operation to finish before quitting.".into(),
                    ),
                );
            } else {
                app.exit(0);
            }
        }
        _ => {}
    }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        for result in [window.show(), window.unminimize(), window.set_focus()] {
            if let Err(error) = result {
                eprintln!("Could not open window: {error}");
            }
        }
    }
}

/// Shows the count in the tooltip and a dot on the icon while updates are pending.
pub fn show_pending_updates(app: &AppHandle, pending_updates: u32) -> AppResult<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    tray.set_tooltip(Some(tooltip(pending_updates)))
        .map_err(tray_error)?;
    let icon = tray_icon();
    let icon = if pending_updates > 0 {
        Image::new_owned(
            with_badge(icon.rgba(), icon.width()),
            icon.width(),
            icon.height(),
        )
    } else {
        icon
    };
    tray.set_icon(Some(icon)).map_err(tray_error)?;
    Ok(())
}

/// The Windows taskbar theme is independent of the app's light/dark preference.
fn tray_icon() -> Image<'static> {
    if taskbar_is_light() {
        LIGHT_TRAY
    } else {
        DARK_TRAY
    }
}

#[cfg(windows)]
fn taskbar_is_light() -> bool {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|key| key.get_value::<u32, _>("SystemUsesLightTheme"))
        .unwrap_or(0)
        != 0
}

#[cfg(not(windows))]
fn taskbar_is_light() -> bool {
    false
}

fn tooltip(pending_updates: u32) -> String {
    match pending_updates {
        0 => "Winget Updater – up to date".into(),
        1 => "Winget Updater – 1 update available".into(),
        count => format!("Winget Updater – {count} Updates available"),
    }
}

fn tray_error(error: tauri::Error) -> AppError {
    AppError::Other(format!("Tray: {error}"))
}

const BADGE_COLOR: [u8; 3] = [0xC9, 0xF4, 0x6A];
const BADGE_RING: [u8; 3] = [0x18, 0x1B, 0x19];
/// A graphite outline keeps the lime dot visible on both taskbar themes.
const BADGE_RADIUS: f32 = 0.17;
const BADGE_GAP: f32 = 0.05;

/// Draws an anti-aliased dot into the bottom-right corner of a square RGBA icon.
fn with_badge(rgba: &[u8], width: u32) -> Vec<u8> {
    let size = width as f32;
    let radius = size * BADGE_RADIUS;
    let center = size - radius;
    let mut pixels = rgba.to_vec();
    for (index, pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let x = (index as u32 % width) as f32 + 0.5;
        let y = (index as u32 / width) as f32 + 0.5;
        let distance = (x - center).hypot(y - center);
        blend_over(
            pixel,
            BADGE_RING,
            coverage(radius + size * BADGE_GAP - distance),
        );
        blend_over(pixel, BADGE_COLOR, coverage(radius - distance));
    }
    pixels
}

/// Fraction of a pixel covered by a shape whose edge is `signed_distance` pixels away.
fn coverage(signed_distance: f32) -> f32 {
    (signed_distance + 0.5).clamp(0.0, 1.0)
}

fn blend_over(pixel: &mut [u8], color: [u8; 3], alpha: f32) {
    let below = f32::from(pixel[3]) / 255.0 * (1.0 - alpha);
    let combined = alpha + below;
    if combined <= 0.0 {
        return;
    }
    for channel in 0..3 {
        let mixed = f32::from(color[channel]) * alpha + f32::from(pixel[channel]) * below;
        pixel[channel] = (mixed / combined).round() as u8;
    }
    pixel[3] = (combined * 255.0).round() as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTH: u32 = 32;

    fn pixel_at(pixels: &[u8], x: u32, y: u32) -> &[u8] {
        let start = ((y * WIDTH + x) * 4) as usize;
        &pixels[start..start + 4]
    }

    #[test]
    fn badge_paints_bottom_right_and_keeps_top_left() {
        let icon = [255u8; (WIDTH * WIDTH * 4) as usize];
        let badged = with_badge(&icon, WIDTH);
        assert_eq!(pixel_at(&badged, 0, 0), [255, 255, 255, 255]);
        assert_eq!(pixel_at(&badged, 24, 24), [0xC9, 0xF4, 0x6A, 255]);
        assert_eq!(pixel_at(&badged, 20, 26), [0x18, 0x1B, 0x19, 255]);
    }

    #[test]
    fn tray_variants_have_matching_transparent_silhouettes() {
        assert_eq!((LIGHT_TRAY.width(), LIGHT_TRAY.height()), (WIDTH, WIDTH));
        assert_eq!((DARK_TRAY.width(), DARK_TRAY.height()), (WIDTH, WIDTH));
        assert_eq!(pixel_at(LIGHT_TRAY.rgba(), 0, 0)[3], 0);
        let mut opaque_pixels = 0;
        for (light, dark) in LIGHT_TRAY
            .rgba()
            .as_chunks::<4>().0.iter()
            .zip(DARK_TRAY.rgba().as_chunks::<4>().0.iter())
        {
            assert_eq!(light[3], dark[3]);
            if light[3] == 255 {
                opaque_pixels += 1;
                assert_eq!(&light[..3], &[0x18, 0x1B, 0x19]);
                assert_eq!(&dark[..3], &[0xF3, 0xF5, 0xF0]);
            }
        }
        assert!(opaque_pixels > 100);
    }

    #[test]
    fn tooltip_uses_singular_and_plural() {
        assert_eq!(tooltip(0), "Winget Updater – up to date");
        assert_eq!(tooltip(1), "Winget Updater – 1 update available");
        assert_eq!(tooltip(3), "Winget Updater – 3 Updates available");
    }
}
