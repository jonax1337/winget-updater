use super::{PackageManager, Upgrades};
use crate::error::{AppError, AppResult};
use crate::models::{ManagerId, Package};
use crate::process::{self, LIST_TIMEOUT, UPDATE_TIMEOUT};

pub struct Chocolatey;

impl PackageManager for Chocolatey {
    fn id(&self) -> ManagerId {
        ManagerId::Chocolatey
    }
    fn display_name(&self) -> &'static str {
        "Chocolatey"
    }
    fn requires_admin(&self) -> bool {
        true
    }

    fn detect(&self) -> AppResult<Option<String>> {
        process::detect_version("choco", &["--version"])
    }

    fn list_upgrades(&self, _include_unknown_versions: bool) -> AppResult<Upgrades> {
        let output = process::run("choco", &["outdated", "-r"], LIST_TIMEOUT)?;
        // Chocolatey's enhanced exit codes use 2 when outdated packages were found.
        if !output.success && output.exit_code != Some(2) {
            return output
                .ensure_success("choco outdated -r")
                .map(|_| Upgrades::default());
        }
        parse_chocolatey_outdated(&output.stdout).map(Upgrades::from)
    }

    fn upgrade(&self, package_id: &str) -> AppResult<()> {
        process::validate_package_id(package_id)?;
        let output = process::run(
            "choco",
            &["upgrade", package_id, "-y", "--no-progress"],
            UPDATE_TIMEOUT,
        )?;
        // Successful MSI installations may request or initiate a restart.
        if matches!(output.exit_code, Some(1641 | 3010)) {
            return Ok(());
        }
        output.ensure_success("choco upgrade")?;
        Ok(())
    }
}

fn parse_chocolatey_outdated(output: &str) -> AppResult<Vec<Package>> {
    let mut packages = Vec::new();
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<_> = line.split('|').map(str::trim).collect();
        if fields.len() != 4 || fields[..3].iter().any(|field| field.is_empty()) {
            return Err(AppError::Parse(format!(
                "Chocolatey: ungültige Paketzeile: {line}"
            )));
        }
        if fields[3].eq_ignore_ascii_case("true") {
            continue;
        }
        if !fields[3].eq_ignore_ascii_case("false") {
            return Err(AppError::Parse(format!(
                "Chocolatey: ungültiger Pin-Status: {line}"
            )));
        }
        packages.push(Package {
            manager: ManagerId::Chocolatey,
            id: fields[0].into(),
            name: fields[0].into(),
            current_version: fields[1].into(),
            available_version: fields[2].into(),
            source: None,
            ignored: false,
        });
    }
    Ok(packages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_limited_output_and_skips_pinned_packages() {
        let packages =
            parse_chocolatey_outdated(include_str!("../../tests/fixtures/choco_outdated.txt"))
                .unwrap();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].id, "7zip");
        assert_eq!(packages[0].current_version, "24.8.0");
        assert_eq!(packages[0].available_version, "24.9.0");
        assert_eq!(packages[1].id, "git");
    }

    #[test]
    fn requires_elevation() {
        assert!(Chocolatey.requires_admin());
    }

    #[test]
    fn empty_output_has_no_updates() {
        assert!(parse_chocolatey_outdated("").unwrap().is_empty());
    }

    #[test]
    fn malformed_rows_are_rejected() {
        for row in [
            "app|1|2",
            "app|1|2|maybe",
            "app||2|false",
            "ERROR: source unavailable",
        ] {
            assert!(parse_chocolatey_outdated(row).is_err());
        }
    }
}
