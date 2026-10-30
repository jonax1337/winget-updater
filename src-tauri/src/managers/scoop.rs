use super::{Installed, PackageManager, Upgrades};
use crate::error::{AppError, AppResult};
use crate::models::{InstalledPackage, ManagerId, Package};
use crate::process::{self, LIST_TIMEOUT, UNINSTALL_TIMEOUT, UPDATE_TIMEOUT};

const GLOBAL_INSTALL_MARKER: &str = "Global install";
const INFO_COLUMN: usize = 4;

pub struct Scoop;

impl PackageManager for Scoop {
    fn id(&self) -> ManagerId {
        ManagerId::Scoop
    }
    fn display_name(&self) -> &'static str {
        "Scoop"
    }
    fn requires_admin(&self) -> bool {
        false
    }

    fn detect(&self) -> AppResult<Option<String>> {
        process::detect_version("scoop", &["--version"])
    }

    fn list_upgrades(&self, _include_unknown_versions: bool) -> AppResult<Upgrades> {
        process::run("scoop", &["update"], LIST_TIMEOUT)?.ensure_success("scoop update")?;
        let output =
            process::run("scoop", &["status"], LIST_TIMEOUT)?.ensure_success("scoop status")?;
        parse_scoop_status(&output.stdout)
    }

    fn upgrade(&self, package_id: &str) -> AppResult<()> {
        process::validate_package_id(package_id)?;
        process::run("scoop", &["update", package_id], UPDATE_TIMEOUT)?
            .ensure_success("scoop update")?;
        Ok(())
    }

    fn list_installed(&self) -> AppResult<Installed> {
        let output =
            process::run("scoop", &["list"], LIST_TIMEOUT)?.ensure_success("scoop list")?;
        parse_scoop_list(&output.stdout)
    }

    fn uninstall(&self, package_id: &str) -> AppResult<()> {
        process::validate_package_id(package_id)?;
        process::run("scoop", &["uninstall", package_id], UNINSTALL_TIMEOUT)?
            .ensure_success("scoop uninstall")?;
        Ok(())
    }
}

fn separator_position(lines: &[&str]) -> Option<usize> {
    lines.iter().position(|line| {
        line.contains("---")
            && line
                .chars()
                .all(|character| character == '-' || character.is_whitespace())
    })
}

fn parse_scoop_list(output: &str) -> AppResult<Installed> {
    if output.contains("There aren't any apps installed") {
        return Ok(Installed::default());
    }
    let lines: Vec<_> = output.lines().collect();
    let separator = separator_position(&lines)
        .ok_or_else(|| AppError::Parse("Scoop: Tabelle der installierten Apps fehlt".into()))?;
    let columns = column_starts(lines[separator]);
    if columns.len() < 3 {
        return Err(AppError::Parse(
            "Scoop: unvollständige Spaltengrenzen".into(),
        ));
    }
    let (global_rows, local_rows): (Vec<_>, Vec<_>) = lines[separator + 1..]
        .iter()
        .copied()
        .partition(|line| is_global_install(line, &columns));
    // Uninstalling needs `--global`, which this app does not pass.
    let warnings = if global_rows.is_empty() {
        Vec::new()
    } else {
        vec!["Global installierte Scoop-Apps werden nicht angezeigt.".to_owned()]
    };
    Ok(Installed {
        packages: local_rows
            .iter()
            .filter_map(|line| parse_installed_row(line, &columns))
            .collect(),
        warnings,
    })
}

fn is_global_install(line: &str, columns: &[usize]) -> bool {
    let characters: Vec<_> = line.chars().collect();
    table_field(&characters, columns, INFO_COLUMN).contains(GLOBAL_INSTALL_MARKER)
}

fn parse_installed_row(line: &str, columns: &[usize]) -> Option<InstalledPackage> {
    let characters: Vec<_> = line.chars().collect();
    let name = table_field(&characters, columns, 0);
    if name.is_empty() {
        return None;
    }
    Some(InstalledPackage {
        manager: ManagerId::Scoop,
        id: name.clone(),
        name,
        version: table_field(&characters, columns, 1),
        source: Some(table_field(&characters, columns, 2)).filter(|source| !source.is_empty()),
    })
}

fn parse_scoop_status(output: &str) -> AppResult<Upgrades> {
    if output.contains("Everything is ok!") {
        return Ok(Upgrades::default());
    }
    let lines: Vec<_> = output.lines().collect();
    let separator = separator_position(&lines)
        .ok_or_else(|| AppError::Parse("Scoop: Statustabelle fehlt".into()))?;
    let columns = column_starts(lines[separator]);
    if columns.len() < 3 {
        return Err(AppError::Parse(
            "Scoop: unvollständige Spaltengrenzen".into(),
        ));
    }
    let mut upgrades = Upgrades::default();
    for line in &lines[separator + 1..] {
        parse_scoop_row(line, &columns, &mut upgrades);
    }
    Ok(upgrades)
}

fn column_starts(separator: &str) -> Vec<usize> {
    let mut previous_dash = false;
    separator
        .chars()
        .enumerate()
        .filter_map(|(index, character)| {
            let dash = character == '-';
            let start = dash && !previous_dash;
            previous_dash = dash;
            start.then_some(index)
        })
        .collect()
}

fn table_field(characters: &[char], columns: &[usize], index: usize) -> String {
    let boundary = |index: usize| {
        columns
            .get(index)
            .copied()
            .unwrap_or(characters.len())
            .min(characters.len())
    };
    characters[boundary(index)..boundary(index + 1)]
        .iter()
        .collect::<String>()
        .trim()
        .to_owned()
}

fn parse_scoop_row(line: &str, columns: &[usize], upgrades: &mut Upgrades) {
    let characters: Vec<_> = line.chars().collect();
    let field = |index: usize| table_field(&characters, columns, index);
    let name = field(0);
    if name.is_empty() {
        return;
    }
    let info = field(INFO_COLUMN);
    if info.contains("Held package") {
        upgrades.warnings.push(format!(
            "{name}: Paket ist gehalten (scoop hold) – nicht aktualisiert"
        ));
        return;
    }
    if info.contains(GLOBAL_INSTALL_MARKER) {
        upgrades.warnings.push(format!(
            "{name}: globale Scoop-Installation wird nicht automatisch aktualisiert"
        ));
        return;
    }
    let latest = field(2);
    if latest.is_empty() {
        return;
    }
    upgrades.packages.push(Package {
        manager: ManagerId::Scoop,
        id: name.clone(),
        name,
        current_version: field(1),
        available_version: latest,
        source: None,
        ignored: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status_table_after_bucket_messages() {
        let packages = parse_scoop_status(include_str!("../../tests/fixtures/scoop_status.txt"))
            .unwrap()
            .packages;
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].id, "7zip");
        assert_eq!(packages[0].current_version, "24.08");
        assert_eq!(packages[1].available_version, "2.48.1");
        assert!(!packages[0].ignored);
    }

    #[test]
    fn parses_installed_apps_and_omits_global_installs() {
        let installed =
            parse_scoop_list(include_str!("../../tests/fixtures/scoop_list.txt")).unwrap();
        let packages = installed.packages;
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].id, "7zip");
        assert_eq!(packages[0].name, "7zip");
        assert_eq!(packages[1].version, "2.47.0.1");
        assert_eq!(packages[1].source.as_deref(), Some("main"));
        assert_eq!(
            installed.warnings,
            vec!["Global installierte Scoop-Apps werden nicht angezeigt."]
        );
    }

    #[test]
    fn local_only_list_has_no_warning() {
        let installed = parse_scoop_list(
            "Name Version Source Updated             Info
---- ------- ------ -------             ----
git  2.47.0  main   2024-12-01 09:12:44 ",
        )
        .unwrap();
        assert_eq!(installed.packages.len(), 1);
        assert!(installed.warnings.is_empty());
    }

    #[test]
    fn no_installed_apps_is_successful() {
        let installed = parse_scoop_list("There aren't any apps installed.").unwrap();
        assert!(installed.packages.is_empty());
        assert!(installed.warnings.is_empty());
    }

    #[test]
    fn malformed_installed_list_is_rejected() {
        assert!(parse_scoop_list("ERROR: scoop is broken").is_err());
        assert!(parse_scoop_list("Name\n----\ngit").is_err());
    }

    #[test]
    fn empty_status_is_successful() {
        assert!(
            parse_scoop_status("Scoop is up to date.\nEverything is ok!")
                .unwrap()
                .packages
                .is_empty()
        );
    }

    #[test]
    fn malformed_output_is_rejected() {
        assert!(parse_scoop_status("ERROR: bucket unavailable").is_err());
        assert!(
            parse_scoop_status("Name Installed Latest\n---- --------- ------\nbroken")
                .unwrap()
                .packages
                .is_empty()
        );
    }
    #[test]
    fn skips_missing_latest_and_reports_held_and_global() {
        let upgrades =
            parse_scoop_status(include_str!("../../tests/fixtures/scoop_edge_cases.txt")).unwrap();
        assert_eq!(upgrades.packages.len(), 1);
        assert_eq!(upgrades.packages[0].id, "git");
        assert_eq!(
            upgrades.warnings,
            vec![
                "held: Paket ist gehalten (scoop hold) – nicht aktualisiert",
                "glob: globale Scoop-Installation wird nicht automatisch aktualisiert",
            ]
        );
    }
}
