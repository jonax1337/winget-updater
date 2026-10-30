use std::collections::BTreeMap;

use serde::Deserialize;

use super::{Installed, PackageManager, Upgrades};
use crate::error::{AppError, AppResult};
use crate::models::{InstalledPackage, ManagerId, Package};
use crate::process::{self, LIST_TIMEOUT, UNINSTALL_TIMEOUT, UPDATE_TIMEOUT};

pub struct Npm;

impl PackageManager for Npm {
    fn id(&self) -> ManagerId {
        ManagerId::Npm
    }
    fn display_name(&self) -> &'static str {
        "npm (global)"
    }
    fn requires_admin(&self) -> bool {
        false
    }

    fn detect(&self) -> AppResult<Option<String>> {
        process::detect_version("npm", &["--version"])
    }

    fn list_upgrades(&self, _include_unknown_versions: bool) -> AppResult<Upgrades> {
        let output = process::run("npm", &["outdated", "-g", "--json"], LIST_TIMEOUT)?;
        // npm exits with 1 for a nonempty outdated list, not just command failures.
        if !output.success && output.exit_code != Some(1) {
            return output
                .ensure_success("npm outdated -g --json")
                .map(|_| Upgrades::default());
        }
        parse_npm_outdated(&output.stdout).map(Upgrades::from)
    }

    fn upgrade(&self, package_id: &str) -> AppResult<()> {
        process::validate_package_id(package_id)?;
        let package = format!("{package_id}@latest");
        process::run("npm", &["install", "-g", &package], UPDATE_TIMEOUT)?
            .ensure_success("npm install -g")?;
        Ok(())
    }

    fn list_installed(&self) -> AppResult<Installed> {
        let output = process::run("npm", &["ls", "-g", "--json", "--depth=0"], LIST_TIMEOUT)?;
        // npm exits with 1 for problems such as unmet peer dependencies but still prints the tree.
        if !output.success && output.exit_code != Some(1) {
            return output
                .ensure_success("npm ls -g --json")
                .map(|_| Installed::default());
        }
        parse_npm_installed(&output.stdout).map(Installed::from)
    }

    fn uninstall(&self, package_id: &str) -> AppResult<()> {
        process::validate_package_id(package_id)?;
        process::run("npm", &["uninstall", "-g", package_id], UNINSTALL_TIMEOUT)?
            .ensure_success("npm uninstall -g")?;
        Ok(())
    }
}

#[derive(Deserialize)]
struct OutdatedPackage {
    current: Option<String>,
    latest: String,
}

fn parse_npm_outdated(output: &str) -> AppResult<Vec<Package>> {
    let packages: BTreeMap<String, OutdatedPackage> =
        serde_json::from_str(output.trim().trim_start_matches('\u{feff}'))
            .map_err(|error| AppError::Parse(format!("npm outdated: {error}")))?;
    Ok(packages
        .into_iter()
        .map(|(name, version)| Package {
            manager: ManagerId::Npm,
            id: name.clone(),
            name,
            current_version: version.current.unwrap_or_else(|| "Unknown".into()),
            available_version: version.latest,
            source: Some("npm".into()),
            ignored: false,
        })
        .collect())
}

#[derive(Deserialize)]
struct Listing {
    error: Option<serde_json::Value>,
    #[serde(default)]
    dependencies: BTreeMap<String, ListedPackage>,
}

#[derive(Deserialize)]
struct ListedPackage {
    version: Option<String>,
}

fn parse_npm_installed(output: &str) -> AppResult<Vec<InstalledPackage>> {
    let listing: Listing = serde_json::from_str(output.trim().trim_start_matches('\u{feff}'))
        .map_err(|error| AppError::Parse(format!("npm ls: {error}")))?;
    if let Some(error) = listing.error {
        return Err(AppError::Parse(format!("npm ls: {error}")));
    }
    Ok(listing
        .dependencies
        .into_iter()
        .map(|(name, package)| InstalledPackage {
            manager: ManagerId::Npm,
            id: name.clone(),
            name,
            version: package.version.unwrap_or_else(|| "Unknown".into()),
            source: Some("npm".into()),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_installed_global_packages() {
        let packages =
            parse_npm_installed(include_str!("../../tests/fixtures/npm_list.json")).unwrap();
        assert_eq!(packages.len(), 4);
        assert_eq!(packages[0].id, "@example/cli");
        assert_eq!(packages[0].version, "1.0.0");
        assert_eq!(packages[0].source.as_deref(), Some("npm"));
        assert_eq!(packages[2].id, "local-tool");
    }

    #[test]
    fn installed_without_dependencies_or_version() {
        assert!(parse_npm_installed(r#"{"name":"lib"}"#).unwrap().is_empty());
        let packages =
            parse_npm_installed(r#"{"dependencies":{"broken":{"missing":true}}}"#).unwrap();
        assert_eq!(packages[0].version, "Unknown");
    }

    #[test]
    fn installed_error_objects_and_invalid_json_are_rejected() {
        for output in [
            r#"{"error":{"code":"EJSONPARSE","summary":"Unexpected token"}}"#,
            "not json",
            "null",
        ] {
            assert!(parse_npm_installed(output).is_err(), "{output}");
        }
    }

    #[test]
    fn parses_scoped_names_and_uses_latest_not_wanted() {
        let packages =
            parse_npm_outdated(include_str!("../../tests/fixtures/npm_outdated.json")).unwrap();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].id, "@example/cli");
        assert_eq!(packages[0].current_version, "1.0.0");
        assert_eq!(packages[0].available_version, "2.0.0");
    }

    #[test]
    fn empty_object_has_no_updates() {
        assert!(parse_npm_outdated("{}").unwrap().is_empty());
    }

    #[test]
    fn missing_current_version_is_unknown() {
        let packages = parse_npm_outdated(r#"{"cli":{"latest":"2"}}"#).unwrap();
        assert_eq!(packages[0].current_version, "Unknown");
    }

    #[test]
    fn npm_error_objects_and_invalid_json_are_rejected() {
        for output in [
            r#"{"error":{"code":"EACCES","summary":"Access denied"}}"#,
            "not json",
            "null",
        ] {
            assert!(parse_npm_outdated(output).is_err());
        }
    }
}
