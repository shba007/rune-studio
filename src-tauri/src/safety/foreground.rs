use super::rules::Foreground;

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

pub type WindowId = isize;

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restore {
    Nothing,
    Refocus(WindowId),
    Gone,
}

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

#[cfg(windows)]
static LAST_TARGET: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

#[cfg_attr(not(windows), allow(dead_code))]
pub fn pick_target(
    front: Option<WindowId>,
    front_is_own: bool,
    last: Option<WindowId>,
) -> Option<WindowId> {
    match front {
        Some(window) if !front_is_own => Some(window),
        _ => last,
    }
}

#[cfg(windows)]
pub fn remember() -> Option<WindowId> {
    use std::sync::atomic::Ordering;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    let (front, own) = unsafe {
        let window = GetForegroundWindow();
        if window.is_invalid() {
            (None, false)
        } else {
            let mut process_id = 0_u32;
            GetWindowThreadProcessId(window, Some(&mut process_id));
            (Some(window.0 as WindowId), process_id == std::process::id())
        }
    };
    let last = LAST_TARGET.load(Ordering::Relaxed);
    let chosen = pick_target(front, own, (last != 0).then_some(last));
    if let Some(window) = chosen.filter(|window| Some(*window) == front) {
        LAST_TARGET.store(window, Ordering::Relaxed);
    }
    chosen
}

#[cfg(not(windows))]
pub fn remember() -> Option<WindowId> {
    None
}

#[cfg(windows)]
pub async fn give_back(remembered: Option<WindowId>) -> Result<(), &'static str> {
    use windows::Win32::{
        Foundation::HWND,
        UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId, IsWindow, SetForegroundWindow,
        },
    };

    let handle = |id: WindowId| HWND(id as *mut core::ffi::c_void);
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

pub const ELEVATED_WARNING: &str = "The front window runs as administrator: clicks and typing are ignored. Use ask_user or finish with done and success=false.";

#[cfg(windows)]
fn token_is_elevated(process: windows::Win32::Foundation::HANDLE) -> Option<bool> {
    use windows::Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation},
        System::Threading::OpenProcessToken,
    };

    unsafe {
        let mut token = HANDLE::default();
        if let Err(err) = OpenProcessToken(process, TOKEN_QUERY, &mut token) {
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

#[cfg(windows)]
pub fn is_elevated() -> Option<bool> {
    use windows::Win32::{
        Foundation::CloseHandle,
        System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    };

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

fn app_is_elevated() -> bool {
    static ELEVATED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ELEVATED.get_or_init(|| {
        #[cfg(windows)]
        {
            token_is_elevated(unsafe { windows::Win32::System::Threading::GetCurrentProcess() })
                .unwrap_or(false)
        }
        #[cfg(not(windows))]
        {
            false
        }
    })
}

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
    fn the_target_is_the_front_window_or_else_the_last_outside_one() {
        assert_eq!(
            pick_target(Some(5), false, Some(9)),
            Some(5),
            "an outside window in front is the target"
        );
        assert_eq!(
            pick_target(Some(7), true, Some(9)),
            Some(9),
            "Rune in front: the last outside window"
        );
        assert_eq!(pick_target(None, false, Some(9)), Some(9));
        assert_eq!(
            pick_target(Some(7), true, None),
            None,
            "nothing known: nothing to give back"
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
