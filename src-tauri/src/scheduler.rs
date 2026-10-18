use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use tauri::{AppHandle, Manager};
use tokio::sync::watch;

use crate::engine::{self, AppState};
use crate::error::AppError;
use crate::models::Settings;

pub fn start(app: &AppHandle) {
    let changes = app.state::<AppState>().schedule_changes();
    let app = app.clone();
    tauri::async_runtime::spawn(run(app, changes));
}

async fn run(app: AppHandle, mut changes: watch::Receiver<Settings>) {
    let mut preferences = changes.borrow_and_update().clone();
    let mut delay = initial_delay(&preferences);
    loop {
        publish_deadline(&app, delay);
        tokio::select! {
            changed = changes.changed() => {
                if changed.is_err() { break; }
                preferences = changes.borrow_and_update().clone();
                delay = initial_delay(&preferences);
                continue;
            }
            _ = tokio::time::sleep(delay) => {}
        }
        // Do not cancel an installer when preferences change during a running cycle.
        if let Err(error) = engine::scheduled_cycle(&app).await
            && !matches!(error, AppError::Busy)
        {
            engine::report_error(&app, &error);
        }
        if changes.has_changed().unwrap_or(false) {
            preferences = changes.borrow_and_update().clone();
            delay = initial_delay(&preferences);
        } else {
            delay = interval(&preferences);
        }
    }
}

fn initial_delay(preferences: &Settings) -> Duration {
    if preferences.update_on_startup {
        Duration::from_secs(u64::from(preferences.startup_delay_minutes) * 60)
    } else {
        interval(preferences)
    }
}

fn interval(preferences: &Settings) -> Duration {
    Duration::from_secs(u64::from(preferences.check_interval_hours) * 60 * 60)
}

fn publish_deadline(app: &AppHandle, delay: Duration) {
    let next = Utc::now() + chrono::Duration::seconds(delay.as_secs() as i64);
    engine::set_next_scan(app, next.to_rfc3339_opts(SecondsFormat::Secs, true));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_run_uses_startup_delay() {
        let preferences = Settings {
            startup_delay_minutes: 3,
            ..Settings::default()
        };
        assert_eq!(initial_delay(&preferences), Duration::from_secs(180));
    }

    #[test]
    fn disabled_startup_waits_full_interval() {
        let preferences = Settings {
            update_on_startup: false,
            check_interval_hours: 2,
            ..Settings::default()
        };
        assert_eq!(initial_delay(&preferences), Duration::from_secs(7200));
    }
}
