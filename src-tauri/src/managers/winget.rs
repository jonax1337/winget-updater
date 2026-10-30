use std::collections::HashSet;

use serde::Deserialize;

use super::{Installed, PackageManager, Upgrades};
use crate::error::{AppError, AppResult};
use crate::models::{InstalledPackage, ManagerId, Package};
use crate::process::{self, CommandOutput, LIST_TIMEOUT, UNINSTALL_TIMEOUT, UPDATE_TIMEOUT};

const NO_APPLICABLE_UPGRADE: u32 = 0x8A15002B;
const POWERSHELL_PRELUDE: &str = r#"$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); if (-not (Get-Module -ListAvailable Microsoft.WinGet.Client)) { exit 42 }; Import-Module Microsoft.WinGet.Client; "#;
const POWERSHELL_UPGRADABLE_QUERY: &str = r#"$packages=@(Get-WinGetPackage | Where-Object IsUpdateAvailable | Select-Object Id,Name,InstalledVersion,@{Name='AvailableVersion';Expression={@($_.AvailableVersions)[0]}},Source); ConvertTo-Json -InputObject $packages -Compress"#;
const POWERSHELL_INSTALLED_QUERY: &str = r#"$packages=@(Get-WinGetPackage | Select-Object Id,Name,InstalledVersion,Source); ConvertTo-Json -InputObject $packages -Compress"#;

pub struct Winget;

impl PackageManager for Winget {
    fn id(&self) -> ManagerId {
        ManagerId::Winget
    }
    fn display_name(&self) -> &'static str {
        "WinGet"
    }
    fn requires_admin(&self) -> bool {
        false
    }

    fn detect(&self) -> AppResult<Option<String>> {
        process::detect_version("winget", &["--version"])
    }

    fn list_upgrades(&self, include_unknown_versions: bool) -> AppResult<Upgrades> {
        let mut warnings = Vec::new();
        if let Ok(output) = run_powershell(POWERSHELL_UPGRADABLE_QUERY)
            && output.success
        {
            match parse_powershell_packages(&output.stdout) {
                Ok(mut packages) => {
                    if !include_unknown_versions {
                        packages.retain(|package| !unknown_version(&package.current_version));
                    }
                    return Ok(packages.into());
                }
                Err(error) => warnings.push(powershell_fallback_warning(&error)),
            }
        }
        let mut args = vec![
            "upgrade",
            "--accept-source-agreements",
            "--disable-interactivity",
        ];
        if include_unknown_versions {
            args.push("--include-unknown");
        }
        let output = process::run("winget", &args, LIST_TIMEOUT)?;
        if no_applicable_upgrade(&output) {
            return Ok(Upgrades {
                packages: Vec::new(),
                warnings,
            });
        }
        let output = output.ensure_success("winget upgrade")?;
        let mut upgrades = parse_winget_table(&output.stdout)?;
        upgrades.warnings.extend(warnings);
        Ok(upgrades)
    }

    fn upgrade(&self, package_id: &str) -> AppResult<()> {
        process::validate_package_id(package_id)?;
        let args = upgrade_args(package_id);
        let output = process::run("winget", &args, UPDATE_TIMEOUT)?;
        if no_applicable_upgrade(&output) {
            return Err(AppError::Command(
                "winget upgrade: no applicable upgrade found (0x8A15002B)".into(),
            ));
        }
        output.ensure_success("winget upgrade")?;
        Ok(())
    }

    fn list_installed(&self) -> AppResult<Installed> {
        let mut warnings = Vec::new();
        if let Ok(output) = run_powershell(POWERSHELL_INSTALLED_QUERY)
            && output.success
        {
            match parse_powershell_installed(&output.stdout) {
                Ok(packages) => return Ok(Installed { packages, warnings }),
                Err(error) => warnings.push(powershell_fallback_warning(&error)),
            }
        }
        let output = process::run(
            "winget",
            &[
                "list",
                "--accept-source-agreements",
                "--disable-interactivity",
            ],
            LIST_TIMEOUT,
        )?
        .ensure_success("winget list")?;
        let mut installed = parse_winget_installed(&output.stdout)?;
        installed.warnings.extend(warnings);
        Ok(installed)
    }

    fn uninstall(&self, package_id: &str) -> AppResult<()> {
        validate_installed_id(package_id)?;
        process::run("winget", &uninstall_args(package_id), UNINSTALL_TIMEOUT)?
            .ensure_success("winget uninstall")?;
        Ok(())
    }
}

fn run_powershell(query: &str) -> AppResult<CommandOutput> {
    let script = format!("{POWERSHELL_PRELUDE}{query}");
    process::run(
        "powershell.exe",
        &["-NoProfile", "-NonInteractive", "-Command", &script],
        LIST_TIMEOUT,
    )
}

fn powershell_fallback_warning(error: &AppError) -> String {
    format!("Could not read WinGet PowerShell output – using the table fallback: {error}")
}

fn upgrade_args(id: &str) -> Vec<&str> {
    vec![
        "upgrade",
        "--id",
        id,
        "--exact",
        "--silent",
        "--accept-package-agreements",
        "--accept-source-agreements",
        "--disable-interactivity",
        "--include-unknown",
    ]
}

fn uninstall_args(id: &str) -> Vec<&str> {
    vec![
        "uninstall",
        "--id",
        id,
        "--exact",
        "--silent",
        "--accept-source-agreements",
        "--disable-interactivity",
    ]
}

/// Installed software also carries IDs like `ARP\Machine\X64\{GUID}` or names with spaces,
/// so only option injection and control characters are rejected.
fn validate_installed_id(package_id: &str) -> AppResult<()> {
    let valid = !package_id.trim().is_empty()
        && !package_id.starts_with('-')
        && !package_id.chars().any(char::is_control);
    if valid {
        return Ok(());
    }
    Err(process::invalid_package_id(package_id))
}

fn no_applicable_upgrade(output: &CommandOutput) -> bool {
    output.exit_code.map(|code| code as u32) == Some(NO_APPLICABLE_UPGRADE)
}

fn unknown_version(version: &str) -> bool {
    version.is_empty()
        || ["unknown", "unbekannt", "<unknown>"]
            .iter()
            .any(|unknown| version.eq_ignore_ascii_case(unknown))
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PowershellPackage {
    id: String,
    name: String,
    installed_version: Option<String>,
    available_version: Option<String>,
    source: Option<String>,
}

fn parse_powershell_rows(output: &str) -> AppResult<Vec<PowershellPackage>> {
    let value: serde_json::Value =
        serde_json::from_str(output.trim().trim_start_matches('\u{feff}'))
            .map_err(|error| AppError::Parse(format!("WinGet-PowerShell: {error}")))?;
    let rows = match value {
        serde_json::Value::Null => Vec::new(),
        serde_json::Value::Array(rows) => rows,
        value => vec![value],
    };
    rows.into_iter()
        .map(|row| {
            serde_json::from_value(row)
                .map_err(|error| AppError::Parse(format!("WinGet PowerShell package: {error}")))
        })
        .collect()
}

fn parse_powershell_packages(output: &str) -> AppResult<Vec<Package>> {
    Ok(parse_powershell_rows(output)?
        .into_iter()
        .filter_map(upgradable_package)
        .collect())
}

fn upgradable_package(row: PowershellPackage) -> Option<Package> {
    let available_version = row
        .available_version
        .filter(|value| !value.trim().is_empty())?;
    Some(Package {
        manager: ManagerId::Winget,
        id: row.id,
        name: row.name,
        current_version: row.installed_version.unwrap_or_else(|| "Unknown".into()),
        available_version,
        source: non_empty(row.source),
        ignored: false,
    })
}

fn parse_powershell_installed(output: &str) -> AppResult<Vec<InstalledPackage>> {
    let packages = parse_powershell_rows(output)?
        .into_iter()
        .map(|row| InstalledPackage {
            manager: ManagerId::Winget,
            id: row.id,
            name: row.name,
            version: row.installed_version.unwrap_or_else(|| "Unknown".into()),
            source: non_empty(row.source),
        })
        .collect();
    Ok(without_duplicates(packages))
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.is_empty())
}

/// `winget list` reports one row per installed architecture or runtime build, which would
/// otherwise show up as indistinguishable entries.
fn without_duplicates(mut packages: Vec<InstalledPackage>) -> Vec<InstalledPackage> {
    let mut seen = HashSet::new();
    packages.retain(|package| seen.insert((package.id.clone(), package.version.clone())));
    packages
}

struct WingetTable<'a> {
    columns: Vec<usize>,
    rows: Vec<&'a str>,
}

/// `None` means winget reported that nothing matched, which is not an error.
fn read_table(clean: &str) -> AppResult<Option<WingetTable<'_>>> {
    let lines: Vec<_> = clean
        .split(['\r', '\n'])
        .filter(|line| !line.trim().is_empty())
        .collect();
    let Some(separator) = lines.iter().position(|line| is_separator(line)) else {
        if is_empty_package_list(clean) {
            return Ok(None);
        }
        return Err(AppError::Parse("WinGet: table header is missing".into()));
    };
    let header = separator
        .checked_sub(1)
        .and_then(|index| lines.get(index))
        .ok_or_else(|| AppError::Parse("WinGet: table header is missing".into()))?;
    Ok(Some(WingetTable {
        columns: column_starts(header),
        rows: lines[separator + 1..].to_vec(),
    }))
}

fn parse_winget_table(output: &str) -> AppResult<Upgrades> {
    let clean = strip_terminal_sequences(output);
    let Some(table) = read_table(&clean)? else {
        return Ok(Upgrades::default());
    };
    if table.columns.len() != 5 {
        return Err(AppError::Parse(
            "WinGet: expected five table columns".into(),
        ));
    }
    let (packages, warnings) = parse_rows(&table, parse_table_row, |package| &package.id);
    Ok(Upgrades { packages, warnings })
}

/// Without any available upgrade winget omits the "available" column.
fn parse_winget_installed(output: &str) -> AppResult<Installed> {
    let clean = strip_terminal_sequences(output);
    let Some(table) = read_table(&clean)? else {
        return Ok(Installed::default());
    };
    if !(4..=5).contains(&table.columns.len()) {
        return Err(AppError::Parse(
            "WinGet: expected four or five table columns".into(),
        ));
    }
    let (packages, warnings) = parse_rows(&table, parse_installed_row, |package| &package.id);
    Ok(Installed {
        packages: without_duplicates(packages),
        warnings,
    })
}

/// Stops at the first line that is no row (e.g. the summary) and skips rows with truncated IDs.
fn parse_rows<T>(
    table: &WingetTable,
    parse_row: fn(&str, &[usize]) -> Option<T>,
    id: impl Fn(&T) -> &str,
) -> (Vec<T>, Vec<String>) {
    let mut rows = Vec::new();
    let mut truncated = 0;
    for line in &table.rows {
        let Some(row) = parse_row(line, &table.columns) else {
            break;
        };
        if has_truncated_id(id(&row)) {
            truncated += 1;
        } else {
            rows.push(row);
        }
    }
    (rows, truncated_id_warnings(truncated))
}

fn has_truncated_id(id: &str) -> bool {
    id.contains(['…', '�']) || id.ends_with("...")
}

fn truncated_id_warnings(truncated: usize) -> Vec<String> {
    if truncated == 0 {
        return Vec::new();
    }
    let noun = if truncated == 1 {
        "package"
    } else {
        "packages"
    };
    let ids = if truncated == 1 {
        "a truncated ID"
    } else {
        "truncated IDs"
    };
    vec![format!(
        "{truncated} {noun} with {ids} – install the WinGet PowerShell module"
    )]
}

fn strip_terminal_sequences(output: &str) -> String {
    let mut characters = output.chars().peekable();
    let mut clean = String::new();
    while let Some(character) = characters.next() {
        if character == '\u{1b}' && characters.peek() == Some(&'[') {
            characters.next();
            for control in characters.by_ref() {
                if ('@'..='~').contains(&control) {
                    break;
                }
            }
        } else if character != '\u{8}' {
            clean.push(character);
        }
    }
    clean
}

fn is_separator(line: &str) -> bool {
    let line = line.trim();
    line.len() >= 5 && line.chars().all(|character| character == '-')
}

fn column_starts(header: &str) -> Vec<usize> {
    let mut previous_space = true;
    header
        .chars()
        .enumerate()
        .filter_map(|(index, character)| {
            let start = previous_space && !character.is_whitespace();
            previous_space = character.is_whitespace();
            start.then_some(index)
        })
        .collect()
}

fn table_fields(line: &str, columns: &[usize]) -> Vec<String> {
    let characters: Vec<_> = line.chars().collect();
    let boundary = |index: usize| {
        columns
            .get(index)
            .copied()
            .unwrap_or(characters.len())
            .min(characters.len())
    };
    (0..columns.len())
        .map(|index| {
            characters[boundary(index)..boundary(index + 1)]
                .iter()
                .collect::<String>()
                .trim()
                .to_owned()
        })
        .collect()
}

fn parse_table_row(line: &str, columns: &[usize]) -> Option<Package> {
    let fields = table_fields(line, columns);
    if fields.iter().any(String::is_empty) || fields[4].split_whitespace().count() != 1 {
        return None;
    }
    Some(Package {
        manager: ManagerId::Winget,
        name: fields[0].clone(),
        id: fields[1].clone(),
        current_version: fields[2].clone(),
        available_version: fields[3].clone(),
        source: Some(fields[4].clone()),
        ignored: false,
    })
}

fn parse_installed_row(line: &str, columns: &[usize]) -> Option<InstalledPackage> {
    let fields = table_fields(line, columns);
    // Versions that winget only knows as a bound are printed like "> 1.8.10".
    let version = fields[2].trim_start_matches(['<', '>']).trim();
    if fields[..2].iter().any(String::is_empty) || version.is_empty() {
        return None;
    }
    Some(InstalledPackage {
        manager: ManagerId::Winget,
        name: fields[0].clone(),
        id: fields[1].clone(),
        version: version.to_owned(),
        source: non_empty(fields.last().cloned()),
    })
}

fn is_empty_package_list(output: &str) -> bool {
    let output = output.to_lowercase();
    [
        "keine verfügbaren upgrades",
        "no available upgrades",
        "no installed package found matching input criteria",
        "kein installiertes paket gefunden",
        "no applicable upgrade found",
        "kein anwendbares upgrade gefunden",
    ]
    .iter()
    .any(|message| output.contains(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_german_output_and_stops_at_summary() {
        let packages = parse_winget_table(include_str!("../../tests/fixtures/winget_upgrade.txt"))
            .unwrap()
            .packages;
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].id, "Microsoft.WSL");
        assert_eq!(packages[0].current_version, "2.7.3.0");
        assert_eq!(packages[0].available_version, "2.7.13");
    }

    #[test]
    fn parses_localized_headers_by_character_position() {
        let packages = parse_winget_table(include_str!("../../tests/fixtures/winget_english.txt"))
            .unwrap()
            .packages;
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "Müller Werkzeug");
        assert_eq!(packages[0].id, "Vendor.Mueller");
        assert_eq!(packages[1].current_version, "Unknown");
    }

    #[test]
    fn discards_spinner_and_ansi_prefix() {
        let table = include_str!("../../tests/fixtures/winget_english.txt");
        let output = format!("\u{1b}[?25l\r-\r\\\r|\r/\r ███▒▒ 50%\r\n{table}\u{1b}[?25h");
        assert_eq!(parse_winget_table(&output).unwrap().packages.len(), 2);
    }

    #[test]
    fn skips_truncated_ids_and_keeps_following_rows() {
        let upgrades =
            parse_winget_table(include_str!("../../tests/fixtures/winget_truncated.txt")).unwrap();
        assert_eq!(
            upgrades.warnings,
            vec!["1 package with a truncated ID – install the WinGet PowerShell module"]
        );
        let packages = upgrades.packages;
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].id, "Vendor.Complete");
    }

    #[test]
    fn empty_upgrade_list_is_not_an_error() {
        assert!(
            parse_winget_table("Keine verfügbaren Upgrades")
                .unwrap()
                .packages
                .is_empty()
        );
        assert!(
            parse_winget_table("No available upgrades found.")
                .unwrap()
                .packages
                .is_empty()
        );
    }

    #[test]
    fn unexpected_output_is_an_error() {
        assert!(parse_winget_table("The source could not be opened").is_err());
    }

    #[test]
    fn parses_powershell_singleton_and_array() {
        let row = r#"{"Id":"Vendor.App","Name":"Äpp","InstalledVersion":"1","AvailableVersion":"2","Source":"winget"}"#;
        assert_eq!(parse_powershell_packages(row).unwrap()[0].name, "Äpp");
        assert_eq!(
            parse_powershell_packages(&format!("[{row}]"))
                .unwrap()
                .len(),
            1
        );
        assert!(parse_powershell_packages("null").unwrap().is_empty());
        assert!(parse_powershell_packages("[]").unwrap().is_empty());
        assert!(parse_powershell_packages("{}").is_err());
    }

    #[test]
    fn projected_powershell_null_versions_are_skipped() {
        let output = r#"[{"Id":"Vendor.App","Name":"App","InstalledVersion":"1","AvailableVersion":"2","Source":"winget"},{"Id":"Vendor.Empty","Name":"Empty","InstalledVersion":"1","AvailableVersion":null,"Source":"winget"},{"Id":"Vendor.Blank","Name":"Blank","InstalledVersion":"1","AvailableVersion":"  ","Source":"winget"}]"#;
        let packages = parse_powershell_packages(output).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].available_version, "2");
        assert_eq!(packages[0].source.as_deref(), Some("winget"));
        assert!(POWERSHELL_UPGRADABLE_QUERY.contains("@($_.AvailableVersions)[0]"));
    }

    #[test]
    fn parses_installed_table_with_blank_available_and_source() {
        let installed =
            parse_winget_installed(include_str!("../../tests/fixtures/winget_list.txt")).unwrap();
        let packages = installed.packages;
        assert!(installed.warnings.is_empty());
        assert_eq!(packages.len(), 160);
        assert_eq!(packages[0].id, "rhysd.actionlint");
        assert_eq!(packages[0].version, "1.7.12");
        assert_eq!(packages[0].source.as_deref(), Some("winget"));
        let chipset = &packages[1];
        assert_eq!(chipset.name, "AMD Chipset Software");
        assert_eq!(chipset.id, r"ARP\Machine\X86\AMD_Chipset_IODrivers");
        assert_eq!(chipset.source, None);
    }

    #[test]
    fn installed_table_keeps_distinct_versions_but_drops_repeated_rows() {
        let packages = parse_winget_installed(include_str!("../../tests/fixtures/winget_list.txt"))
            .unwrap()
            .packages;
        let versions_of = |id: &str| {
            packages
                .iter()
                .filter(|package| package.id == id)
                .map(|package| package.version.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            versions_of("EclipseAdoptium.Temurin.17.JDK"),
            ["17.0.20.8", "17.0.20.101"]
        );
        assert_eq!(versions_of("Microsoft.WindowsAppRuntime.1.8"), ["1.8.10"]);
    }

    #[test]
    fn installed_table_without_available_column_stops_at_summary() {
        let packages = parse_winget_installed(include_str!(
            "../../tests/fixtures/winget_list_without_available.txt"
        ))
        .unwrap()
        .packages;
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].id, "Git.Git");
        assert_eq!(packages[0].source.as_deref(), Some("winget"));
        assert_eq!(packages[1].id, r"ARP\User\X64\Zettel");
        assert_eq!(packages[1].version, "0.20.0");
        assert_eq!(packages[1].source, None);
    }

    #[test]
    fn installed_table_skips_truncated_ids_with_warning() {
        let installed =
            parse_winget_installed(include_str!("../../tests/fixtures/winget_truncated.txt"))
                .unwrap();
        assert_eq!(
            installed.warnings,
            vec!["1 package with a truncated ID – install the WinGet PowerShell module"]
        );
        assert_eq!(installed.packages.len(), 1);
        assert_eq!(installed.packages[0].id, "Vendor.Complete");
    }

    #[test]
    fn empty_installed_list_is_not_an_error() {
        assert!(
            parse_winget_installed(
                "Kein installiertes Paket gefunden, das den Eingabekriterien entspricht."
            )
            .unwrap()
            .packages
            .is_empty()
        );
        assert!(parse_winget_installed("The source could not be opened").is_err());
    }

    #[test]
    fn installed_table_with_unexpected_columns_is_an_error() {
        assert!(parse_winget_installed("Name Id\n-------\nGit Git.Git").is_err());
    }

    #[test]
    fn parses_installed_powershell_rows_without_available_version() {
        let output = r#"[{"Id":"Vendor.App","Name":"App","InstalledVersion":"1","Source":"winget"},{"Id":"ARP\\Machine\\X64\\Tool","Name":"Tool","InstalledVersion":null,"Source":""},{"Id":"Vendor.App","Name":"App","InstalledVersion":"1","Source":"winget"}]"#;
        let packages = parse_powershell_installed(output).unwrap();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].source.as_deref(), Some("winget"));
        assert_eq!(packages[1].id, r"ARP\Machine\X64\Tool");
        assert_eq!(packages[1].version, "Unknown");
        assert_eq!(packages[1].source, None);
        assert!(parse_powershell_installed("[]").unwrap().is_empty());
        assert!(parse_powershell_installed("{}").is_err());
        assert!(POWERSHELL_INSTALLED_QUERY.contains("Get-WinGetPackage | Select-Object"));
    }

    #[test]
    fn uninstall_arguments_are_exact_silent_and_without_package_agreements() {
        let args = uninstall_args("Vendor.App");
        assert!(args.contains(&"--exact"));
        assert!(args.contains(&"--silent"));
        assert!(!args.contains(&"--accept-package-agreements"));
        assert!(args.windows(2).any(|pair| pair == ["--id", "Vendor.App"]));
    }

    #[test]
    fn installed_ids_allow_registry_and_msix_ids_but_not_options() {
        for id in [
            r"ARP\Machine\X64\{CEF1D4CD-0C30-4B42-A922-5FDB044C0ACF}",
            r"ARP\Machine\X86\Google Chrome",
            r"MSIX\Microsoft.GamingApp_2609.1001.20.0_x64__8wekyb3d8bbwe",
            "Git.Git",
        ] {
            assert!(validate_installed_id(id).is_ok(), "{id}");
        }
        for id in ["", "  ", "--force", "Vendor.App\n--force"] {
            assert!(validate_installed_id(id).is_err(), "{id:?}");
        }
    }

    #[test]
    fn upgrade_arguments_are_exact_and_include_unknown() {
        let args = upgrade_args("Vendor.App");
        assert!(args.contains(&"--include-unknown"));
        assert!(args.contains(&"--exact"));
        assert!(args.windows(2).any(|pair| pair == ["--id", "Vendor.App"]));
    }

    #[test]
    fn recognizes_signed_windows_no_upgrade_exit_code() {
        let mut output = CommandOutput {
            success: false,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(NO_APPLICABLE_UPGRADE as i32),
        };
        assert!(no_applicable_upgrade(&output));
        output.exit_code = Some(1);
        assert!(!no_applicable_upgrade(&output));
    }
}
