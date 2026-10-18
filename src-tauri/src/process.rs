use std::io::Read;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::{AppError, AppResult};

pub const LIST_TIMEOUT: Duration = Duration::from_secs(120);
pub const UPDATE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const POLL_INTERVAL: Duration = Duration::from_millis(25);
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

type OutputReader = Receiver<std::io::Result<Vec<u8>>>;

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

pub fn detect_version(program: &str, args: &[&str]) -> Option<String> {
    let output = run(program, args, LIST_TIMEOUT).ok()?;
    if !output.success {
        return None;
    }
    output
        .stdout
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_owned())
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
    Err(AppError::Command(format!(
        "Ungültige Paket-ID: {package_id:?}"
    )))
}

pub fn run(program: &str, args: &[&str], timeout: Duration) -> AppResult<CommandOutput> {
    let context = format!("{program} {}", args.join(" "));
    let mut command = command(program, args)?;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| command_error(&context, error))?;
    let started = Instant::now();
    let stdout = read_in_background(child.stdout.take().expect("stdout was piped"));
    let stderr = read_in_background(child.stderr.take().expect("stderr was piped"));
    let status = wait_for_exit(&mut child, timeout, &context)?;
    Ok(CommandOutput {
        success: status.success(),
        exit_code: status.code(),
        stdout: collect_output(stdout, timeout.saturating_sub(started.elapsed()), &context)?,
        stderr: collect_output(stderr, timeout.saturating_sub(started.elapsed()), &context)?,
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
    let quoted = std::iter::once(shim)
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
fn quote_cmd_argument(argument: &str) -> AppResult<String> {
    if argument.contains(['"', '&', '|', '<', '>', '^', '%', '!', '\r', '\n', '\0']) {
        return Err(AppError::Command(format!(
            "Unsicheres Argument für cmd.exe: {argument:?}"
        )));
    }
    Ok(format!("\"{argument}\""))
}

fn read_in_background(mut stream: impl Read + Send + 'static) -> OutputReader {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stream.read_to_end(&mut bytes).map(|_| bytes);
        let _ = sender.send(result);
    });
    receiver
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
    let bytes = reader
        .recv_timeout(remaining)
        .map_err(|error| match error {
            RecvTimeoutError::Timeout => {
                AppError::Timeout(format!("{context}: Ausgabe nicht abgeschlossen"))
            }
            RecvTimeoutError::Disconnected => {
                AppError::Command(format!("{context}: Ausgabe-Thread abgebrochen"))
            }
        })?
        .map_err(|error| command_error(context, error))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn command_error(context: &str, error: impl std::fmt::Display) -> AppError {
    AppError::Command(format!("{context}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn missing_executable_has_command_context() {
        let result = run("winget-updater-nonexistent-test-command", &[], LIST_TIMEOUT);
        assert!(
            matches!(result, Err(AppError::Command(message)) if message.contains("winget-updater-nonexistent-test-command"))
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
