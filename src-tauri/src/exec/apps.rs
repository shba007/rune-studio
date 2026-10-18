//! Launching applications by name.

use super::ExecError;

/// Opens an application, file, folder or web address the way Win+R would, but without a shell:
/// the name is one argument to `ShellExecute`, so nothing in it can chain a second command.
#[cfg(windows)]
pub fn open(name: &str) -> Result<(), ExecError> {
    use windows::{
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
        core::{HSTRING, PCWSTR},
    };

    let name = name.trim();
    if name.is_empty() {
        return Err(ExecError::Input("no application name given".into()));
    }
    // SAFETY: all strings outlive the call; a null parameters/directory pointer is allowed.
    let result = unsafe {
        ShellExecuteW(None, &HSTRING::from("open"), &HSTRING::from(name), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL)
    };
    // ShellExecute reports failure with a value of 32 or less.
    if result.0 as isize <= 32 {
        return Err(ExecError::Input(format!("could not open \"{name}\" (error {})", result.0 as isize)));
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn open(_name: &str) -> Result<(), ExecError> {
    Err(ExecError::Input("opening applications is only supported on Windows".into()))
}
