use serde::Deserialize;

use super::PackageManager;
use crate::error::{AppError, AppResult};
use crate::models::{ManagerId, Package};
use crate::process::{self, LIST_TIMEOUT, UPDATE_TIMEOUT};

pub struct Pip;

impl PackageManager for Pip {
    fn id(&self) -> ManagerId {
        ManagerId::Pip
    }
    fn display_name(&self) -> &'static str {
        "pip (Benutzer)"
    }
    fn requires_admin(&self) -> bool {
        false
    }

    fn detect(&self) -> Option<String> {
        process::detect_version("python", &["-m", "pip", "--version"])
    }

    fn list_upgrades(&self, _include_unknown_versions: bool) -> AppResult<Vec<Package>> {
        let output = process::run(
            "python",
            &["-m", "pip", "list", "--outdated", "--format=json", "--user"],
            LIST_TIMEOUT,
        )?
        .ensure_success("python -m pip list --outdated")?;
        parse_pip_outdated(&output.stdout)
    }

    fn upgrade(&self, package_id: &str) -> AppResult<()> {
        process::validate_package_id(package_id)?;
        process::run(
            "python",
            &["-m", "pip", "install", "-U", "--user", package_id],
            UPDATE_TIMEOUT,
        )?
        .ensure_success("python -m pip install -U --user")?;
        Ok(())
    }
}

#[derive(Deserialize)]
struct OutdatedPackage {
    name: String,
    version: String,
    latest_version: String,
}

fn parse_pip_outdated(output: &str) -> AppResult<Vec<Package>> {
    let packages: Vec<OutdatedPackage> =
        serde_json::from_str(output.trim().trim_start_matches('\u{feff}'))
            .map_err(|error| AppError::Parse(format!("pip list --outdated: {error}")))?;
    Ok(packages
        .into_iter()
        .map(|package| Package {
            manager: ManagerId::Pip,
            id: package.name.clone(),
            name: package.name,
            current_version: package.version,
            available_version: package.latest_version,
            source: Some("PyPI".into()),
            ignored: false,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pip_json_versions() {
        let packages =
            parse_pip_outdated(include_str!("../../tests/fixtures/pip_outdated.json")).unwrap();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].id, "requests");
        assert_eq!(packages[0].current_version, "2.31.0");
        assert_eq!(packages[0].available_version, "2.32.3");
    }

    #[test]
    fn empty_array_has_no_updates() {
        assert!(parse_pip_outdated("[]").unwrap().is_empty());
    }

    #[test]
    fn malformed_json_and_incomplete_packages_are_rejected() {
        for output in ["not json", "{}", r#"[{"name":"broken","version":"1"}]"#] {
            assert!(parse_pip_outdated(output).is_err());
        }
    }
}
