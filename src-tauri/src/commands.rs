use tauri::AppHandle;

use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::{engine, history, icons};

#[tauri::command]
pub async fn get_managers(app: AppHandle) -> AppResult<Vec<ManagerInfo>> {
    engine::get_managers(&app).await
}

#[tauri::command]
pub async fn scan_updates(app: AppHandle) -> AppResult<ScanResult> {
    engine::scan(&app).await
}

#[tauri::command]
pub fn get_last_scan(app: AppHandle) -> AppResult<Option<ScanResult>> {
    Ok(engine::get_last_scan(&app))
}

#[tauri::command]
pub async fn list_installed(app: AppHandle) -> AppResult<InstalledResult> {
    engine::list_installed(&app).await
}

#[tauri::command]
pub async fn uninstall_package(app: AppHandle, package: PackageRef) -> AppResult<()> {
    engine::uninstall(&app, package).await
}

#[tauri::command]
pub async fn update_packages(
    app: AppHandle,
    packages: Vec<PackageRef>,
) -> AppResult<UpdateSummary> {
    engine::update(&app, packages).await
}

#[tauri::command]
pub async fn update_all(app: AppHandle) -> AppResult<UpdateSummary> {
    engine::update_all(&app).await
}

#[tauri::command]
pub fn get_run_state(app: AppHandle) -> AppResult<RunState> {
    Ok(engine::get_run_state(&app))
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> AppResult<Settings> {
    Ok(engine::get_settings(&app))
}

#[tauri::command]
pub async fn save_settings(app: AppHandle, settings: Settings) -> AppResult<Settings> {
    engine::save_settings(&app, settings).await
}

#[tauri::command]
pub async fn set_package_ignored(
    app: AppHandle,
    package: PackageRef,
    ignored: bool,
) -> AppResult<Settings> {
    engine::set_package_ignored(&app, package, ignored).await
}

#[tauri::command]
pub async fn get_history(app: AppHandle, limit: usize) -> AppResult<Vec<HistoryEntry>> {
    tauri::async_runtime::spawn_blocking(move || history::list(&app, limit))
        .await
        .map_err(background_error)?
}

#[tauri::command]
pub async fn clear_history(app: AppHandle) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || history::clear(&app))
        .await
        .map_err(background_error)?
}

#[tauri::command]
pub async fn get_package_icon(
    app: AppHandle,
    package: PackageRef,
    name: String,
) -> AppResult<Option<String>> {
    tauri::async_runtime::spawn_blocking(move || icons::package_icon(&app, &package, &name))
        .await
        .map_err(background_error)?
}

#[tauri::command]
pub async fn install_winget_module(app: AppHandle) -> AppResult<()> {
    engine::install_winget_module(&app).await
}

fn background_error(error: impl std::fmt::Display) -> AppError {
    AppError::Other(format!("Hintergrundaufgabe abgebrochen: {error}"))
}
