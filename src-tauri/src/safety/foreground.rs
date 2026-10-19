//! Which window has the keyboard right now, so the rules can tell a Delete in File Explorer from a
//! Delete in Notepad.

use super::rules::Foreground;

/// The foreground window's title and program. Empty when it cannot be read.
#[cfg(windows)]
pub fn current() -> Foreground {
    use windows::{
        Win32::{
            Foundation::CloseHandle,
            System::Threading::{OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW},
            UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId},
        },
        core::PWSTR,
    };

    // SAFETY: plain Win32 calls; every buffer is valid for the length passed and the process
    // handle is closed again.
    unsafe {
        let window = GetForegroundWindow();
        if window.is_invalid() {
            return Foreground::default();
        }
        let mut title = [0_u16; 512];
        let length = GetWindowTextW(window, &mut title).max(0) as usize;
        let title = String::from_utf16_lossy(&title[..length]);

        let mut process_id = 0_u32;
        GetWindowThreadProcessId(window, Some(&mut process_id));
        let mut process = String::new();
        if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) {
            let mut path = [0_u16; 520];
            let mut size = path.len() as u32;
            if QueryFullProcessImageNameW(handle, PROCESS_NAME_FORMAT(0), PWSTR(path.as_mut_ptr()), &mut size).is_ok() {
                let full = String::from_utf16_lossy(&path[..size as usize]).to_lowercase();
                process = full.rsplit(['\\', '/']).next().unwrap_or("").trim_end_matches(".exe").to_string();
            }
            let _ = CloseHandle(handle);
        }
        Foreground { title, process }
    }
}

#[cfg(not(windows))]
pub fn current() -> Foreground {
    Foreground::default()
}

/// What the model and outside agents are told when clicks and typing cannot reach the front window.
pub const ELEVATED_WARNING: &str = "The front window runs as administrator: clicks and typing are ignored. Use ask_user or finish with done and success=false.";

/// Whether the process behind `process` runs with an elevated token. `None` when it cannot be read.
#[cfg(windows)]
fn token_is_elevated(process: windows::Win32::Foundation::HANDLE) -> Option<bool> {
    use windows::Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation},
        System::Threading::OpenProcessToken,
    };

    // SAFETY: plain Win32 calls; the output buffer is a TOKEN_ELEVATION and its size is passed,
    // and the token handle is closed again on every path.
    unsafe {
        let mut token = HANDLE::default();
        if let Err(err) = OpenProcessToken(process, TOKEN_QUERY, &mut token) {
            // Reading the token of a process that runs at a higher level can be refused outright.
            log::debug!("could not open the process token: {err}");
            return None;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut returned = 0_u32;
        let read = GetTokenInformation(token, TokenElevation, Some(&mut elevation as *mut TOKEN_ELEVATION as *mut core::ffi::c_void), std::mem::size_of::<TOKEN_ELEVATION>() as u32, &mut returned);
        let _ = CloseHandle(token);
        read.ok().map(|()| elevation.TokenIsElevated != 0)
    }
}

/// Whether the front window's program runs as administrator. `None` when that cannot be read (and
/// everywhere but Windows).
#[cfg(windows)]
pub fn is_elevated() -> Option<bool> {
    use windows::Win32::{
        Foundation::CloseHandle,
        System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    };

    // SAFETY: plain Win32 calls; the process handle is closed again.
    unsafe {
        let window = GetForegroundWindow();
        if window.is_invalid() {
            return None;
        }
        let mut process_id = 0_u32;
        GetWindowThreadProcessId(window, Some(&mut process_id));
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()?;
        let elevated = token_is_elevated(handle);
        let _ = CloseHandle(handle);
        elevated
    }
}

#[cfg(not(windows))]
pub fn is_elevated() -> Option<bool> {
    None
}

/// Whether Rune Studio itself runs as administrator, worked out once.
fn app_is_elevated() -> bool {
    static ELEVATED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ELEVATED.get_or_init(|| {
        #[cfg(windows)]
        {
            // SAFETY: GetCurrentProcess returns a pseudo handle that needs no closing.
            token_is_elevated(unsafe { windows::Win32::System::Threading::GetCurrentProcess() }).unwrap_or(false)
        }
        #[cfg(not(windows))]
        {
            false
        }
    })
}

/// The front window runs as administrator and we do not, so Windows drops our clicks and typing.
pub fn blocked_by_elevation() -> bool {
    is_elevated() == Some(true) && !app_is_elevated()
}
