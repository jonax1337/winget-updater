use crate::error::AppResult;
use crate::models::{ManagerId, Package};

/// One package manager. Methods block (they spawn processes); call them via `spawn_blocking`.
pub trait PackageManager: Send + Sync {
    fn id(&self) -> ManagerId;
    fn display_name(&self) -> &'static str;
    fn requires_admin(&self) -> bool;
    /// Installed version, or `None` if the manager is not installed.
    fn detect(&self) -> Option<String>;
    /// Outdated packages. `ignored` is always `false` here; the engine applies settings.
    fn list_upgrades(&self, include_unknown_versions: bool) -> AppResult<Vec<Package>>;
    fn upgrade(&self, package_id: &str) -> AppResult<()>;
}

pub fn registry() -> Vec<Box<dyn PackageManager>> {
    todo!("filled in by the managers implementation")
}
