//! Which window has the keyboard right now, so the rules can tell a Delete in File Explorer from a
//! Delete in Notepad.

use super::rules::Foreground;

/// The foreground window's title and program. Empty when it cannot be read.
#[cfg(windows)]
pub fn current() -> Foreground {
    use windows::{
        Win32::{
            Foundation::CloseHandle,
            System::Threading::{
                OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
                QueryFullProcessImageNameW,
            },
            UI::WindowsAndMessaging::{
                GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
            },
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
            if QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_FORMAT(0),
                PWSTR(path.as_mut_ptr()),
                &mut size,
            )
            .is_ok()
            {
                let full = String::from_utf16_lossy(&path[..size as usize]).to_lowercase();
                process = full
                    .rsplit(['\\', '/'])
                    .next()
                    .unwrap_or("")
                    .trim_end_matches(".exe")
                    .to_string();
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

/// A window handle as a plain number, so the decision about it needs no Win32 types.
pub type WindowId = isize;

#[cfg_attr(not(windows), allow(dead_code))]
/// What to do once the user has answered the question about an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restore {
    /// The window that had the keyboard still has it, or it was Rune itself: nothing to do.
    Nothing,
    /// Answering made another window (Rune's pill) the foreground one: give the keyboard back.
    Refocus(WindowId),
    /// The window the action was meant for is gone: the action must not run against whatever is in front.
    Gone,
}

/// Decides what to do after an answer. `remembered` is the foreground window when the question
/// appeared (`None` when it could not be read), `own` whether it belonged to Rune Studio itself,
/// `now` the foreground window now and `alive` whether the remembered window still exists.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn plan_restore(
    remembered: Option<WindowId>,
    own: bool,
    now: Option<WindowId>,
    alive: bool,
) -> Restore {
    match remembered {
        None => Restore::Nothing,
        Some(_) if own => Restore::Nothing,
        Some(window) if now == Some(window) => Restore::Nothing,
        Some(_) if !alive => Restore::Gone,
        Some(window) => Restore::Refocus(window),
    }
}

/// The window that has the keyboard now, if it is not Rune Studio's own (the pill, a card): that
/// is the window an approved action is meant for.
#[cfg(windows)]
pub fn remember() -> Option<WindowId> {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    // SAFETY: plain Win32 calls with valid out pointers.
    unsafe {
        let window = GetForegroundWindow();
        if window.is_invalid() {
            return None;
        }
        let mut process_id = 0_u32;
        GetWindowThreadProcessId(window, Some(&mut process_id));
        (process_id != std::process::id()).then_some(window.0 as WindowId)
    }
}

#[cfg(not(windows))]
pub fn remember() -> Option<WindowId> {
    None
}

/// After an approval: makes sure the remembered window has the keyboard again before the action
/// runs. `Err` says why the action must not run (the window is gone, or Windows would not give it
/// back).
#[cfg(windows)]
pub async fn give_back(remembered: Option<WindowId>) -> Result<(), &'static str> {
    use windows::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId, IsWindow, SetForegroundWindow,
        },
    };

    let handle = |id: WindowId| HWND(id as *mut core::ffi::c_void);
    // SAFETY (all blocks below): plain Win32 calls on a handle that is only tested or focused.
    let own = |id: WindowId| unsafe {
        let mut process_id = 0_u32;
        GetWindowThreadProcessId(handle(id), Some(&mut process_id));
        process_id == std::process::id()
    };
    let now = || {
        let window = unsafe { GetForegroundWindow() };
        (!window.is_invalid()).then_some(window.0 as WindowId)
    };
    let alive = |id: WindowId| unsafe { IsWindow(Some(handle(id))).as_bool() };

    let Some(window) = remembered else {
        return Ok(());
    };
    match plan_restore(remembered, own(window), now(), alive(window)) {
        Restore::Nothing => Ok(()),
        Restore::Gone => Err("the window for this action is gone"),
        Restore::Refocus(window) => {
            let _ = unsafe { SetForegroundWindow(handle(window)) };
            // Focus changes are asynchronous: wait for it instead of typing into Rune.
            for _ in 0..20 {
                if now() == Some(window) {
                    return Ok(());
                }
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
            Err("could not return to the window for this action")
        }
    }
}

#[cfg(not(windows))]
pub async fn give_back(_remembered: Option<WindowId>) -> Result<(), &'static str> {
    Ok(())
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
        let read = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut TOKEN_ELEVATION as *mut core::ffi::c_void),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        );
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
            token_is_elevated(unsafe { windows::Win32::System::Threading::GetCurrentProcess() })
                .unwrap_or(false)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_done_when_the_target_still_has_the_keyboard_or_was_never_known() {
        assert_eq!(
            plan_restore(Some(10), false, Some(10), true),
            Restore::Nothing
        );
        assert_eq!(plan_restore(None, false, Some(10), true), Restore::Nothing);
        assert_eq!(
            plan_restore(Some(10), true, Some(99), true),
            Restore::Nothing,
            "Rune's own window is not a target"
        );
    }

    #[test]
    fn the_target_gets_the_keyboard_back_when_answering_took_it() {
        assert_eq!(
            plan_restore(Some(10), false, Some(99), true),
            Restore::Refocus(10)
        );
        assert_eq!(
            plan_restore(Some(10), false, None, true),
            Restore::Refocus(10)
        );
    }

    #[test]
    fn a_target_that_is_gone_stops_the_action() {
        assert_eq!(
            plan_restore(Some(10), false, Some(99), false),
            Restore::Gone
        );
        assert_eq!(plan_restore(Some(10), false, None, false), Restore::Gone);
    }
}
