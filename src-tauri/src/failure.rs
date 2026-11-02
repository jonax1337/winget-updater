//! Turns the raw output of a failed upgrade into a reason the UI can explain.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::process::CommandOutput;

pub const WINGET_UPDATE_NOT_APPLICABLE: u32 = 0x8A15_002B;
const WINGET_REBOOT_REQUIRED_TO_FINISH: u32 = 0x8A15_0109;
const CHOCOLATEY_REBOOT_CODES: [i32; 2] = [1641, 3010];

const IN_USE_CODES: [u32; 3] = [0x8A15_0101, 0x8A15_0103, 0x8A15_0111];
const INSTALLER_BUSY_CODES: [u32; 2] = [0x8A15_0102, 1618];
const NEEDS_ADMIN_CODES: [u32; 4] = [0x8A15_0019, 0x8007_3D28, 0x8007_02E4, 740];
const APPROVAL_DECLINED_CODES: [u32; 3] = [0x8A15_010C, 0x8007_04C7, 1223];
const CHECKSUM_CODES: [u32; 1] = [0x8A15_0011];
const NO_NETWORK_CODES: [u32; 5] = [
    0x8A15_0008,
    0x8A15_0107,
    0x8007_2EE7,
    0x8007_2EFD,
    0x8007_2EE2,
];
const REBOOT_REQUIRED_CODES: [u32; 1] = [0x8A15_010A];

const IN_USE_TEXT: [&str; 6] = [
    "ebusy",
    "resource busy or locked",
    "[winerror 32]",
    "being used by another process",
    "are still running",
    "is still running",
];
const NEEDS_ADMIN_TEXT: [&str; 2] = ["not running from an elevated", "requires administrator"];
const NO_NETWORK_TEXT: [&str; 4] = ["enotfound", "etimedout", "econnreset", "newconnectionerror"];

/// Installers report their own exit code inside winget's localized output, e.g.
/// "Installer failed with exit code: 1223" or "Installation fehlgeschlagen mit Exitcode: 6".
static INSTALLER_EXIT_CODE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)exit\s*code:\s*(0x[0-9a-f]+|-?\d+)").expect("valid regex"));

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum FailureReason {
    InUse,
    InstallerBusy,
    NeedsAdmin,
    ApprovalDeclined,
    ChecksumMismatch,
    NoApplicableUpgrade,
    NoNetwork,
    RebootRequired,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFailure {
    pub reason: FailureReason,
    /// Raw command output for the details view.
    pub details: String,
}

impl UpdateFailure {
    pub fn new(reason: FailureReason, details: impl Into<String>) -> Self {
        Self {
            reason,
            details: details.into(),
        }
    }
}

impl From<AppError> for UpdateFailure {
    fn from(error: AppError) -> Self {
        match error {
            AppError::Update(failure) => failure,
            other => Self::new(FailureReason::Unknown, other.to_string()),
        }
    }
}

/// Accepts a finished upgrade, treating "restart required to finish" as success.
pub fn check_upgrade(output: CommandOutput, command: &str) -> AppResult<()> {
    if output.success || finished_pending_reboot(&output) {
        return Ok(());
    }
    let details = format!(
        "{command} (exit code {}): {}",
        exit_code_label(output.exit_code),
        output.diagnostic()
    );
    Err(AppError::Update(UpdateFailure::new(
        diagnose(&output),
        details,
    )))
}

fn finished_pending_reboot(output: &CommandOutput) -> bool {
    output.exit_code.is_some_and(|code| {
        code as u32 == WINGET_REBOOT_REQUIRED_TO_FINISH || CHOCOLATEY_REBOOT_CODES.contains(&code)
    })
}

fn exit_code_label(code: Option<i32>) -> String {
    match code {
        Some(code) if code < 0 => format!("{:#010X}", code as u32),
        Some(code) => code.to_string(),
        None => "unknown".into(),
    }
}

pub fn diagnose(output: &CommandOutput) -> FailureReason {
    let text = format!("{}\n{}", output.stdout, output.stderr).to_lowercase();
    let codes: Vec<u32> = output
        .exit_code
        .map(|code| code as u32)
        .into_iter()
        .chain(installer_exit_codes(&text))
        .collect();
    reason_for_codes(&codes).unwrap_or_else(|| reason_for_text(&text))
}

fn installer_exit_codes(text: &str) -> impl Iterator<Item = u32> + '_ {
    INSTALLER_EXIT_CODE
        .captures_iter(text)
        .filter_map(|captures| parse_code(&captures[1]))
}

fn parse_code(value: &str) -> Option<u32> {
    match value.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => value.parse::<i64>().ok().map(|code| code as u32),
    }
}

fn reason_for_codes(codes: &[u32]) -> Option<FailureReason> {
    let table: [(&[u32], FailureReason); 8] = [
        (&IN_USE_CODES, FailureReason::InUse),
        (&INSTALLER_BUSY_CODES, FailureReason::InstallerBusy),
        (&NEEDS_ADMIN_CODES, FailureReason::NeedsAdmin),
        (&APPROVAL_DECLINED_CODES, FailureReason::ApprovalDeclined),
        (&CHECKSUM_CODES, FailureReason::ChecksumMismatch),
        (
            &[WINGET_UPDATE_NOT_APPLICABLE],
            FailureReason::NoApplicableUpgrade,
        ),
        (&NO_NETWORK_CODES, FailureReason::NoNetwork),
        (&REBOOT_REQUIRED_CODES, FailureReason::RebootRequired),
    ];
    table
        .iter()
        .find(|(known, _)| codes.iter().any(|code| known.contains(code)))
        .map(|(_, reason)| *reason)
}

fn reason_for_text(text: &str) -> FailureReason {
    let contains_any = |needles: &[&str]| needles.iter().any(|needle| text.contains(needle));
    if contains_any(&IN_USE_TEXT) {
        FailureReason::InUse
    } else if contains_any(&NEEDS_ADMIN_TEXT) {
        FailureReason::NeedsAdmin
    } else if contains_any(&NO_NETWORK_TEXT) {
        FailureReason::NoNetwork
    } else {
        FailureReason::Unknown
    }
}

/// A sentence for people, naming the processes that hold the app's files when they are known.
pub fn explain(failure: &UpdateFailure, app_name: &str, running: &[String]) -> String {
    match failure.reason {
        FailureReason::InUse if running.is_empty() => {
            format!("{app_name} or one of its files is in use. Close it and try again.")
        }
        FailureReason::InUse => format!(
            "{app_name} is still running ({}). Close it and try again.",
            running.join(", ")
        ),
        FailureReason::InstallerBusy => {
            "Another installation is in progress. Try again when it has finished.".into()
        }
        FailureReason::NeedsAdmin => "This update needs administrator rights.".into(),
        FailureReason::ApprovalDeclined => {
            "Administrator approval was declined, so the update did not run.".into()
        }
        FailureReason::ChecksumMismatch => {
            "The downloaded installer failed its checksum check. Try again later.".into()
        }
        FailureReason::NoApplicableUpgrade => {
            "No update applies to this system, for example because of its architecture.".into()
        }
        FailureReason::NoNetwork => {
            "The download failed. Check your internet connection and try again.".into()
        }
        FailureReason::RebootRequired => "Restart Windows, then try again.".into(),
        FailureReason::Unknown => failure.details.clone(),
    }
}

/// Installers usually fail for unknown reasons when the app they replace is still running.
pub fn attribute_to_running_app(failure: UpdateFailure, running: &[String]) -> UpdateFailure {
    if failure.reason == FailureReason::Unknown && !running.is_empty() {
        return UpdateFailure::new(FailureReason::InUse, failure.details);
    }
    failure
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failed(exit_code: i32, stdout: &str) -> CommandOutput {
        CommandOutput {
            success: false,
            stdout: stdout.into(),
            stderr: String::new(),
            exit_code: Some(exit_code),
        }
    }

    #[test]
    fn package_in_use_by_application_is_in_use() {
        // Observed for OBS Studio while it was running.
        assert_eq!(diagnose(&failed(-1978334959, "")), FailureReason::InUse);
    }

    #[test]
    fn msix_admin_requirement_in_localized_text_needs_admin() {
        let output = failed(
            -2147009240,
            "Installation fehlgeschlagen mit Exitcode: 0x80073d28 : Administratorrechte",
        );
        assert_eq!(diagnose(&output), FailureReason::NeedsAdmin);
    }

    #[test]
    fn declined_installer_elevation_is_detected_from_installer_exit_code() {
        let output = failed(-1978335226, "Installer failed with exit code: 1223");
        assert_eq!(diagnose(&output), FailureReason::ApprovalDeclined);
    }

    #[test]
    fn generic_installer_failure_stays_unknown() {
        let output = failed(-1978335226, "Installation fehlgeschlagen mit Exitcode: 1");
        assert_eq!(diagnose(&output), FailureReason::Unknown);
    }

    #[test]
    fn npm_lock_is_in_use() {
        let output = failed(-4082, "npm error code EBUSY\nnpm error syscall rename");
        assert_eq!(diagnose(&output), FailureReason::InUse);
    }

    #[test]
    fn hash_mismatch_is_checksum_failure() {
        assert_eq!(
            diagnose(&failed(0x8A15_0011_u32 as i32, "")),
            FailureReason::ChecksumMismatch
        );
    }

    #[test]
    fn reboot_to_finish_counts_as_success() {
        assert!(check_upgrade(failed(0x8A15_0109_u32 as i32, ""), "winget upgrade").is_ok());
        assert!(check_upgrade(failed(3010, ""), "choco upgrade").is_ok());
    }

    #[test]
    fn failure_details_show_hresults_in_hex() {
        let error = check_upgrade(failed(-1978334959, "busy"), "winget upgrade").unwrap_err();
        let failure = UpdateFailure::from(error);
        assert_eq!(
            failure.details,
            "winget upgrade (exit code 0x8A150111): busy"
        );
    }

    #[test]
    fn unknown_failure_with_running_processes_becomes_in_use() {
        let failure = UpdateFailure::new(FailureReason::Unknown, "exit code 1");
        let running = vec!["bash.exe".to_owned()];
        assert_eq!(
            attribute_to_running_app(failure, &running).reason,
            FailureReason::InUse
        );
    }

    #[test]
    fn explanation_names_running_processes() {
        let failure = UpdateFailure::new(FailureReason::InUse, "");
        let running = vec!["obs64.exe".to_owned()];
        assert_eq!(
            explain(&failure, "OBS Studio", &running),
            "OBS Studio is still running (obs64.exe). Close it and try again."
        );
    }
}
