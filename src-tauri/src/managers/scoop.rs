use super::PackageManager;
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

    fn detect(&self) -> Option<String> {
        process::detect_version("scoop", &["--version"])
    }

    fn list_upgrades(&self, _include_unknown_versions: bool) -> AppResult<Vec<Package>> {
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

fn parse_scoop_status(output: &str) -> AppResult<Vec<Package>> {
    if output.contains("Everything is ok!") {
        return Ok(Vec::new());
    }
    let lines: Vec<_> = output.lines().collect();
    let Some(separator) = lines.iter().position(|line| {
        let line = line.trim();
        line.contains("---")
            && line
                .chars()
                .all(|character| character == '-' || character.is_whitespace())
    }) else {
        return Err(AppError::Parse("Scoop: Statustabelle fehlt".into()));
    };
    lines[separator + 1..]
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| parse_scoop_row(line))
        .collect()
}

fn parse_scoop_row(line: &str) -> AppResult<Package> {
    let fields: Vec<_> = line.split_whitespace().collect();
    if fields.len() < 3 {
        return Err(AppError::Parse(format!(
            "Scoop: ungültige Statuszeile: {line}"
        )));
    }
    Ok(Package {
        manager: ManagerId::Scoop,
        id: fields[0].into(),
        name: fields[0].into(),
        current_version: fields[1].into(),
        available_version: fields[2].into(),
        source: None,
        ignored: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status_table_after_bucket_messages() {
        let packages =
            parse_scoop_status(include_str!("../../tests/fixtures/scoop_status.txt")).unwrap();
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
                .is_empty()
        );
    }

    #[test]
    fn malformed_output_is_rejected() {
        assert!(parse_scoop_status("ERROR: bucket unavailable").is_err());
        assert!(
            parse_scoop_status("Name Installed Latest\n---- --------- ------\nbroken").is_err()
        );
    }
}
