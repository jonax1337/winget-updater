use super::{PackageManager, Upgrades};
use crate::error::{AppError, AppResult};
use crate::models::{ManagerId, Package};
use crate::process::{self, LIST_TIMEOUT, UPDATE_TIMEOUT};

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
}

fn parse_scoop_status(output: &str) -> AppResult<Upgrades> {
    if output.contains("Everything is ok!") {
        return Ok(Upgrades::default());
    }
    let lines: Vec<_> = output.lines().collect();
    let separator = lines
        .iter()
        .position(|line| {
            line.contains("---")
                && line
                    .chars()
                    .all(|character| character == '-' || character.is_whitespace())
        })
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

fn parse_scoop_row(line: &str, columns: &[usize], upgrades: &mut Upgrades) {
    let characters: Vec<_> = line.chars().collect();
    let field = |index: usize| -> String {
        let start = columns
            .get(index)
            .copied()
            .unwrap_or(characters.len())
            .min(characters.len());
        let end = columns
            .get(index + 1)
            .copied()
            .unwrap_or(characters.len())
            .min(characters.len());
        characters[start..end]
            .iter()
            .collect::<String>()
            .trim()
            .to_owned()
    };
    let name = field(0);
    if name.is_empty() {
        return;
    }
    let info = field(4);
    if info.contains("Held package") {
        upgrades.warnings.push(format!(
            "{name}: Paket ist gehalten (scoop hold) – nicht aktualisiert"
        ));
        return;
    }
    if info.contains("Global install") {
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
