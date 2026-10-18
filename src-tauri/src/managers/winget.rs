use serde::Deserialize;

use super::{PackageManager, Upgrades};
use crate::error::{AppError, AppResult};
use crate::models::{ManagerId, Package};
use crate::process::{self, CommandOutput, LIST_TIMEOUT, UPDATE_TIMEOUT};

const NO_APPLICABLE_UPGRADE: u32 = 0x8A15002B;
const POWERSHELL_PACKAGES: &str = r#"$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); if (-not (Get-Module -ListAvailable Microsoft.WinGet.Client)) { exit 42 }; Import-Module Microsoft.WinGet.Client; $packages=@(Get-WinGetPackage | Where-Object IsUpdateAvailable | Select-Object Id,Name,InstalledVersion,@{Name='AvailableVersion';Expression={@($_.AvailableVersions)[0]}},Source); ConvertTo-Json -InputObject $packages -Compress"#;

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
        if let Ok(output) = process::run(
            "powershell.exe",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                POWERSHELL_PACKAGES,
            ],
            LIST_TIMEOUT,
        ) && output.success
        {
            match parse_powershell_packages(&output.stdout) {
                Ok(mut packages) => {
                    if !include_unknown_versions {
                        packages.retain(|package| !unknown_version(&package.current_version));
                    }
                    return Ok(packages.into());
                }
                Err(error) => warnings.push(format!(
                    "WinGet-PowerShell-Ausgabe nicht lesbar – Tabellen-Fallback genutzt: {error}"
                )),
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
                "winget upgrade: Kein anwendbares Upgrade gefunden (0x8A15002B)".into(),
            ));
        }
        output.ensure_success("winget upgrade")?;
        Ok(())
    }
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

fn parse_powershell_packages(output: &str) -> AppResult<Vec<Package>> {
    let value: serde_json::Value =
        serde_json::from_str(output.trim().trim_start_matches('\u{feff}'))
            .map_err(|error| AppError::Parse(format!("WinGet-PowerShell: {error}")))?;
    let rows = match value {
        serde_json::Value::Null => Vec::new(),
        serde_json::Value::Array(rows) => rows,
        value => vec![value],
    };
    let mut packages = Vec::new();
    for row in rows {
        let row: PowershellPackage = serde_json::from_value(row)
            .map_err(|error| AppError::Parse(format!("WinGet-PowerShell-Paket: {error}")))?;
        let Some(available_version) = row
            .available_version
            .filter(|value| !value.trim().is_empty())
        else {
            continue;
        };
        packages.push(Package {
            manager: ManagerId::Winget,
            id: row.id,
            name: row.name,
            current_version: row.installed_version.unwrap_or_else(|| "Unknown".into()),
            available_version,
            source: row.source.filter(|source| !source.is_empty()),
            ignored: false,
        });
    }
    Ok(packages)
}

fn parse_winget_table(output: &str) -> AppResult<Upgrades> {
    let clean = strip_terminal_sequences(output);
    let lines: Vec<_> = clean
        .split(['\r', '\n'])
        .filter(|line| !line.trim().is_empty())
        .collect();
    let Some(separator) = lines.iter().position(|line| is_separator(line)) else {
        if is_empty_upgrade_list(&clean) {
            return Ok(Upgrades::default());
        }
        return Err(AppError::Parse("WinGet: Tabellenkopf fehlt".into()));
    };
    let header = separator
        .checked_sub(1)
        .and_then(|index| lines.get(index))
        .ok_or_else(|| AppError::Parse("WinGet: Tabellenkopf fehlt".into()))?;
    let columns = column_starts(header);
    if columns.len() != 5 {
        return Err(AppError::Parse(
            "WinGet: fünf Tabellenspalten erwartet".into(),
        ));
    }
    let mut packages = Vec::new();
    let mut truncated = 0;
    for line in &lines[separator + 1..] {
        let Some(package) = parse_table_row(line, &columns) else {
            break;
        };
        if !package.id.contains(['…', '�']) && !package.id.ends_with("...") {
            packages.push(package);
        } else {
            truncated += 1;
        }
    }
    let warnings = if truncated == 0 {
        Vec::new()
    } else {
        let noun = if truncated == 1 { "Paket" } else { "Pakete" };
        vec![format!(
            "{truncated} {noun} mit abgeschnittener ID – WinGet-PowerShell-Modul installieren"
        )]
    };
    Ok(Upgrades { packages, warnings })
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

fn parse_table_row(line: &str, columns: &[usize]) -> Option<Package> {
    let characters: Vec<_> = line.chars().collect();
    if characters.len() <= columns[4] {
        return None;
    }
    let field = |index: usize| -> String {
        let end = columns.get(index + 1).copied().unwrap_or(characters.len());
        characters[columns[index]..end]
            .iter()
            .collect::<String>()
            .trim()
            .to_owned()
    };
    let fields: Vec<_> = (0..5).map(field).collect();
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

fn is_empty_upgrade_list(output: &str) -> bool {
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
            vec!["1 Paket mit abgeschnittener ID – WinGet-PowerShell-Modul installieren"]
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
        assert!(POWERSHELL_PACKAGES.contains("@($_.AvailableVersions)[0]"));
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
