mod native;
mod protocol;

use std::os::windows::io::OwnedHandle;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeServer, ServerOptions};
use tokio::sync::Mutex;
use windows::Win32::Storage::FileSystem::SECURITY_IDENTIFICATION;
use windows::Win32::System::Pipes::{GetNamedPipeClientProcessId, GetNamedPipeServerProcessId};

use crate::error::{AppError, AppResult};
use crate::failure::UpdateFailure;
use crate::process::{LIST_TIMEOUT, UPDATE_TIMEOUT};
pub use protocol::{Action, Request, needs_admin};
use protocol::{Response, ipc_error, read_frame, write_frame};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const REPLY_TIMEOUT: Duration =
    Duration::from_secs(UPDATE_TIMEOUT.as_secs() + LIST_TIMEOUT.as_secs() + 60);
const HELPER_FLAG: &str = "--admin-helper";
static NEXT_PIPE: AtomicU64 = AtomicU64::new(0);

#[derive(Default)]
pub struct AdminSession {
    connection: Mutex<Option<Connection>>,
}

impl AdminSession {
    pub async fn authorized(&self) -> bool {
        self.connection
            .lock()
            .await
            .as_ref()
            .is_some_and(Connection::alive)
    }

    pub async fn authorize(&self) -> AppResult<()> {
        let mut slot = self.connection.lock().await;
        if slot.as_ref().is_some_and(Connection::alive) {
            return Ok(());
        }
        *slot = None;
        *slot = Some(Connection::open().await?);
        Ok(())
    }

    pub async fn revoke(&self) {
        self.connection.lock().await.take();
    }

    pub async fn execute(&self, request: Request) -> AppResult<()> {
        request.validate()?;
        let mut slot = self.connection.lock().await;
        // Never replay an operation after disconnect: an installer may already have run.
        let mut connection = slot
            .take()
            .ok_or_else(|| ipc_error("Administrator access is not active"))?;
        let reply = tokio::time::timeout(REPLY_TIMEOUT, async {
            write_frame(&mut connection.pipe, &request).await?;
            read_frame::<Response>(&mut connection.pipe).await
        })
        .await
        .map_err(|_| {
            ipc_error("No reply from the helper; check the installed version before retrying")
        })??;
        *slot = Some(connection);
        reply.map_err(AppError::Update)
    }
}

struct Connection {
    pipe: NamedPipeServer,
    process: OwnedHandle,
}

impl Connection {
    fn alive(&self) -> bool {
        native::is_running(&self.process)
    }

    async fn open() -> AppResult<Self> {
        let name = pipe_name(
            std::process::id(),
            NEXT_PIPE.fetch_add(1, Ordering::Relaxed),
        );
        // Creating first prevents a pre-existing server from impersonating this app.
        let pipe = ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .max_instances(1)
            .create(&name)
            .map_err(ipc_error)?;
        let process = tokio::task::spawn_blocking(move || native::launch(&name))
            .await
            .map_err(ipc_error)??;
        let mut connection = Self { pipe, process };
        tokio::time::timeout(CONNECT_TIMEOUT, connection.authenticate())
            .await.map_err(|_| ipc_error("The administrator helper did not connect. Approve with the same Windows account."))??;
        Ok(connection)
    }

    async fn authenticate(&mut self) -> AppResult<()> {
        loop {
            self.pipe.connect().await.map_err(ipc_error)?;
            let mut pid = 0;
            unsafe { GetNamedPipeClientProcessId(native::raw(&self.pipe), &mut pid) }
                .map_err(ipc_error)?;
            if pid == native::process_id(&self.process) {
                native::verify_elevated_peer(pid)?;
                return read_frame::<Response>(&mut self.pipe)
                    .await?
                    .map_err(AppError::Update);
            }
            self.pipe.disconnect().map_err(ipc_error)?;
        }
    }
}

fn pipe_name(parent: u32, sequence: u64) -> String {
    format!(r"\\.\pipe\WingetUpdater.Admin.{parent}.{sequence}")
}

/// Called before Tauri's single-instance plugin: the worker has no WebView or app plugins.
pub fn run_helper_if_requested() -> Option<i32> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.first().is_none_or(|arg| arg != HELPER_FLAG) {
        return None;
    }
    let result = helper_arguments(&arguments).and_then(|(pipe, parent)| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(ipc_error)?
            .block_on(serve(&pipe, parent))
    });
    Some(match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    })
}

fn helper_arguments(arguments: &[String]) -> AppResult<(String, u32)> {
    if arguments.len() != 3 {
        return Err(ipc_error("Invalid helper arguments"));
    }
    let parent: u32 = arguments[2].parse().map_err(ipc_error)?;
    let prefix = format!(r"\\.\pipe\WingetUpdater.Admin.{parent}.");
    let sequence = arguments[1]
        .strip_prefix(&prefix)
        .ok_or_else(|| ipc_error("Invalid pipe name"))?;
    sequence.parse::<u64>().map_err(ipc_error)?;
    Ok((arguments[1].clone(), parent))
}

async fn serve(name: &str, parent: u32) -> AppResult<()> {
    if !native::current_elevated()? {
        return Err(ipc_error(
            "The administrator helper must be launched through Windows UAC",
        ));
    }
    let _parent = native::verify_peer(parent)?;
    // Identification-only prevents a pipe server from impersonating this elevated client.
    let mut pipe = ClientOptions::new()
        .security_qos_flags(SECURITY_IDENTIFICATION.0)
        .open(name)
        .map_err(ipc_error)?;
    let mut server_pid = 0;
    unsafe { GetNamedPipeServerProcessId(native::raw(&pipe), &mut server_pid) }
        .map_err(ipc_error)?;
    if server_pid != parent {
        return Err(ipc_error("The pipe does not belong to the launching app"));
    }
    write_frame(&mut pipe, &Response::Ok(())).await?;
    loop {
        let request: Request = match read_frame(&mut pipe).await {
            Ok(request) => request,
            Err(error) => {
                // Closing the app closes the pipe and ends this session, without persistence.
                eprintln!("Administrator connection closed: {error}");
                return Ok(());
            }
        };
        let response: Response = tokio::task::spawn_blocking(move || request.execute())
            .await
            .map_err(ipc_error)?
            .map_err(UpdateFailure::from);
        write_frame(&mut pipe, &response).await?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn connected_session() -> (
        AdminSession,
        tokio::net::windows::named_pipe::NamedPipeClient,
    ) {
        let name = pipe_name(
            std::process::id(),
            NEXT_PIPE.fetch_add(1, Ordering::Relaxed),
        );
        let pipe = ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .create(&name)
            .unwrap();
        let client = ClientOptions::new().open(&name).unwrap();
        pipe.connect().await.unwrap();
        let connection = Connection {
            pipe,
            process: native::verify_peer(std::process::id()).unwrap(),
        };
        (
            AdminSession {
                connection: Mutex::new(Some(connection)),
            },
            client,
        )
    }

    fn upgrade_request() -> Request {
        Request {
            action: Action::Upgrade,
            manager: crate::models::ManagerId::Winget,
            id: "Microsoft.WSL".into(),
        }
    }

    #[tokio::test]
    async fn authorization_and_multiple_actions_reuse_the_existing_connection() {
        let (session, mut client) = connected_session().await;
        let responder = tokio::spawn(async move {
            for _ in 0..2 {
                let _: Request = read_frame(&mut client).await.unwrap();
                write_frame(&mut client, &Response::Ok(())).await.unwrap();
            }
        });
        session.authorize().await.unwrap();
        session.execute(upgrade_request()).await.unwrap();
        session.execute(upgrade_request()).await.unwrap();
        responder.await.unwrap();
    }

    #[tokio::test]
    async fn installer_failure_does_not_discard_the_session() {
        let (session, mut client) = connected_session().await;
        let responder = tokio::spawn(async move {
            let _: Request = read_frame(&mut client).await.unwrap();
            let failure =
                UpdateFailure::new(crate::failure::FailureReason::InUse, "files are locked");
            write_frame(&mut client, &Response::Err(failure))
                .await
                .unwrap();
            let _: Request = read_frame(&mut client).await.unwrap();
            write_frame(&mut client, &Response::Ok(())).await.unwrap();
        });
        assert!(
            matches!(session.execute(upgrade_request()).await, Err(AppError::Update(failure)) if failure.reason == crate::failure::FailureReason::InUse)
        );
        session.execute(upgrade_request()).await.unwrap();
        responder.await.unwrap();
    }

    #[tokio::test]
    async fn disconnected_helper_clears_access_without_replaying_the_operation() {
        let (session, client) = connected_session().await;
        drop(client);
        assert!(session.execute(upgrade_request()).await.is_err());
        assert!(!session.authorized().await);
    }

    #[tokio::test]
    async fn revoking_access_closes_the_channel() {
        let (session, mut client) = connected_session().await;
        session.revoke().await;
        assert!(!session.authorized().await);
        assert!(read_frame::<Request>(&mut client).await.is_err());
    }

    #[test]
    fn helper_only_connects_to_an_app_scoped_local_pipe() {
        let arguments = vec![HELPER_FLAG.into(), pipe_name(123, 4), "123".into()];
        assert_eq!(
            helper_arguments(&arguments).unwrap(),
            (pipe_name(123, 4), 123)
        );
        for pipe in [
            r"\\remote\pipe\anything",
            r"\\.\pipe\Other",
            r"\\.\pipe\WingetUpdater.Admin.456.4",
        ] {
            let arguments = vec![HELPER_FLAG.into(), pipe.into(), "123".into()];
            assert!(helper_arguments(&arguments).is_err());
        }
    }

    #[test]
    fn helper_rejects_extra_arguments_and_invalid_process_ids() {
        assert!(helper_arguments(&[HELPER_FLAG.into()]).is_err());
        assert!(helper_arguments(&[HELPER_FLAG.into(), pipe_name(1, 0), "bad".into()]).is_err());
    }
}
