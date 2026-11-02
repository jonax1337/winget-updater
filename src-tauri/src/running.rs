//! Finds running processes that belong to an installed app, so updates of busy apps can run last
//! and failures can name what to close.

use std::path::Path;

use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::process::{self, LIST_TIMEOUT};

const QUERY: &str = r#"$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); $keys='HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*','HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*'; $apps=@(Get-ItemProperty $keys -ErrorAction SilentlyContinue | Where-Object DisplayName | Select-Object DisplayName,InstallLocation,DisplayIcon); $processes=@(Get-CimInstance Win32_Process | Where-Object ExecutablePath | Select-Object Name,ExecutablePath); ConvertTo-Json -InputObject @{apps=$apps; processes=$processes} -Depth 3 -Compress"#;

/// Folders this shallow are shared by many apps, e.g. `C:\Program Files`.
const MIN_FOLDER_DEPTH: usize = 2;
const TRUNCATION_MARK: char = '…';

#[derive(Deserialize, Default)]
pub struct RunningApps {
    apps: Vec<InstalledApp>,
    processes: Vec<RunningProcess>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct InstalledApp {
    display_name: String,
    install_location: Option<String>,
    display_icon: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RunningProcess {
    name: String,
    executable_path: String,
}

impl RunningApps {
    pub fn query() -> AppResult<Self> {
        let output = process::run(
            "powershell.exe",
            &["-NoProfile", "-NonInteractive", "-Command", QUERY],
            LIST_TIMEOUT,
        )?
        .ensure_success("running app query")?;
        serde_json::from_str(output.stdout.trim())
            .map_err(|error| AppError::Parse(format!("running app query: {error}")))
    }

    /// Process names, e.g. `obs64.exe`, running from the folder of the app called `app_name`.
    pub fn processes_of(&self, app_name: &str) -> Vec<String> {
        let folders: Vec<String> = self
            .apps
            .iter()
            .filter(|app| same_app(&app.display_name, app_name))
            .filter_map(InstalledApp::folder)
            .collect();
        let mut names: Vec<String> = self
            .processes
            .iter()
            .filter(|process| folders.iter().any(|folder| process.runs_from(folder)))
            .map(|process| process.name.clone())
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }
}

/// winget shortens long names in its table output with a trailing ellipsis.
fn same_app(display_name: &str, app_name: &str) -> bool {
    match app_name.strip_suffix(TRUNCATION_MARK) {
        Some(prefix) => display_name.starts_with(prefix),
        None => display_name.eq_ignore_ascii_case(app_name),
    }
}

impl InstalledApp {
    fn folder(&self) -> Option<String> {
        let location = self
            .install_location
            .as_deref()
            .map(clean_path)
            .filter(|location| !location.is_empty())
            .or_else(|| self.display_icon.as_deref().and_then(icon_folder))?;
        specific_folder(&location)
    }
}

/// `DisplayIcon` looks like `"C:\Apps\x.exe",0`; its folder stands in for a missing location.
fn icon_folder(icon: &str) -> Option<String> {
    let path = icon.rsplit_once(',').map_or(icon, |(path, _)| path);
    Path::new(&clean_path(path))
        .parent()
        .map(|parent| parent.to_string_lossy().into_owned())
}

fn clean_path(path: &str) -> String {
    path.trim()
        .trim_matches('"')
        .trim_end_matches('\\')
        .to_owned()
}

fn specific_folder(folder: &str) -> Option<String> {
    let normalized = folder.to_lowercase();
    let depth = normalized
        .split('\\')
        .filter(|part| !part.is_empty())
        .count()
        - 1;
    let shared = normalized.contains("\\windows\\") || normalized.ends_with("\\windows");
    (depth >= MIN_FOLDER_DEPTH && !shared).then_some(normalized)
}

impl RunningProcess {
    fn runs_from(&self, folder: &str) -> bool {
        self.executable_path
            .to_lowercase()
            .strip_prefix(folder)
            .is_some_and(|rest| rest.starts_with('\\'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apps(json: &str) -> RunningApps {
        serde_json::from_str(json).unwrap()
    }

    const GIT_RUNNING: &str = r#"{"apps":[{"DisplayName":"Git","InstallLocation":"C:\\Program Files\\Git\\","DisplayIcon":null}],
        "processes":[{"Name":"bash.exe","ExecutablePath":"C:\\Program Files\\Git\\usr\\bin\\bash.exe"},
                     {"Name":"bash.exe","ExecutablePath":"C:\\Program Files\\Git\\bin\\bash.exe"},
                     {"Name":"GitHubDesktop.exe","ExecutablePath":"C:\\Program Files\\GitHub Desktop\\GitHubDesktop.exe"}]}"#;

    #[test]
    fn finds_processes_below_the_install_location() {
        assert_eq!(apps(GIT_RUNNING).processes_of("Git"), vec!["bash.exe"]);
    }

    #[test]
    fn sibling_folders_with_the_same_prefix_do_not_match() {
        assert!(apps(GIT_RUNNING).processes_of("GitHub Desktop").is_empty());
    }

    #[test]
    fn falls_back_to_the_display_icon_folder() {
        let running = apps(
            r#"{"apps":[{"DisplayName":"OBS Studio","InstallLocation":"","DisplayIcon":"\"C:\\Program Files\\obs-studio\\bin\\64bit\\obs64.exe\",0"}],
                "processes":[{"Name":"obs64.exe","ExecutablePath":"C:\\Program Files\\obs-studio\\bin\\64bit\\obs64.exe"}]}"#,
        );
        assert_eq!(running.processes_of("OBS Studio"), vec!["obs64.exe"]);
    }

    #[test]
    fn shared_folders_are_never_matched() {
        let running = apps(
            r#"{"apps":[{"DisplayName":"Tool","InstallLocation":"C:\\Program Files","DisplayIcon":null}],
                "processes":[{"Name":"other.exe","ExecutablePath":"C:\\Program Files\\Other\\other.exe"}]}"#,
        );
        assert!(running.processes_of("Tool").is_empty());
    }

    #[test]
    fn truncated_names_match_by_prefix() {
        assert!(same_app(
            "Microsoft Visual Studio Code",
            "Microsoft Visual Stu…"
        ));
        assert!(!same_app("Microsoft Edge", "Microsoft Visual Stu…"));
    }
}
