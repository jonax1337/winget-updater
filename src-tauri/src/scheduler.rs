use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use tauri::{AppHandle, Manager};
use tokio::sync::watch;
use tokio::time::Instant;

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
    // The startup check runs once per app start (i.e. at login), not after every settings change.
    let mut startup_pending = true;
    let mut deadline = reschedule(&app, &preferences, startup_pending);
    loop {
        tokio::select! {
            changed = changes.changed() => {
                if changed.is_err() { break; }
                let updated = changes.borrow_and_update().clone();
                // Unrelated saves (theme, ignored packages) must not postpone a pending check.
                if schedule_changed(&preferences, &updated) {
                    deadline = reschedule(&app, &updated, startup_pending);
                }
                preferences = updated;
                continue;
            }
            _ = sleep_until(deadline) => {}
        }
        startup_pending = false;
        // Do not cancel an installer when preferences change during a running cycle.
        if let Err(error) = engine::scheduled_cycle(&app).await
            && !matches!(error, AppError::Busy)
        {
            engine::report_error(&app, &error);
        }
        preferences = changes.borrow_and_update().clone();
        deadline = reschedule(&app, &preferences, startup_pending);
    }
}

fn reschedule(app: &AppHandle, preferences: &Settings, startup_pending: bool) -> Option<Instant> {
    let delay = next_delay(preferences, startup_pending);
    publish_deadline(app, delay);
    delay.map(|delay| Instant::now() + delay)
}

fn schedule_changed(previous: &Settings, updated: &Settings) -> bool {
    previous.update_on_startup != updated.update_on_startup
        || previous.startup_delay_minutes != updated.startup_delay_minutes
        || previous.periodic_checks != updated.periodic_checks
        || previous.check_interval_hours != updated.check_interval_hours
}

fn next_delay(preferences: &Settings, startup_pending: bool) -> Option<Duration> {
    if startup_pending && preferences.update_on_startup {
        return Some(Duration::from_secs(
            u64::from(preferences.startup_delay_minutes) * 60,
        ));
    }
    preferences
        .periodic_checks
        .then(|| Duration::from_secs(u64::from(preferences.check_interval_hours) * 60 * 60))
}

async fn sleep_until(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

fn publish_deadline(app: &AppHandle, delay: Option<Duration>) {
    let next = delay.map(|delay| {
        let deadline = Utc::now() + chrono::Duration::seconds(delay.as_secs() as i64);
        deadline.to_rfc3339_opts(SecondsFormat::Secs, true)
    });
    engine::set_next_scan(app, next);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Theme;

    #[test]
    fn startup_check_uses_startup_delay() {
        let preferences = Settings {
            startup_delay_minutes: 3,
            ..Settings::default()
        };
        assert_eq!(
            next_delay(&preferences, true),
            Some(Duration::from_secs(180))
        );
    }

    #[test]
    fn without_periodic_checks_nothing_follows_the_startup_check() {
        assert_eq!(next_delay(&Settings::default(), false), None);
    }

    #[test]
    fn periodic_checks_wait_full_interval() {
        let preferences = Settings {
            periodic_checks: true,
            check_interval_hours: 2,
            ..Settings::default()
        };
        assert_eq!(
            next_delay(&preferences, false),
            Some(Duration::from_secs(7200))
        );
    }

    #[test]
    fn disabled_startup_check_falls_back_to_interval() {
        let preferences = Settings {
            update_on_startup: false,
            periodic_checks: true,
            check_interval_hours: 2,
            ..Settings::default()
        };
        assert_eq!(
            next_delay(&preferences, true),
            Some(Duration::from_secs(7200))
        );
    }

    #[test]
    fn unrelated_settings_do_not_change_the_schedule() {
        let previous = Settings::default();
        let updated = Settings {
            theme: Theme::Dark,
            ignored_packages: vec!["winget:Microsoft.WSL".into()],
            notify: !previous.notify,
            ..previous.clone()
        };
        assert!(!schedule_changed(&previous, &updated));
    }

    #[test]
    fn every_schedule_field_changes_the_schedule() {
        let previous = Settings::default();
        let changed = [
            Settings {
                update_on_startup: !previous.update_on_startup,
                ..previous.clone()
            },
            Settings {
                startup_delay_minutes: previous.startup_delay_minutes + 1,
                ..previous.clone()
            },
            Settings {
                periodic_checks: !previous.periodic_checks,
                ..previous.clone()
            },
            Settings {
                check_interval_hours: previous.check_interval_hours + 1,
                ..previous.clone()
            },
        ];
        assert!(
            changed
                .iter()
                .all(|updated| schedule_changed(&previous, updated))
        );
    }
}
