mod catalog;
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

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            tray::show_main_window(app)
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--minimized"])
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_managers,
            commands::scan_updates,
            commands::get_last_scan,
            commands::list_installed,
            commands::uninstall_package,
            commands::update_packages,
            commands::update_all,
            commands::get_run_state,
            commands::get_settings,
            commands::save_settings,
            commands::set_package_ignored,
            commands::get_history,
            commands::clear_history,
            commands::get_package_icon,
            commands::install_winget_module,
            commands::search_catalog,
            commands::install_catalog_package,
        ])
        .setup(|app| {
            let handle = app.handle();
            #[cfg(desktop)]
            handle.plugin(tauri_plugin_updater::Builder::new().build())?;
            let preferences = settings::initialize(handle)?;
            history::initialize(handle)?;
            engine::apply_autostart(handle, preferences.autostart)?;
            app.manage(engine::AppState::new(preferences));
            tray::create(handle)?;
            scheduler::start(handle);
            if !std::env::args().any(|argument| argument == "--minimized") {
                tray::show_main_window(handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                if let Err(error) = window.hide() {
                    eprintln!("Could not hide window: {error}");
                }
            }
            tauri::WindowEvent::ThemeChanged(_)
                if window.try_state::<engine::AppState>().is_some() =>
            {
                let app = window.app_handle();
                let pending = engine::get_run_state(app).pending_updates;
                if let Err(error) = tray::show_pending_updates(app, pending) {
                    eprintln!("Could not update tray appearance: {error}");
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("Could not start Winget Updater");
}
