use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ManagerId {
    Winget,
    Scoop,
    Chocolatey,
    Npm,
    Pip,
}

impl ManagerId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Winget => "winget",
            Self::Scoop => "scoop",
            Self::Chocolatey => "chocolatey",
            Self::Npm => "npm",
            Self::Pip => "pip",
        }
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ManagerInfo {
    pub id: ManagerId,
    pub name: String,
    pub available: bool,
    pub version: Option<String>,
    pub enabled: bool,
    pub requires_admin: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PackageRef {
    pub manager: ManagerId,
    pub id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub manager: ManagerId,
    pub id: String,
    pub name: String,
    pub current_version: String,
    pub available_version: String,
    pub source: Option<String>,
    pub ignored: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ManagerError {
    pub manager: ManagerId,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub packages: Vec<Package>,
    pub errors: Vec<ManagerError>,
    /// RFC 3339
    pub scanned_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPackage {
    pub manager: ManagerId,
    pub id: String,
    pub name: String,
    pub version: String,
    pub source: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InstalledResult {
    pub packages: Vec<InstalledPackage>,
    pub errors: Vec<ManagerError>,
    /// RFC 3339
    pub listed_at: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum UpdateStatus {
    Queued,
    Skipped,
    Running,
    Succeeded,
    Failed,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEvent {
    pub package: PackageRef,
    pub status: UpdateStatus,
    pub message: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSummary {
    pub succeeded: u32,
    pub failed: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub package: PackageRef,
    pub name: String,
    pub from_version: String,
    pub to_version: String,
    pub status: UpdateStatus,
    pub message: Option<String>,
    /// RFC 3339
    pub finished_at: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct RunState {
    pub scanning: bool,
    pub updating: bool,
    /// RFC 3339
    pub last_scan: Option<String>,
    /// RFC 3339
    pub next_scan: Option<String>,
    pub pending_updates: u32,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub auto_update: bool,
    /// Without periodic checks the app only checks once after it starts (i.e. at login).
    pub periodic_checks: bool,
    pub check_interval_hours: u32,
    pub update_on_startup: bool,
    pub startup_delay_minutes: u32,
    pub notify: bool,
    pub autostart: bool,
    pub include_unknown_versions: bool,
    pub enabled_managers: Vec<ManagerId>,
    /// Keys "<manager>:<id>", e.g. "winget:Microsoft.WSL"
    pub ignored_packages: Vec<String>,
    pub theme: Theme,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_update: false,
            periodic_checks: false,
            check_interval_hours: 6,
            update_on_startup: true,
            startup_delay_minutes: 2,
            notify: true,
            autostart: true,
            include_unknown_versions: false,
            enabled_managers: vec![
                ManagerId::Winget,
                ManagerId::Scoop,
                ManagerId::Chocolatey,
                ManagerId::Npm,
            ],
            ignored_packages: Vec::new(),
            theme: Theme::System,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manager_names_match_serialization() {
        for id in [
            ManagerId::Winget,
            ManagerId::Scoop,
            ManagerId::Chocolatey,
            ManagerId::Npm,
            ManagerId::Pip,
        ] {
            assert_eq!(serde_json::to_value(id).unwrap(), id.as_str());
        }
        assert_eq!(
            serde_json::to_value(UpdateStatus::Skipped).unwrap(),
            "skipped"
        );
    }
}
