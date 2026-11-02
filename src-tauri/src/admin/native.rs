use std::ffi::OsStr;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::PathBuf;

use windows::Win32::Foundation::{HANDLE, WAIT_TIMEOUT};
use windows::Win32::Security::{
    EqualSid, GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TOKEN_USER, TokenElevation,
    TokenUser,
};
use windows::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetProcessId, OpenProcess, OpenProcessToken, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, QueryFullProcessImageNameW,
    WaitForSingleObject,
};
use windows::Win32::UI::Shell::{
    SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    ShellExecuteExW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::core::{PCWSTR, PWSTR, w};

use super::protocol::ipc_error;
use crate::error::{AppError, AppResult};
use crate::failure::{FailureReason, UpdateFailure};

pub fn raw(handle: &impl AsRawHandle) -> HANDLE {
    HANDLE(handle.as_raw_handle())
}

fn owned(handle: HANDLE) -> OwnedHandle {
    // The caller transfers a successful Win32 open/launch result, never a pseudo handle.
    unsafe { OwnedHandle::from_raw_handle(handle.0) }
}

fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(Some(0)).collect()
}

pub fn launch(pipe: &str) -> AppResult<OwnedHandle> {
    let executable = wide(std::env::current_exe().map_err(ipc_error)?);
    let arguments = wide(format!("--admin-helper {pipe} {}", std::process::id()));
    let mut directory = vec![0u16; 32768];
    let length = unsafe { GetSystemDirectoryW(Some(&mut directory)) };
    if length == 0 || length as usize >= directory.len() {
        return Err(ipc_error("Could not resolve the Windows system directory"));
    }
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
        lpVerb: w!("runas"),
        lpFile: PCWSTR(executable.as_ptr()),
        lpParameters: PCWSTR(arguments.as_ptr()),
        lpDirectory: PCWSTR(directory.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    // ShellExecute may use COM shell extensions. This blocking task owns the apartment.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) }
        .ok()
        .map_err(ipc_error)?;
    let result = unsafe { ShellExecuteExW(&mut info) };
    unsafe { CoUninitialize() };
    result.map_err(|error| {
        if error.code().0 as u32 == 0x8007_04C7 {
            AppError::Update(UpdateFailure::new(
                FailureReason::ApprovalDeclined,
                "Administrator approval was declined. No system updates were started.",
            ))
        } else {
            ipc_error(error)
        }
    })?;
    if info.hProcess.is_invalid() {
        return Err(ipc_error("Windows did not return the helper process"));
    }
    Ok(owned(info.hProcess))
}

pub fn process_id(process: &OwnedHandle) -> u32 {
    unsafe { GetProcessId(raw(process)) }
}

pub fn is_running(process: &OwnedHandle) -> bool {
    unsafe { WaitForSingleObject(raw(process), 0) == WAIT_TIMEOUT }
}

fn token(process: HANDLE) -> AppResult<OwnedHandle> {
    let mut handle = HANDLE::default();
    unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut handle) }.map_err(ipc_error)?;
    Ok(owned(handle))
}

fn elevated(process: HANDLE) -> AppResult<bool> {
    let token = token(process)?;
    let mut elevation = TOKEN_ELEVATION::default();
    let mut returned = 0;
    unsafe {
        GetTokenInformation(
            raw(&token),
            TokenElevation,
            Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        )
    }
    .map_err(ipc_error)?;
    Ok(elevation.TokenIsElevated != 0)
}

pub fn current_elevated() -> AppResult<bool> {
    elevated(unsafe { GetCurrentProcess() })
}

fn token_user(process: HANDLE) -> AppResult<Vec<usize>> {
    let token = token(process)?;
    let mut length = 0;
    // The first call reports the required buffer size; TOKEN_USER contains an aligned SID pointer.
    let _ = unsafe { GetTokenInformation(raw(&token), TokenUser, None, 0, &mut length) };
    if length == 0 {
        return Err(ipc_error("Could not determine the Windows user"));
    }
    let mut buffer = vec![0usize; (length as usize).div_ceil(std::mem::size_of::<usize>())];
    unsafe {
        GetTokenInformation(
            raw(&token),
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            length,
            &mut length,
        )
    }
    .map_err(ipc_error)?;
    Ok(buffer)
}

fn same_user(process: HANDLE) -> AppResult<()> {
    let current = token_user(unsafe { GetCurrentProcess() })?;
    let peer = token_user(process)?;
    let matches = unsafe {
        let current = &*current.as_ptr().cast::<TOKEN_USER>();
        let peer = &*peer.as_ptr().cast::<TOKEN_USER>();
        EqualSid(current.User.Sid, peer.User.Sid)
    };
    matches.map_err(|_| {
        ipc_error("Approve access with the same Windows account, not a different administrator")
    })
}

fn image_path(process: HANDLE) -> AppResult<PathBuf> {
    let mut buffer = vec![0u16; 32768];
    let mut length = buffer.len() as u32;
    unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    }
    .map_err(ipc_error)?;
    Ok(PathBuf::from(std::ffi::OsString::from_wide(
        &buffer[..length as usize],
    )))
}

/// Retain the process handle to prevent PID reuse while communicating with this peer.
pub fn verify_peer(pid: u32) -> AppResult<OwnedHandle> {
    let process = owned(
        unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                false,
                pid,
            )
        }
        .map_err(ipc_error)?,
    );
    same_user(raw(&process))?;
    let expected = std::env::current_exe().map_err(ipc_error)?;
    if image_path(raw(&process))? != expected {
        return Err(ipc_error("The connected process is not this application"));
    }
    Ok(process)
}

pub fn verify_elevated_peer(pid: u32) -> AppResult<()> {
    let process = verify_peer(pid)?;
    if !elevated(raw(&process))? {
        return Err(ipc_error("The helper does not have administrator rights"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_process_identity_is_accepted_without_requesting_uac() {
        assert!(verify_peer(std::process::id()).is_ok());
        assert!(current_elevated().is_ok());
    }

    #[test]
    fn missing_process_is_rejected() {
        assert!(verify_peer(u32::MAX).is_err());
    }
}
