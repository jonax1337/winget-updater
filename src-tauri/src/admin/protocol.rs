use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::{AppError, AppResult};
use crate::failure::UpdateFailure;
use crate::models::{ManagerId, PackageRef};
use crate::{catalog, managers, process};

const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;
const MAX_PACKAGE_ID_LENGTH: usize = 256;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    Upgrade,
    Uninstall,
    Install,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub action: Action,
    pub manager: ManagerId,
    pub id: String,
}

pub type Response = Result<(), UpdateFailure>;

pub fn needs_admin(manager: ManagerId) -> bool {
    matches!(manager, ManagerId::Winget | ManagerId::Chocolatey)
}

impl Request {
    pub fn validate(&self) -> AppResult<()> {
        if !needs_admin(self.manager)
            || self.id.len() > MAX_PACKAGE_ID_LENGTH
            || matches!(self.action, Action::Install) && self.manager != ManagerId::Winget
        {
            return Err(AppError::Other(
                "Unsupported administrator operation".into(),
            ));
        }
        match (&self.action, self.manager) {
            (Action::Uninstall, ManagerId::Winget) => {
                managers::winget::validate_installed_id(&self.id)
            }
            _ => process::validate_package_id(&self.id),
        }
    }

    pub fn execute(&self) -> AppResult<()> {
        self.validate()?;
        let package = PackageRef {
            manager: self.manager,
            id: self.id.clone(),
        };
        match self.action {
            Action::Upgrade => managers::upgrade(&package),
            Action::Uninstall => managers::available(&package)?.uninstall(&self.id),
            Action::Install => catalog::install(&self.id),
        }
    }
}

pub async fn write_frame<T: Serialize>(
    stream: &mut (impl AsyncWrite + Unpin),
    value: &T,
) -> AppResult<()> {
    let bytes = serde_json::to_vec(value).map_err(ipc_error)?;
    validate_frame_length(bytes.len())?;
    stream
        .write_u32_le(bytes.len() as u32)
        .await
        .map_err(ipc_error)?;
    stream.write_all(&bytes).await.map_err(ipc_error)?;
    stream.flush().await.map_err(ipc_error)
}

pub async fn read_frame<T: DeserializeOwned>(
    stream: &mut (impl AsyncRead + Unpin),
) -> AppResult<T> {
    let length = stream.read_u32_le().await.map_err(ipc_error)? as usize;
    validate_frame_length(length)?;
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).await.map_err(ipc_error)?;
    serde_json::from_slice(&bytes).map_err(ipc_error)
}

fn validate_frame_length(length: usize) -> AppResult<()> {
    if length == 0 || length > MAX_FRAME_BYTES {
        return Err(ipc_error("Invalid message size"));
    }
    Ok(())
}

pub fn ipc_error(error: impl std::fmt::Display) -> AppError {
    AppError::Other(format!("Administrator session: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(manager: ManagerId, id: &str) -> Request {
        Request {
            action: Action::Upgrade,
            manager,
            id: id.into(),
        }
    }

    #[test]
    fn user_package_managers_never_run_elevated() {
        for manager in [ManagerId::Npm, ManagerId::Pip, ManagerId::Scoop] {
            assert!(!needs_admin(manager));
            assert!(request(manager, "package").validate().is_err());
        }
    }

    #[test]
    fn system_upgrades_are_allowed() {
        assert!(
            request(ManagerId::Winget, "Microsoft.WSL")
                .validate()
                .is_ok()
        );
        assert!(request(ManagerId::Chocolatey, "git").validate().is_ok());
    }

    #[test]
    fn shell_expressions_and_options_are_rejected() {
        for id in ["", "--override", "x & calc", "x\ncalc", "%PATH%", "x\"y"] {
            assert!(request(ManagerId::Winget, id).validate().is_err());
        }
    }

    #[test]
    fn arbitrary_commands_and_extra_fields_are_not_part_of_the_protocol() {
        assert!(
            serde_json::from_str::<Request>(r#"{"action":"shell","manager":"winget","id":"cmd"}"#)
                .is_err()
        );
        assert!(serde_json::from_str::<Request>(r#"{"action":"upgrade","manager":"winget","id":"Microsoft.WSL","args":["--override"]}"#).is_err());
    }

    #[tokio::test]
    async fn requests_round_trip_without_executing_them() {
        let (mut writer, mut reader) = tokio::io::duplex(4096);
        write_frame(&mut writer, &request(ManagerId::Winget, "Microsoft.WSL"))
            .await
            .unwrap();
        let decoded: Request = read_frame(&mut reader).await.unwrap();
        assert_eq!(decoded.id, "Microsoft.WSL");
    }

    #[tokio::test]
    async fn oversized_frames_are_rejected_before_allocation() {
        let (mut writer, mut reader) = tokio::io::duplex(64);
        writer
            .write_u32_le(MAX_FRAME_BYTES as u32 + 1)
            .await
            .unwrap();
        assert!(read_frame::<Request>(&mut reader).await.is_err());
    }

    #[test]
    fn chocolatey_catalog_installs_are_rejected() {
        let request = Request {
            action: Action::Install,
            ..request(ManagerId::Chocolatey, "git")
        };
        assert!(request.validate().is_err());
    }
}
