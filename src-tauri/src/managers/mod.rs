mod chocolatey;
mod npm;
mod pip;
mod scoop;
mod winget;

use crate::error::AppResult;
use crate::models::{ManagerId, Package};

#[derive(Debug, Default)]
pub struct Upgrades {
    pub packages: Vec<Package>,
    pub warnings: Vec<String>,
}

impl From<Vec<Package>> for Upgrades {
    fn from(packages: Vec<Package>) -> Self {
        Self {
            packages,
            warnings: Vec::new(),
        }
    }
}

/// One package manager. Methods block (they spawn processes); call them via `spawn_blocking`.
pub trait PackageManager: Send + Sync {
    fn id(&self) -> ManagerId;
    fn display_name(&self) -> &'static str;
    fn requires_admin(&self) -> bool;
    /// Installed version, or `None` if the manager is not installed.
    fn detect(&self) -> AppResult<Option<String>>;
    /// Outdated packages. `ignored` is always `false` here; the engine applies settings.
    fn list_upgrades(&self, include_unknown_versions: bool) -> AppResult<Upgrades>;
    fn upgrade(&self, package_id: &str) -> AppResult<()>;
}

pub fn registry() -> Vec<Box<dyn PackageManager>> {
    vec![
        Box::new(winget::Winget),
        Box::new(scoop::Scoop),
        Box::new(chocolatey::Chocolatey),
        Box::new(npm::Npm),
        Box::new(pip::Pip),
    ]
}
