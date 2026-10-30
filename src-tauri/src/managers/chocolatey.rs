use super::{Installed, PackageManager, Upgrades};
use crate::error::{AppError, AppResult};
use crate::models::{InstalledPackage, ManagerId, Package};
use crate::process::{self, CommandOutput, LIST_TIMEOUT, UNINSTALL_TIMEOUT, UPDATE_TIMEOUT};

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
        ensure_success_or_restart(output, "choco upgrade")
    }

    fn list_installed(&self) -> AppResult<Installed> {
        let output = process::run("choco", &["list", "-r", "--local-only"], LIST_TIMEOUT)?
            .ensure_success("choco list -r --local-only")?;
        parse_chocolatey_list(&output.stdout).map(Installed::from)
    }

    fn uninstall(&self, package_id: &str) -> AppResult<()> {
        process::validate_package_id(package_id)?;
        let output = process::run(
            "choco",
            &["uninstall", package_id, "-y", "--no-progress"],
            UNINSTALL_TIMEOUT,
        )?;
        ensure_success_or_restart(output, "choco uninstall")
    }
}

/// Successful MSI installations may request or initiate a restart.
fn ensure_success_or_restart(output: CommandOutput, command: &str) -> AppResult<()> {
    if matches!(output.exit_code, Some(1641 | 3010)) {
        return Ok(());
    }
    output.ensure_success(command)?;
    Ok(())
}

/// Lines without a separator are warnings Chocolatey prints next to the `-r` output.
fn parse_chocolatey_list(output: &str) -> AppResult<Vec<InstalledPackage>> {
    let mut packages = Vec::new();
    for line in output.lines().filter(|line| line.contains('|')) {
        let fields: Vec<_> = line.split('|').map(str::trim).collect();
        if fields.len() != 2 || fields.iter().any(|field| field.is_empty()) {
            return Err(AppError::Parse(format!(
                "Chocolatey: ungültige Paketzeile: {line}"
            )));
        }
        packages.push(InstalledPackage {
            manager: ManagerId::Chocolatey,
            id: fields[0].into(),
            name: fields[0].into(),
            version: fields[1].into(),
            source: None,
        });
    }
    Ok(packages)
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
    fn parses_installed_packages() {
        let packages =
            parse_chocolatey_list(include_str!("../../tests/fixtures/choco_list.txt")).unwrap();
        assert_eq!(packages.len(), 3);
        assert_eq!(packages[1].id, "chocolatey");
        assert_eq!(packages[1].version, "2.4.1");
        assert_eq!(packages[2].name, "git");
    }

    #[test]
    fn installed_list_ignores_warnings_and_rejects_broken_rows() {
        let packages =
            parse_chocolatey_list("Chocolatey v2.4.1\nWARNING: deprecated option\ngit|2.47.0\n")
                .unwrap();
        assert_eq!(packages.len(), 1);
        assert!(parse_chocolatey_list("").unwrap().is_empty());
        for row in ["git|", "|1.0", "git|1.0|extra"] {
            assert!(parse_chocolatey_list(row).is_err(), "{row}");
        }
    }

    #[test]
    fn restart_request_counts_as_success() {
        let output = |exit_code| CommandOutput {
            success: false,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(exit_code),
        };
        assert!(ensure_success_or_restart(output(3010), "choco").is_ok());
        assert!(ensure_success_or_restart(output(1641), "choco").is_ok());
        assert!(ensure_success_or_restart(output(1), "choco").is_err());
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
