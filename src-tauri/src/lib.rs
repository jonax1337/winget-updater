mod commands;
mod engine;
mod error;
mod history;
mod icons;
mod managers;
mod models;
mod process;
mod scheduler;
mod settings;
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
