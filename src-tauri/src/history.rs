use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};
use tauri_plugin_store::{Store, StoreExt};

use crate::error::{AppError, AppResult};
use crate::models::HistoryEntry;

const STORE_PATH: &str = "history.json";
const ENTRIES_KEY: &str = "entries";
const MAX_ENTRIES: usize = 500;
static HISTORY_LOCK: Mutex<()> = Mutex::new(());

pub fn initialize(app: &AppHandle) -> AppResult<()> {
    let store = store(app)?;
    let path = app
        .path()
        .app_data_dir()
        .map_err(storage_error)?
        .join(STORE_PATH);
    if path.try_exists().map_err(storage_error)? {
        store.reload().map_err(storage_error)?;
    }
    read_entries(&store).map(|_| ())
}

pub fn append(app: &AppHandle, entry: HistoryEntry) -> AppResult<()> {
    let _guard = HISTORY_LOCK.lock().map_err(storage_error)?;
    let store = store(app)?;
    let mut entries = read_entries(&store)?;
    prepend_capped(&mut entries, entry);
    write_entries(&store, &entries)
}

pub fn list(app: &AppHandle, limit: usize) -> AppResult<Vec<HistoryEntry>> {
    let _guard = HISTORY_LOCK.lock().map_err(storage_error)?;
    let store = store(app)?;
    let mut entries = read_entries(&store)?;
    entries.truncate(limit.min(MAX_ENTRIES));
    Ok(entries)
}

pub fn clear(app: &AppHandle) -> AppResult<()> {
    let _guard = HISTORY_LOCK.lock().map_err(storage_error)?;
    let store = store(app)?;
    write_entries(&store, &[])
}

fn store(app: &AppHandle) -> AppResult<Arc<Store<tauri::Wry>>> {
    app.store_builder(STORE_PATH)
        .disable_auto_save()
        .build()
        .map_err(storage_error)
}

fn read_entries(store: &Store<tauri::Wry>) -> AppResult<Vec<HistoryEntry>> {
    match store.get(ENTRIES_KEY) {
        Some(value) => serde_json::from_value(value).map_err(storage_error),
        None => Ok(Vec::new()),
    }
}

fn prepend_capped(entries: &mut Vec<HistoryEntry>, entry: HistoryEntry) {
    entries.insert(0, entry);
    entries.truncate(MAX_ENTRIES);
}

fn write_entries(store: &Store<tauri::Wry>, entries: &[HistoryEntry]) -> AppResult<()> {
    let previous = store.get(ENTRIES_KEY);
    store.set(
        ENTRIES_KEY,
        serde_json::to_value(entries).map_err(storage_error)?,
    );
    if let Err(error) = store.save() {
        match previous {
            Some(value) => store.set(ENTRIES_KEY, value),
            None => {
                store.delete(ENTRIES_KEY);
            }
        }
        return Err(storage_error(error));
    }
    Ok(())
}

fn storage_error(error: impl std::fmt::Display) -> AppError {
    AppError::Storage(format!("Verlauf: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ManagerId, PackageRef, UpdateStatus};

    fn entry(index: usize) -> HistoryEntry {
        HistoryEntry {
            package: PackageRef {
                manager: ManagerId::Winget,
                id: index.to_string(),
            },
            name: index.to_string(),
            from_version: "1".into(),
            to_version: "2".into(),
            status: UpdateStatus::Succeeded,
            message: None,
            finished_at: index.to_string(),
        }
    }

    #[test]
    fn inserts_newest_entry_first() {
        let mut entries = vec![entry(1)];
        prepend_capped(&mut entries, entry(2));
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            ["2", "1"]
        );
    }

    #[test]
    fn retains_only_newest_five_hundred_entries() {
        let mut entries = Vec::new();
        for index in 0..502 {
            prepend_capped(&mut entries, entry(index));
        }
        assert_eq!(entries.len(), MAX_ENTRIES);
        assert_eq!(entries.first().unwrap().name, "501");
        assert_eq!(entries.last().unwrap().name, "2");
    }
}
