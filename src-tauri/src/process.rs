use std::io::Read;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::{AppError, AppResult};

pub const LIST_TIMEOUT: Duration = Duration::from_secs(120);
pub const UPDATE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
pub const UNINSTALL_TIMEOUT: Duration = UPDATE_TIMEOUT;
const POLL_INTERVAL: Duration = Duration::from_millis(25);
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const OUTPUT_GRACE: Duration = Duration::from_secs(5);

struct OutputReader {
    finished: Receiver<std::io::Result<()>>,
    bytes: Arc<Mutex<Vec<u8>>>,
}

#[derive(Debug)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

impl CommandOutput {
    pub fn ensure_success(self, command: &str) -> AppResult<Self> {
        if self.success {
            return Ok(self);
        }
        Err(AppError::Command(format!(
            "{command} (Exit-Code {:?}): {}",
            self.exit_code,
            self.diagnostic()
        )))
    }

    pub fn diagnostic(&self) -> &str {
        if self.stderr.trim().is_empty() {
            self.stdout.trim()
        } else {
            self.stderr.trim()
        }
    }
}

pub fn detect_version(program: &str, args: &[&str]) -> AppResult<Option<String>> {
    version_from_output(run(program, args, LIST_TIMEOUT), program)
}

fn version_from_output(
    result: AppResult<CommandOutput>,
    program: &str,
) -> AppResult<Option<String>> {
    let output = match result {
        Err(AppError::ManagerUnavailable(_)) => return Ok(None),
        result => result?,
    }
    .ensure_success(&format!("{program} --version"))?;
    let version = output
        .stdout
        .lines()
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| AppError::Parse(format!("{program} --version: Versionsausgabe fehlt")))?;
    Ok(Some(version.trim().to_owned()))
}

pub fn validate_package_id(package_id: &str) -> AppResult<()> {
    let valid = !package_id.is_empty()
        && !package_id.starts_with('-')
        && package_id.chars().any(char::is_alphanumeric)
        && package_id
            .chars()
            .all(|character| character.is_alphanumeric() || "._-/@+".contains(character));
    if valid {
        return Ok(());
    }
    Err(invalid_package_id(package_id))
}

pub fn invalid_package_id(package_id: &str) -> AppError {
    AppError::Command(format!("Ungültige Paket-ID: {package_id:?}"))
}

pub fn run(program: &str, args: &[&str], timeout: Duration) -> AppResult<CommandOutput> {
    let context = format!("{program} {}", args.join(" "));
    let mut command = command(program, args)?;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                AppError::ManagerUnavailable(program.into())
            } else {
                command_error(&context, error)
            }
        })?;
    let stdout = read_in_background(child.stdout.take().expect("stdout was piped"));
    let stderr = read_in_background(child.stderr.take().expect("stderr was piped"));
    let status = wait_for_exit(&mut child, timeout, &context)?;
    let output_deadline = Instant::now() + OUTPUT_GRACE;
    Ok(CommandOutput {
        success: status.success(),
        exit_code: status.code(),
        stdout: collect_output(
            stdout,
            output_deadline.saturating_duration_since(Instant::now()),
            &context,
        )?,
        stderr: collect_output(
            stderr,
            output_deadline.saturating_duration_since(Instant::now()),
            &context,
        )?,
    })
}

fn command(program: &str, args: &[&str]) -> AppResult<Command> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut command = windows_command(program, args)?;
        command.creation_flags(CREATE_NO_WINDOW);
        Ok(command)
    }
    #[cfg(not(windows))]
    {
        let mut command = Command::new(program);
        command.args(args);
        Ok(command)
    }
}

#[cfg(windows)]
fn windows_command(program: &str, args: &[&str]) -> AppResult<Command> {
    use std::os::windows::process::CommandExt;
    let shim = match program {
        "npm" => "npm.cmd",
        "scoop" => "scoop.cmd",
        _ => program,
    };
    if !shim.ends_with(".cmd") && !shim.ends_with(".bat") {
        let mut command = Command::new(program);
        command.args(args);
        return Ok(command);
    }
    let shim_path = find_shim(shim)?;
    let shim = shim_path.to_string_lossy();
    let quoted = std::iter::once(shim.as_ref())
        .chain(args.iter().copied())
        .map(quote_cmd_argument)
        .collect::<AppResult<Vec<_>>>()?
        .join(" ");
    let mut command = Command::new("cmd.exe");
    // raw_arg preserves cmd's outer quotes; Command::arg uses incompatible CRT escaping.
    command.raw_arg(format!("/D /S /C \"{quoted}\""));
    Ok(command)
}

#[cfg(windows)]
fn find_shim(shim: &str) -> AppResult<std::path::PathBuf> {
    let path = std::path::Path::new(shim);
    let candidates = if path.components().count() > 1 || path.is_absolute() {
        vec![path.to_owned()]
    } else {
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|directory| directory.join(shim))
            .collect()
    };
    for candidate in candidates {
        match std::fs::symlink_metadata(&candidate) {
            Ok(metadata) if !metadata.is_dir() => return Ok(candidate),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(command_error(shim, error)),
        }
    }
    Err(AppError::ManagerUnavailable(shim.into()))
}

#[cfg(windows)]
fn quote_cmd_argument(argument: &str) -> AppResult<String> {
    if argument.contains(['"', '&', '|', '<', '>', '^', '%', '!', '\r', '\n', '\0']) {
        return Err(AppError::Command(format!(
            "Unsicheres Argument für cmd.exe: {argument:?}"
        )));
    }
    Ok(format!("\"{argument}\""))
}

fn read_in_background(mut stream: impl Read + Send + 'static) -> OutputReader {
    let (sender, finished) = mpsc::channel();
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let buffer = Arc::clone(&bytes);
    thread::spawn(move || {
        let result = read_chunks(&mut stream, &buffer);
        let _ = sender.send(result);
    });
    OutputReader { finished, bytes }
}

fn read_chunks(stream: &mut impl Read, buffer: &Mutex<Vec<u8>>) -> std::io::Result<()> {
    let mut chunk = [0; 8192];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => return Ok(()),
            Ok(count) => buffer
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .extend_from_slice(&chunk[..count]),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
}

fn wait_for_exit(child: &mut Child, timeout: Duration, context: &str) -> AppResult<ExitStatus> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {}
            Err(error) => {
                terminate(child);
                return Err(command_error(context, error));
            }
        }
        if started.elapsed() >= timeout {
            terminate(child);
            return Err(AppError::Timeout(format!(
                "{context} (nach {:.2} Sekunden)",
                timeout.as_secs_f64()
            )));
        }
        thread::sleep(POLL_INTERVAL.min(timeout.saturating_sub(started.elapsed())));
    }
}

fn terminate(child: &mut Child) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // cmd shims launch descendants which otherwise survive the timeout with open pipes.
        let _ = Command::new("taskkill.exe")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn collect_output(reader: OutputReader, remaining: Duration, context: &str) -> AppResult<String> {
    match reader.finished.recv_timeout(remaining) {
        Ok(result) => result.map_err(|error| command_error(context, error))?,
        Err(RecvTimeoutError::Timeout) => {}
        Err(RecvTimeoutError::Disconnected) => {
            return Err(AppError::Command(format!(
                "{context}: Ausgabe-Thread abgebrochen"
            )));
        }
    }
    let bytes = reader
        .bytes
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn command_error(context: &str, error: impl std::fmt::Display) -> AppError {
    AppError::Command(format!("{context}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_reports_missing_executable_as_absent() {
        assert!(
            detect_version("winget-updater-nonexistent-test-command", &[])
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn detection_preserves_timeouts() {
        #[cfg(windows)]
        let output = run(
            "powershell.exe",
            &["-NoProfile", "-Command", "Start-Sleep 10"],
            Duration::from_millis(100),
        );
        #[cfg(not(windows))]
        let output = run("sh", &["-c", "sleep 10"], Duration::from_millis(100));
        assert!(matches!(
            version_from_output(output, "test"),
            Err(AppError::Timeout(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn missing_shim_is_absent_before_cmd_starts() {
        assert!(
            detect_version("winget-updater-nonexistent-shim.cmd", &[])
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn detection_rejects_unsuccessful_exit() {
        #[cfg(windows)]
        let output = run("cmd.exe", &["/D", "/C", "exit 3"], LIST_TIMEOUT);
        #[cfg(not(windows))]
        let output = run("sh", &["-c", "exit 3"], LIST_TIMEOUT);
        assert!(
            matches!(version_from_output(output, "test"), Err(AppError::Command(message)) if message.contains("3"))
        );
    }

    #[cfg(windows)]
    #[test]
    fn inherited_pipes_do_not_delay_successful_child() {
        let started = Instant::now();
        let output = run(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/inherited_pipes.cmd"
            ),
            &[],
            Duration::from_secs(8),
        )
        .unwrap();
        assert!(output.success);
        assert!(output.stdout.contains("retained"));
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn captures_stdout_stderr_and_failure_status() {
        #[cfg(windows)]
        let output = run(
            "cmd.exe",
            &["/D", "/C", "echo output & echo problem 1>&2 & exit /B 7"],
            LIST_TIMEOUT,
        )
        .unwrap();
        #[cfg(not(windows))]
        let output = run(
            "sh",
            &["-c", "echo output; echo problem >&2; exit 7"],
            LIST_TIMEOUT,
        )
        .unwrap();
        assert!(!output.success);
        assert_eq!(output.exit_code, Some(7));
        assert_eq!(output.stdout.trim(), "output");
        assert_eq!(output.stderr.trim(), "problem");
    }

    #[test]
    fn missing_executable_is_unavailable() {
        let result = run("winget-updater-nonexistent-test-command", &[], LIST_TIMEOUT);
        assert!(
            matches!(result, Err(AppError::ManagerUnavailable(message)) if message.contains("winget-updater-nonexistent-test-command"))
        );
    }

    #[test]
    fn timeout_has_command_context() {
        #[cfg(windows)]
        let result = run(
            "powershell.exe",
            &["-NoProfile", "-Command", "Start-Sleep -Seconds 10"],
            Duration::from_millis(100),
        );
        #[cfg(not(windows))]
        let result = run("sh", &["-c", "sleep 10"], Duration::from_millis(100));
        assert!(matches!(result, Err(AppError::Timeout(message)) if message.contains("10")));
    }

    #[test]
    fn decodes_invalid_utf8_lossily() {
        let reader = read_in_background(std::io::Cursor::new(vec![b'A', 0xff, b'B']));
        assert_eq!(
            collect_output(reader, LIST_TIMEOUT, "test").unwrap(),
            "A\u{fffd}B"
        );
    }

    #[cfg(windows)]
    #[test]
    fn cmd_arguments_reject_shell_expansion() {
        for value in ["x&calc", "%PATH%", "!name!", "x\"y", "x\ny"] {
            assert!(quote_cmd_argument(value).is_err());
        }
        assert_eq!(
            quote_cmd_argument("@scope/package@latest").unwrap(),
            "\"@scope/package@latest\""
        );
    }
}
