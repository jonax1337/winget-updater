use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_notification::NotificationExt;
use tokio::sync::{OwnedMutexGuard, watch};

use crate::error::{AppError, AppResult};
use crate::managers;
use crate::models::*;
use crate::{history, process, settings, tray};

pub struct AppState {
    snapshot: Mutex<Snapshot>,
    operation: Arc<tokio::sync::Mutex<()>>,
    settings_write: tokio::sync::Mutex<()>,
    schedule: watch::Sender<Settings>,
}

struct Snapshot {
    settings: Settings,
    last_scan: Option<ScanResult>,
    run: RunState,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        let (schedule, _) = watch::channel(settings.clone());
        Self {
            snapshot: Mutex::new(Snapshot {
                settings,
                last_scan: None,
                run: RunState::default(),
            }),
            operation: Arc::new(tokio::sync::Mutex::new(())),
            settings_write: tokio::sync::Mutex::new(()),
            schedule,
        }
    }

    fn snapshot(&self) -> MutexGuard<'_, Snapshot> {
        self.snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn schedule_changes(&self) -> watch::Receiver<Settings> {
        self.schedule.subscribe()
    }
}

pub fn get_settings(app: &AppHandle) -> Settings {
    app.state::<AppState>().snapshot().settings.clone()
}

pub fn get_run_state(app: &AppHandle) -> RunState {
    app.state::<AppState>().snapshot().run.clone()
}

pub fn get_last_scan(app: &AppHandle) -> Option<ScanResult> {
    app.state::<AppState>().snapshot().last_scan.clone()
}

pub async fn save_settings(app: &AppHandle, preferences: Settings) -> AppResult<Settings> {
    let state = app.state::<AppState>();
    let _write = state.settings_write.lock().await;
    persist_settings(app, preferences).await
}

async fn persist_settings(app: &AppHandle, preferences: Settings) -> AppResult<Settings> {
    settings::validate(&preferences)?;
    let previous = get_settings(app);
    apply_autostart(app, preferences.autostart)?;
    let handle = app.clone();
    let saved = preferences.clone();
    let result = blocking(move || settings::save(&handle, &saved)).await;
    if let Err(error) = result {
        if let Err(rollback) = apply_autostart(app, previous.autostart) {
            return Err(AppError::Storage(format!(
                "{error}; Autostart nicht wiederhergestellt: {rollback}"
            )));
        }
        return Err(error);
    }
    commit_settings(app, preferences.clone());
    Ok(preferences)
}

fn commit_settings(app: &AppHandle, preferences: Settings) {
    let state = app.state::<AppState>();
    {
        let mut snapshot = state.snapshot();
        snapshot.settings = preferences.clone();
        if let Some(scan) = snapshot.last_scan.as_mut() {
            apply_ignored(&mut scan.packages, &preferences);
        }
        snapshot.run.pending_updates = snapshot.last_scan.as_ref().map_or(0, pending_count);
    }
    state.schedule.send_replace(preferences);
    publish_run_state(app);
    refresh_tooltip(app);
}

pub async fn set_package_ignored(
    app: &AppHandle,
    package: PackageRef,
    ignored: bool,
) -> AppResult<Settings> {
    let state = app.state::<AppState>();
    let _write = state.settings_write.lock().await;
    let mut preferences = get_settings(app);
    let key = settings::package_key(&package);
    preferences
        .ignored_packages
        .retain(|existing| existing != &key);
    if ignored {
        preferences.ignored_packages.push(key);
    }
    persist_settings(app, preferences).await
}

pub fn apply_autostart(app: &AppHandle, enabled: bool) -> AppResult<()> {
    let autolaunch = app.autolaunch();
    let result = if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    result.map_err(|error| AppError::Other(format!("Autostart: {error}")))
}

pub async fn get_managers(app: &AppHandle) -> AppResult<Vec<ManagerInfo>> {
    let preferences = get_settings(app);
    blocking(move || {
        Ok(managers::registry()
            .into_iter()
            .map(|manager| {
                let version = manager.detect().ok().flatten();
                ManagerInfo {
                    id: manager.id(),
                    name: manager.display_name().into(),
                    available: version.is_some(),
                    version,
                    enabled: preferences.enabled_managers.contains(&manager.id()),
                    requires_admin: manager.requires_admin(),
                }
            })
            .collect())
    })
    .await
}

pub async fn scan(app: &AppHandle) -> AppResult<ScanResult> {
    let operation = Operation::begin(app, Activity::Scan)?;
    let result = scan_locked(app).await;
    drop(operation);
    result
}

async fn scan_locked(app: &AppHandle) -> AppResult<ScanResult> {
    set_scanning(app, true);
    let preferences = get_settings(app);
    let mut result = collect_upgrades(&preferences).await;
    store_scan(app, &mut result);
    let current = get_settings(app);
    emit(app, "scan-finished", result.clone());
    refresh_tooltip(app);
    if current.notify && !current.auto_update && pending_count(&result) > 0 {
        notify(
            app,
            "Updates verfügbar",
            &available_updates_message(pending_count(&result)),
        );
    }
    Ok(result)
}

async fn collect_upgrades(preferences: &Settings) -> ScanResult {
    let mut tasks = Vec::new();
    for manager in managers::registry() {
        if !preferences.enabled_managers.contains(&manager.id()) {
            continue;
        }
        let id = manager.id();
        let include_unknown = preferences.include_unknown_versions;
        tasks.push((
            id,
            tauri::async_runtime::spawn_blocking(move || match manager.detect() {
                Ok(None) => Ok(managers::Upgrades::default()),
                Ok(Some(_)) => manager.list_upgrades(include_unknown),
                Err(error) => Err(error),
            }),
        ));
    }
    let mut result = ScanResult {
        packages: Vec::new(),
        errors: Vec::new(),
        scanned_at: timestamp(),
    };
    for (manager, task) in tasks {
        match task.await {
            Ok(Ok(upgrades)) => {
                result.packages.extend(upgrades.packages);
                result.errors.extend(
                    upgrades
                        .warnings
                        .into_iter()
                        .map(|message| ManagerError { manager, message }),
                );
            }
            Ok(Err(error)) => result.errors.push(ManagerError {
                manager,
                message: error.to_string(),
            }),
            Err(error) => result.errors.push(ManagerError {
                manager,
                message: format!("Prüfung abgebrochen: {error}"),
            }),
        }
    }
    result.scanned_at = timestamp();
    result
}

fn apply_ignored(packages: &mut [Package], preferences: &Settings) {
    let ignored: HashSet<_> = preferences.ignored_packages.iter().collect();
    for package in packages {
        let key = settings::package_key(&package_ref(package));
        package.ignored = ignored.contains(&key);
    }
}

fn store_scan(app: &AppHandle, result: &mut ScanResult) {
    let state = app.state::<AppState>();
    let mut snapshot = state.snapshot();
    apply_ignored(&mut result.packages, &snapshot.settings);
    snapshot.run.scanning = false;
    snapshot.run.last_scan = Some(result.scanned_at.clone());
    snapshot.run.pending_updates = pending_count(result);
    snapshot.last_scan = Some(result.clone());
    drop(snapshot);
    publish_run_state(app);
}

pub async fn update(app: &AppHandle, packages: Vec<PackageRef>) -> AppResult<UpdateSummary> {
    let operation = Operation::begin(app, Activity::Update)?;
    let packages = resolve_packages(app, packages)?;
    let result = update_locked(app, packages, UpdateMode::Manual).await;
    drop(operation);
    finish_update(app, result)
}

pub async fn update_all(app: &AppHandle) -> AppResult<UpdateSummary> {
    let operation = Operation::begin(app, Activity::Update)?;
    let packages = automatic_packages(app);
    let result = update_locked(app, packages, UpdateMode::Automatic).await;
    drop(operation);
    finish_update(app, result)
}

pub async fn scheduled_cycle(app: &AppHandle) -> AppResult<()> {
    let operation = Operation::begin(app, Activity::Scan)?;
    scan_locked(app).await?;
    if !get_settings(app).auto_update {
        return Ok(());
    }
    set_updating(app);
    let result = update_locked(app, automatic_packages(app), UpdateMode::Automatic).await;
    drop(operation);
    finish_update(app, result).map(|_| ())
}

fn automatic_packages(app: &AppHandle) -> Vec<Package> {
    let state = app.state::<AppState>();
    let snapshot = state.snapshot();
    snapshot.last_scan.as_ref().map_or_else(Vec::new, |scan| {
        scan.packages
            .iter()
            .filter(|package| {
                !package.ignored
                    && snapshot
                        .settings
                        .enabled_managers
                        .contains(&package.manager)
            })
            .cloned()
            .collect()
    })
}

fn resolve_packages(app: &AppHandle, requested: Vec<PackageRef>) -> AppResult<Vec<Package>> {
    let state = app.state::<AppState>();
    let snapshot = state.snapshot();
    let mut seen = HashSet::new();
    let mut packages = Vec::new();
    for reference in requested {
        if !seen.insert(reference.clone()) {
            continue;
        }
        let found = snapshot.last_scan.as_ref().and_then(|scan| {
            scan.packages
                .iter()
                .find(|package| package_ref(package) == reference)
        });
        let package = found.ok_or_else(|| {
            AppError::Other(format!("Paket nicht im letzten Scan: {}", reference.id))
        })?;
        packages.push(package.clone());
    }
    Ok(packages)
}

struct UpdateRun {
    summary: UpdateSummary,
    storage_error: Option<AppError>,
}

enum UpdateMode {
    Manual,
    Automatic,
}

fn still_automatic(package: &Package, settings: &Settings) -> bool {
    settings.enabled_managers.contains(&package.manager)
        && !settings
            .ignored_packages
            .contains(&settings::package_key(&package_ref(package)))
}

fn available_updates_message(count: u32) -> String {
    if count == 1 {
        "1 Paket kann aktualisiert werden.".into()
    } else {
        format!("{count} Pakete können aktualisiert werden.")
    }
}

async fn update_locked(app: &AppHandle, packages: Vec<Package>, mode: UpdateMode) -> UpdateRun {
    let mut run = UpdateRun {
        summary: UpdateSummary::default(),
        storage_error: None,
    };
    for package in &packages {
        publish_update(app, package, UpdateStatus::Queued, None);
    }
    for package in packages {
        if matches!(mode, UpdateMode::Automatic) && !still_automatic(&package, &get_settings(app)) {
            publish_update(
                app,
                &package,
                UpdateStatus::Skipped,
                Some("Übersprungen – inzwischen ignoriert oder Paketmanager deaktiviert".into()),
            );
            continue;
        }
        update_one(app, package, &mut run).await;
    }
    if let Err(error) = scan_locked(app).await {
        report_error(app, &error);
    }
    run
}

async fn update_one(app: &AppHandle, package: Package, run: &mut UpdateRun) {
    publish_update(app, &package, UpdateStatus::Running, None);
    let reference = package_ref(&package);
    let request = reference.clone();
    let result = blocking(move || upgrade_package(request)).await;
    let (status, message) = match result {
        Ok(()) => {
            run.summary.succeeded += 1;
            (UpdateStatus::Succeeded, None)
        }
        Err(error) => {
            run.summary.failed += 1;
            (UpdateStatus::Failed, Some(error.to_string()))
        }
    };
    let entry = HistoryEntry {
        package: reference,
        name: package.name.clone(),
        from_version: package.current_version.clone(),
        to_version: package.available_version.clone(),
        status,
        message: message.clone(),
        finished_at: timestamp(),
    };
    let handle = app.clone();
    if let Err(error) = blocking(move || history::append(&handle, entry)).await {
        report_error(app, &error);
        run.storage_error.get_or_insert(error);
    }
    publish_update(app, &package, status, message);
}

fn upgrade_package(package: PackageRef) -> AppResult<()> {
    let manager = managers::registry()
        .into_iter()
        .find(|manager| manager.id() == package.manager)
        .ok_or_else(|| AppError::ManagerUnavailable(settings::package_key(&package)))?;
    match manager.detect()? {
        None => Err(AppError::ManagerUnavailable(manager.display_name().into())),
        Some(_) => manager.upgrade(&package.id),
    }
}

fn finish_update(app: &AppHandle, run: UpdateRun) -> AppResult<UpdateSummary> {
    emit(app, "update-finished", run.summary.clone());
    if get_settings(app).notify {
        notify(
            app,
            "Aktualisierung abgeschlossen",
            &format!(
                "{} erfolgreich, {} fehlgeschlagen.",
                run.summary.succeeded, run.summary.failed
            ),
        );
    }
    match run.storage_error {
        Some(error) => Err(error),
        None => Ok(run.summary),
    }
}

const INSTALL_WINGET_MODULE: &str = "$ErrorActionPreference='Stop'; [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12; if (-not (Get-PackageProvider -ListAvailable -Name NuGet -ErrorAction SilentlyContinue | Where-Object { $_.Version -ge [version]'2.8.5.201' })) { Install-PackageProvider -Name NuGet -MinimumVersion 2.8.5.201 -Scope CurrentUser -Force | Out-Null }; Install-Module Microsoft.WinGet.Client -Scope CurrentUser -Force -AllowClobber";

pub async fn install_winget_module(app: &AppHandle) -> AppResult<()> {
    let _operation = Operation::begin(app, Activity::Update)?;
    blocking(|| {
        process::run(
            "powershell.exe",
            &[
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                INSTALL_WINGET_MODULE,
            ],
            Duration::from_secs(30 * 60),
        )?
        .ensure_success("WinGet-Modul installieren")?;
        Ok(())
    })
    .await
}

pub fn set_next_scan(app: &AppHandle, next: String) {
    app.state::<AppState>().snapshot().run.next_scan = Some(next);
    publish_run_state(app);
}

fn set_scanning(app: &AppHandle, scanning: bool) {
    app.state::<AppState>().snapshot().run.scanning = scanning;
    publish_run_state(app);
}

fn set_updating(app: &AppHandle) {
    app.state::<AppState>().snapshot().run.updating = true;
    publish_run_state(app);
}

fn pending_count(scan: &ScanResult) -> u32 {
    scan.packages
        .iter()
        .filter(|package| !package.ignored)
        .count() as u32
}

fn package_ref(package: &Package) -> PackageRef {
    PackageRef {
        manager: package.manager,
        id: package.id.clone(),
    }
}

fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| AppError::Other(format!("Hintergrundaufgabe abgebrochen: {error}")))?
}

fn publish_run_state(app: &AppHandle) {
    emit(app, "run-state", get_run_state(app));
}

fn publish_update(
    app: &AppHandle,
    package: &Package,
    status: UpdateStatus,
    message: Option<String>,
) {
    emit(
        app,
        "update-event",
        UpdateEvent {
            package: package_ref(package),
            status,
            message,
        },
    );
}

fn emit(app: &AppHandle, event: &str, payload: impl Serialize + Clone) {
    if let Err(error) = app.emit(event, payload) {
        eprintln!("Ereignis {event} konnte nicht gesendet werden: {error}");
    }
}

fn refresh_tooltip(app: &AppHandle) {
    if let Err(error) = tray::update_tooltip(app, get_run_state(app).pending_updates) {
        eprintln!("Tray-Tooltip konnte nicht aktualisiert werden: {error}");
    }
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    if let Err(error) = app.notification().builder().title(title).body(body).show() {
        eprintln!("Benachrichtigung konnte nicht angezeigt werden: {error}");
    }
}

pub fn report_error(app: &AppHandle, error: &AppError) {
    eprintln!("Winget Updater: {error}");
    if get_settings(app).notify {
        notify(app, "Winget Updater – Fehler", &error.to_string());
    }
}

enum Activity {
    Scan,
    Update,
}

struct Operation {
    app: AppHandle,
    _guard: OwnedMutexGuard<()>,
}

impl Operation {
    fn begin(app: &AppHandle, activity: Activity) -> AppResult<Self> {
        let state = app.state::<AppState>();
        let guard = state
            .operation
            .clone()
            .try_lock_owned()
            .map_err(|_| AppError::Busy)?;
        {
            let mut snapshot = state.snapshot();
            snapshot.run.scanning = matches!(activity, Activity::Scan);
            snapshot.run.updating = matches!(activity, Activity::Update);
        }
        publish_run_state(app);
        Ok(Self {
            app: app.clone(),
            _guard: guard,
        })
    }
}

impl Drop for Operation {
    fn drop(&mut self) {
        {
            let state = self.app.state::<AppState>();
            let mut snapshot = state.snapshot();
            snapshot.run.scanning = false;
            snapshot.run.updating = false;
        }
        publish_run_state(&self.app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn package() -> Package {
        Package {
            manager: ManagerId::Winget,
            id: "Vendor.App".into(),
            name: "App".into(),
            current_version: "1".into(),
            available_version: "2".into(),
            source: None,
            ignored: false,
        }
    }
    #[test]
    fn automatic_update_allows_enabled_package() {
        assert!(still_automatic(&package(), &Settings::default()));
    }
    #[test]
    fn automatic_update_rejects_ignored_package() {
        let preferences = Settings {
            ignored_packages: vec!["winget:Vendor.App".into()],
            ..Settings::default()
        };
        assert!(!still_automatic(&package(), &preferences));
    }
    #[test]
    fn automatic_update_rejects_disabled_manager() {
        let preferences = Settings {
            enabled_managers: vec![],
            ..Settings::default()
        };
        assert!(!still_automatic(&package(), &preferences));
    }
    #[test]
    fn notification_uses_singular_and_plural() {
        assert_eq!(
            available_updates_message(1),
            "1 Paket kann aktualisiert werden."
        );
        assert_eq!(
            available_updates_message(2),
            "2 Pakete können aktualisiert werden."
        );
    }
}
