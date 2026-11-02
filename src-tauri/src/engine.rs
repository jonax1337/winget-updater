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
use crate::failure::{self, UpdateFailure};
use crate::managers;
use crate::models::*;
use crate::running::RunningApps;
use crate::{admin, history, process, scheduler, settings, tray};

const UPDATE_EVENT: &str = "update-event";

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

/// Resolve startup consent before scheduling updates, without blocking the UI thread.
pub fn start_session(app: &AppHandle) -> AppResult<()> {
    let startup_approval = get_settings(app)
        .request_admin_on_startup
        .then(|| Operation::begin(app, Activity::Update))
        .transpose()?;
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        if startup_approval.is_some()
            && let Err(error) = authorize_admin(&handle).await
        {
            report_error(&handle, &error);
        }
        drop(startup_approval);
        scheduler::start(&handle);
    });
    Ok(())
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
                "{error}; could not restore autostart: {rollback}"
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
        recount_pending(&mut snapshot);
    }
    state.schedule.send_replace(preferences);
    publish_run_state(app);
    refresh_tray(app);
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
    // Debug builds open a console window and need the Vite dev server, so they must never be
    // registered. They must not touch the entry either: an installed release build shares it.
    if cfg!(debug_assertions) {
        return Ok(());
    }
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
    refresh_tray(app);
    if current.notify && !current.auto_update && pending_count(&result) > 0 {
        notify(
            app,
            "Updates available",
            &available_updates_message(pending_count(&result)),
        );
    }
    Ok(result)
}

/// Runs `query` concurrently on every enabled manager; managers that are not installed yield `T::default()`.
async fn query_enabled_managers<T: Default + Send + 'static>(
    preferences: &Settings,
    query: impl Fn(&dyn managers::PackageManager) -> AppResult<T> + Copy + Send + 'static,
) -> Vec<(ManagerId, AppResult<T>)> {
    let mut tasks = Vec::new();
    for manager in managers::registry() {
        if !preferences.enabled_managers.contains(&manager.id()) {
            continue;
        }
        tasks.push((
            manager.id(),
            tauri::async_runtime::spawn_blocking(move || match manager.detect() {
                Ok(None) => Ok(T::default()),
                Ok(Some(_)) => query(manager.as_ref()),
                Err(error) => Err(error),
            }),
        ));
    }
    let mut outcomes = Vec::new();
    for (manager, task) in tasks {
        let outcome = task
            .await
            .unwrap_or_else(|error| Err(AppError::Other(format!("Check aborted: {error}"))));
        outcomes.push((manager, outcome));
    }
    outcomes
}

fn manager_errors(manager: ManagerId, messages: Vec<String>) -> impl Iterator<Item = ManagerError> {
    messages
        .into_iter()
        .map(move |message| ManagerError { manager, message })
}

fn failure_as_error(manager: ManagerId, error: AppError) -> ManagerError {
    ManagerError {
        manager,
        message: error.to_string(),
    }
}

async fn collect_upgrades(preferences: &Settings) -> ScanResult {
    let include_unknown = preferences.include_unknown_versions;
    let outcomes = query_enabled_managers(preferences, move |manager| {
        manager.list_upgrades(include_unknown)
    })
    .await;
    let mut result = ScanResult {
        packages: Vec::new(),
        errors: Vec::new(),
        scanned_at: timestamp(),
    };
    for (manager, outcome) in outcomes {
        match outcome {
            Ok(upgrades) => {
                result.packages.extend(upgrades.packages);
                result
                    .errors
                    .extend(manager_errors(manager, upgrades.warnings));
            }
            Err(error) => result.errors.push(failure_as_error(manager, error)),
        }
    }
    result.scanned_at = timestamp();
    result
}

pub async fn list_installed(app: &AppHandle) -> AppResult<InstalledResult> {
    let preferences = get_settings(app);
    let outcomes = query_enabled_managers(&preferences, |manager| manager.list_installed()).await;
    let mut result = InstalledResult {
        packages: Vec::new(),
        errors: Vec::new(),
        listed_at: timestamp(),
    };
    for (manager, outcome) in outcomes {
        match outcome {
            Ok(installed) => {
                result.packages.extend(installed.packages);
                result
                    .errors
                    .extend(manager_errors(manager, installed.warnings));
            }
            Err(error) => result.errors.push(failure_as_error(manager, error)),
        }
    }
    result.listed_at = timestamp();
    Ok(result)
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
    let result = update_locked(app, automatic_packages(app), UpdateMode::Scheduled).await;
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
            AppError::Other(format!(
                "Package not found in the last scan: {}",
                reference.id
            ))
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
    Scheduled,
}

impl UpdateMode {
    fn may_request_admin(&self) -> bool {
        !matches!(self, Self::Scheduled)
    }
}

fn still_automatic(package: &Package, settings: &Settings) -> bool {
    settings.enabled_managers.contains(&package.manager)
        && !settings
            .ignored_packages
            .contains(&settings::package_key(&package_ref(package)))
}

fn available_updates_message(count: u32) -> String {
    if count == 1 {
        "1 package can be updated.".into()
    } else {
        format!("{count} packages can be updated.")
    }
}

async fn update_locked(app: &AppHandle, packages: Vec<Package>, mode: UpdateMode) -> UpdateRun {
    let mut run = UpdateRun {
        summary: UpdateSummary::default(),
        storage_error: None,
    };
    let queue = busy_apps_last(packages, &running_apps().await);
    for (package, processes) in &queue {
        publish_update(
            app,
            package,
            UpdateStatus::Queued,
            queued_message(&package.name, processes),
        );
    }
    let admin_failure = if queue
        .iter()
        .any(|(package, _)| admin::needs_admin(package.manager))
    {
        prepare_update_access(app, &mode)
            .await
            .err()
            .map(UpdateFailure::from)
    } else {
        None
    };
    for (package, processes) in queue {
        if !matches!(mode, UpdateMode::Manual) && !still_automatic(&package, &get_settings(app)) {
            publish_update(
                app,
                &package,
                UpdateStatus::Skipped,
                Some("Skipped – package is now ignored or its source is disabled".into()),
            );
            continue;
        }
        update_one(app, package, &processes, admin_failure.as_ref(), &mut run).await;
    }
    if let Err(error) = scan_locked(app).await {
        report_error(app, &error);
    }
    run
}

async fn running_apps() -> RunningApps {
    blocking(RunningApps::query).await.unwrap_or_else(|error| {
        eprintln!("Could not check running apps: {error}");
        RunningApps::default()
    })
}

/// Running apps are updated last, which leaves the most time to close them.
fn busy_apps_last(packages: Vec<Package>, running: &RunningApps) -> Vec<(Package, Vec<String>)> {
    let mut queue: Vec<_> = packages
        .into_iter()
        .map(|package| {
            let processes = running.processes_of(&package.name);
            (package, processes)
        })
        .collect();
    queue.sort_by_key(|(_, processes)| !processes.is_empty());
    queue
}

fn queued_message(app_name: &str, processes: &[String]) -> Option<String> {
    (!processes.is_empty()).then(|| {
        format!(
            "{app_name} is running ({}), so it is updated last.",
            processes.join(", ")
        )
    })
}

async fn update_one(
    app: &AppHandle,
    package: Package,
    running: &[String],
    admin_failure: Option<&UpdateFailure>,
    run: &mut UpdateRun,
) {
    publish_update(app, &package, UpdateStatus::Running, None);
    let event = match upgrade_with_access(app, package_ref(&package), admin_failure).await {
        Ok(()) => {
            run.summary.succeeded += 1;
            update_event(&package, UpdateStatus::Succeeded, None)
        }
        Err(error) => {
            run.summary.failed += 1;
            failed_event(&package, error, running)
        }
    };
    let entry = history_entry(&package, &event);
    let handle = app.clone();
    if let Err(error) = blocking(move || history::append(&handle, entry)).await {
        report_error(app, &error);
        run.storage_error.get_or_insert(error);
    }
    emit(app, UPDATE_EVENT, event);
}

async fn upgrade_with_access(
    app: &AppHandle,
    package: PackageRef,
    admin_failure: Option<&UpdateFailure>,
) -> AppResult<()> {
    if !admin::needs_admin(package.manager) {
        return blocking(move || managers::upgrade(&package)).await;
    }
    if let Some(failure) = admin_failure {
        return Err(AppError::Update(failure.clone()));
    }
    execute_admin(
        app,
        admin::Request {
            action: admin::Action::Upgrade,
            manager: package.manager,
            id: package.id,
        },
    )
    .await
}

fn failed_event(package: &Package, error: AppError, running: &[String]) -> UpdateEvent {
    let installer_failed = matches!(
        error,
        AppError::Command(_) | AppError::Update(_) | AppError::Timeout(_)
    );
    let failure = UpdateFailure::from(error);
    let failure = if installer_failed {
        failure::attribute_to_running_app(failure, running)
    } else {
        failure
    };
    UpdateEvent {
        message: Some(failure::explain(&failure, &package.name, running)),
        reason: Some(failure.reason),
        details: Some(failure.details),
        ..update_event(package, UpdateStatus::Failed, None)
    }
}

fn history_entry(package: &Package, event: &UpdateEvent) -> HistoryEntry {
    HistoryEntry {
        package: event.package.clone(),
        name: package.name.clone(),
        from_version: package.current_version.clone(),
        to_version: package.available_version.clone(),
        status: event.status,
        message: event.message.clone(),
        reason: event.reason,
        details: event.details.clone(),
        finished_at: timestamp(),
    }
}

pub async fn uninstall(app: &AppHandle, package: PackageRef) -> AppResult<()> {
    let _operation = Operation::begin(app, Activity::Uninstall)?;
    let target = package.clone();
    if admin::needs_admin(target.manager) {
        authorize_admin(app).await?;
        execute_admin(
            app,
            admin::Request {
                action: admin::Action::Uninstall,
                manager: target.manager,
                id: target.id,
            },
        )
        .await?;
    } else {
        blocking(move || managers::available(&target)?.uninstall(&target.id)).await?;
    }
    forget_uninstalled(app, &package);
    Ok(())
}

pub async fn install(app: &AppHandle, package_id: String) -> AppResult<()> {
    let _operation = Operation::begin(app, Activity::Install)?;
    process::validate_package_id(&package_id)?;
    authorize_admin(app).await?;
    execute_admin(
        app,
        admin::Request {
            action: admin::Action::Install,
            manager: ManagerId::Winget,
            id: package_id,
        },
    )
    .await
}

pub async fn enable_admin_access(app: &AppHandle) -> AppResult<()> {
    let _operation = Operation::begin(app, Activity::Update)?;
    authorize_admin(app).await
}

pub async fn disable_admin_access(app: &AppHandle) -> AppResult<()> {
    let _operation = Operation::begin(app, Activity::Update)?;
    app.state::<admin::AdminSession>().revoke().await;
    publish_admin_access(app).await;
    Ok(())
}

async fn prepare_update_access(app: &AppHandle, mode: &UpdateMode) -> AppResult<()> {
    if mode.may_request_admin() {
        return authorize_admin(app).await;
    }
    if app.state::<admin::AdminSession>().authorized().await {
        return Ok(());
    }
    Err(AppError::Update(UpdateFailure::new(
        failure::FailureReason::NeedsAdmin,
        "Background system updates need administrator access. Authorize the session in Settings.",
    )))
}

async fn authorize_admin(app: &AppHandle) -> AppResult<()> {
    let result = app.state::<admin::AdminSession>().authorize().await;
    publish_admin_access(app).await;
    result
}

async fn execute_admin(app: &AppHandle, request: admin::Request) -> AppResult<()> {
    let result = app.state::<admin::AdminSession>().execute(request).await;
    publish_admin_access(app).await;
    result
}

async fn publish_admin_access(app: &AppHandle) {
    emit(
        app,
        "admin-access",
        app.state::<admin::AdminSession>().authorized().await,
    );
}

fn forget_uninstalled(app: &AppHandle, package: &PackageRef) {
    {
        let state = app.state::<AppState>();
        let mut snapshot = state.snapshot();
        if let Some(scan) = snapshot.last_scan.as_mut() {
            remove_package(scan, package);
        }
        recount_pending(&mut snapshot);
    }
    publish_run_state(app);
    refresh_tray(app);
}

fn remove_package(scan: &mut ScanResult, package: &PackageRef) {
    scan.packages
        .retain(|candidate| package_ref(candidate) != *package);
}

fn finish_update(app: &AppHandle, run: UpdateRun) -> AppResult<UpdateSummary> {
    emit(app, "update-finished", run.summary.clone());
    if get_settings(app).notify {
        notify(
            app,
            "Update complete",
            &format!(
                "{} succeeded, {} failed.",
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

pub fn set_next_scan(app: &AppHandle, next: Option<String>) {
    app.state::<AppState>().snapshot().run.next_scan = next;
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

fn recount_pending(snapshot: &mut Snapshot) {
    snapshot.run.pending_updates = snapshot.last_scan.as_ref().map_or(0, pending_count);
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
        .map_err(|error| AppError::Other(format!("Background task aborted: {error}")))?
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
    emit(app, UPDATE_EVENT, update_event(package, status, message));
}

fn update_event(package: &Package, status: UpdateStatus, message: Option<String>) -> UpdateEvent {
    UpdateEvent {
        package: package_ref(package),
        status,
        message,
        reason: None,
        details: None,
    }
}

fn emit(app: &AppHandle, event: &str, payload: impl Serialize + Clone) {
    if let Err(error) = app.emit(event, payload) {
        eprintln!("Could not send event {event}: {error}");
    }
}

fn refresh_tray(app: &AppHandle) {
    if let Err(error) = tray::show_pending_updates(app, get_run_state(app).pending_updates) {
        eprintln!("Could not update tray: {error}");
    }
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    if let Err(error) = app.notification().builder().title(title).body(body).show() {
        eprintln!("Could not show notification: {error}");
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
    Uninstall,
    Install,
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
            // The UI has no separate uninstall or install state; all of them change installed packages.
            snapshot.run.updating = matches!(
                activity,
                Activity::Update | Activity::Uninstall | Activity::Install
            );
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
    fn background_updates_never_request_new_uac_approval() {
        assert!(!UpdateMode::Scheduled.may_request_admin());
        assert!(UpdateMode::Automatic.may_request_admin());
        assert!(UpdateMode::Manual.may_request_admin());
    }

    #[test]
    fn session_errors_are_not_misreported_as_locked_files() {
        let event = failed_event(
            &package(),
            AppError::Other("Administrator session disconnected".into()),
            &["app.exe".into()],
        );
        assert_eq!(event.reason, Some(failure::FailureReason::Unknown));
        assert_eq!(
            event.message.as_deref(),
            Some("Administrator session disconnected")
        );
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
    fn scan_with(packages: Vec<Package>) -> ScanResult {
        ScanResult {
            packages,
            errors: Vec::new(),
            scanned_at: "2025-01-01T00:00:00Z".into(),
        }
    }
    #[test]
    fn removing_uninstalled_package_keeps_other_managers_with_same_id() {
        let mut other_manager = package();
        other_manager.manager = ManagerId::Scoop;
        let mut scan = scan_with(vec![package(), other_manager]);
        remove_package(&mut scan, &package_ref(&package()));
        assert_eq!(scan.packages.len(), 1);
        assert_eq!(scan.packages[0].manager, ManagerId::Scoop);
    }
    #[test]
    fn removing_unknown_package_changes_nothing() {
        let mut scan = scan_with(vec![package()]);
        let unknown = PackageRef {
            manager: ManagerId::Npm,
            id: "other".into(),
        };
        remove_package(&mut scan, &unknown);
        assert_eq!(scan.packages.len(), 1);
    }
    #[test]
    fn pending_count_after_removal_ignores_ignored_packages() {
        let mut ignored = package();
        ignored.id = "Vendor.Ignored".into();
        ignored.ignored = true;
        let mut scan = scan_with(vec![package(), ignored]);
        assert_eq!(pending_count(&scan), 1);
        remove_package(&mut scan, &package_ref(&package()));
        assert_eq!(pending_count(&scan), 0);
    }
    #[test]
    fn notification_uses_singular_and_plural() {
        assert_eq!(available_updates_message(1), "1 package can be updated.");
        assert_eq!(available_updates_message(2), "2 packages can be updated.");
    }
}
