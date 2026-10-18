use tauri::{
    AppHandle, Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::engine;
use crate::error::{AppError, AppResult};

const TRAY_ID: &str = "main-tray";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Öffnen", true, None::<&str>)?;
    let scan = MenuItem::with_id(app, "scan", "Jetzt prüfen", true, None::<&str>)?;
    let update = MenuItem::with_id(app, "update", "Alle aktualisieren", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Beenden", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &scan, &update, &separator, &quit])?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("Winget Updater – 0 Updates")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_menu(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        "open" => show_main_window(app),
        "scan" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = engine::scan(&app).await {
                    engine::report_error(&app, &error);
                }
            });
        }
        "update" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = engine::update_all(&app).await {
                    engine::report_error(&app, &error);
                }
            });
        }
        "quit" => {
            let run = engine::get_run_state(app);
            if run.scanning || run.updating {
                engine::report_error(
                    app,
                    &AppError::Other(
                        "Bitte vor dem Beenden den laufenden Vorgang abwarten.".into(),
                    ),
                );
            } else {
                app.exit(0);
            }
        }
        _ => {}
    }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        for result in [window.show(), window.unminimize(), window.set_focus()] {
            if let Err(error) = result {
                eprintln!("Fenster konnte nicht geöffnet werden: {error}");
            }
        }
    }
}

pub fn update_tooltip(app: &AppHandle, pending_updates: u32) -> AppResult<()> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_tooltip(Some(format!("Winget Updater – {pending_updates} Updates")))
            .map_err(|error| AppError::Other(format!("Tray: {error}")))?;
    }
    Ok(())
}
