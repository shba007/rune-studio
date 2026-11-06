use std::{
    str::FromStr,
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
    },
    time::Duration,
};

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::Shortcut;
use windows::Win32::{
    Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_CONTROL, VK_ESCAPE, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
        },
        WindowsAndMessaging::{
            CallNextHookEx, GetMessageW, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, SetWindowsHookExW,
            UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
        },
    },
};

use crate::sync::LockExt;

static EVENTS: OnceLock<SyncSender<()>> = OnceLock::new();
static AVAILABLE: AtomicBool = AtomicBool::new(false);
static HELD: AtomicBool = AtomicBool::new(false);

fn is_escape(shortcut: Shortcut) -> bool {
    shortcut == Shortcut::from_str(super::DEFAULT_STOP).expect("default shortcut must parse")
}

pub fn handles(shortcut: Shortcut) -> bool {
    AVAILABLE.load(Ordering::Acquire) && is_escape(shortcut)
}

fn should_signal(code: u32, injected: bool, modified: bool, repeated: bool) -> bool {
    code == u32::from(VK_ESCAPE.0) && !injected && !modified && !repeated
}

unsafe extern "system" fn hook(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
    if code >= 0 && data.0 != 0 {
        let event = unsafe { &*(data.0 as *const KBDLLHOOKSTRUCT) };
        if event.vkCode == u32::from(VK_ESCAPE.0) && !event.flags.contains(LLKHF_INJECTED) {
            if message.0 == WM_KEYUP as usize || message.0 == WM_SYSKEYUP as usize {
                HELD.store(false, Ordering::Release);
            } else if message.0 == WM_KEYDOWN as usize || message.0 == WM_SYSKEYDOWN as usize {
                let modified = [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN]
                    .into_iter()
                    .any(|key| unsafe { GetAsyncKeyState(i32::from(key.0)) } < 0);
                let repeated = HELD.swap(true, Ordering::AcqRel);
                if should_signal(event.vkCode, false, modified, repeated)
                    && let Some(sender) = EVENTS.get()
                {
                    let _ = sender.try_send(());
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, code, message, data) }
}

pub fn start(app: &AppHandle) -> Result<(), String> {
    let (sender, receiver) = mpsc::sync_channel(1);
    EVENTS
        .set(sender)
        .map_err(|_| "Escape listener is already initialized".to_string())?;
    let (ready, result) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let module = match unsafe { GetModuleHandleW(None) } {
            Ok(module) => HINSTANCE(module.0),
            Err(error) => {
                let _ = ready.send(Err(error.to_string()));
                return;
            }
        };
        let installed = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), Some(module), 0) };
        let hook = match installed {
            Ok(hook) => hook,
            Err(error) => {
                let _ = ready.send(Err(error.to_string()));
                return;
            }
        };
        if ready.send(Ok(())).is_err() {
            let _ = unsafe { UnhookWindowsHookEx(hook) };
            return;
        }
        let mut message = MSG::default();
        while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {}
        AVAILABLE.store(false, Ordering::Release);
        let _ = unsafe { UnhookWindowsHookEx(hook) };
    });
    result
        .recv_timeout(Duration::from_secs(3))
        .map_err(|error| format!("Escape listener did not start: {error}"))??;
    AVAILABLE.store(true, Ordering::Release);
    let app = app.clone();
    std::thread::spawn(move || {
        while receiver.recv().is_ok() {
            let shortcut = app.state::<crate::events::AppState>().hotkeys.locked().stop;
            if is_escape(shortcut) {
                crate::commands::stop(app.clone());
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_new_unmodified_physical_escape_signals_stop() {
        assert!(should_signal(27, false, false, false));
        assert!(!should_signal(27, true, false, false));
        assert!(!should_signal(27, false, true, false));
        assert!(!should_signal(27, false, false, true));
        assert!(!should_signal(65, false, false, false));
    }
}
