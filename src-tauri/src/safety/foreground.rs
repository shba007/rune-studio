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
