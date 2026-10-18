use crate::error::AppResult;
use crate::models::PackageRef;

// Placeholder until the icons implementation lands; keeps the crate compiling.
pub fn package_icon(_app: &tauri::AppHandle, _package: &PackageRef, _name: &str) -> AppResult<Option<String>> {
    Ok(None)
}
