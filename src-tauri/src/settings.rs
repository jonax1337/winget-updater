use std::sync::Arc;

use tauri::{AppHandle, Manager};
use tauri_plugin_store::{Store, StoreExt};

use crate::error::{AppError, AppResult};
use crate::models::{PackageRef, Settings};

const STORE_PATH: &str = "settings.json";
const SETTINGS_KEY: &str = "settings";

pub fn validate(settings: &Settings) -> AppResult<()> {
    if !(1..=168).contains(&settings.check_interval_hours) {
        return Err(AppError::Other(
            "Das Prüfintervall muss zwischen 1 und 168 Stunden liegen".into(),
        ));
    }
    if settings.startup_delay_minutes > 60 {
        return Err(AppError::Other(
            "Die Startverzögerung muss zwischen 0 und 60 Minuten liegen".into(),
        ));
    }
    Ok(())
}

pub fn initialize(app: &AppHandle) -> AppResult<Settings> {
    let store = store(app)?;
    let path = app
        .path()
        .app_data_dir()
        .map_err(storage_error)?
        .join(STORE_PATH);
    if path.try_exists().map_err(storage_error)? {
        store.reload().map_err(storage_error)?;
    }
    load(app)
}

pub fn load(app: &AppHandle) -> AppResult<Settings> {
    let settings = match store(app)?.get(SETTINGS_KEY) {
        Some(value) => serde_json::from_value(value).map_err(storage_error)?,
        None => Settings::default(),
    };
    validate(&settings)?;
    Ok(settings)
}

pub fn save(app: &AppHandle, settings: &Settings) -> AppResult<()> {
    validate(settings)?;
    let store = store(app)?;
    let previous = store.get(SETTINGS_KEY);
    store.set(
        SETTINGS_KEY,
        serde_json::to_value(settings).map_err(storage_error)?,
    );
    if let Err(error) = store.save() {
        match previous {
            Some(value) => store.set(SETTINGS_KEY, value),
            None => {
                store.delete(SETTINGS_KEY);
            }
        }
        return Err(storage_error(error));
    }
    Ok(())
}

fn store(app: &AppHandle) -> AppResult<Arc<Store<tauri::Wry>>> {
    app.store_builder(STORE_PATH)
        .disable_auto_save()
        .build()
        .map_err(storage_error)
}

pub fn package_key(package: &PackageRef) -> String {
    let manager = match package.manager {
        crate::models::ManagerId::Winget => "winget",
        crate::models::ManagerId::Scoop => "scoop",
        crate::models::ManagerId::Chocolatey => "chocolatey",
        crate::models::ManagerId::Npm => "npm",
        crate::models::ManagerId::Pip => "pip",
    };
    format!("{manager}:{}", package.id)
}

fn storage_error(error: impl std::fmt::Display) -> AppError {
    AppError::Storage(format!("Einstellungen: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ManagerId;

    #[test]
    fn defaults_are_valid() {
        assert!(validate(&Settings::default()).is_ok());
    }

    #[test]
    fn accepts_interval_boundaries() {
        for check_interval_hours in [1, 168] {
            assert!(
                validate(&Settings {
                    check_interval_hours,
                    ..Settings::default()
                })
                .is_ok()
            );
        }
    }

    #[test]
    fn rejects_interval_outside_boundaries() {
        for check_interval_hours in [0, 169, u32::MAX] {
            assert!(
                validate(&Settings {
                    check_interval_hours,
                    ..Settings::default()
                })
                .is_err()
            );
        }
    }

    #[test]
    fn accepts_startup_delay_boundaries() {
        for startup_delay_minutes in [0, 60] {
            assert!(
                validate(&Settings {
                    startup_delay_minutes,
                    ..Settings::default()
                })
                .is_ok()
            );
        }
    }

    #[test]
    fn rejects_excessive_startup_delay() {
        assert!(
            validate(&Settings {
                startup_delay_minutes: 61,
                ..Settings::default()
            })
            .is_err()
        );
    }

    #[test]
    fn ignored_keys_use_contract_manager_names_and_preserve_ids() {
        for (manager, prefix) in [
            (ManagerId::Winget, "winget"),
            (ManagerId::Scoop, "scoop"),
            (ManagerId::Chocolatey, "chocolatey"),
            (ManagerId::Npm, "npm"),
            (ManagerId::Pip, "pip"),
        ] {
            let package = PackageRef {
                manager,
                id: "@Scope/Äpp:One".into(),
            };
            assert_eq!(package_key(&package), format!("{prefix}:@Scope/Äpp:One"));
        }
    }
}
