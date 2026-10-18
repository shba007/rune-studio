//! Where the text caret is on screen, so the overlay can mark the spot an agent just typed into.
//!
//! Two ways to find it, cheapest first: classic Win32 programs report their caret through
//! `GetGUIThreadInfo`; modern apps (Windows 11 Notepad, browsers) only show it through UI
//! Automation. If neither knows, the caller falls back to where the mouse is.

/// The caret's position in physical desktop pixels, when it can be found.
#[cfg(windows)]
pub fn position() -> Option<(i32, i32)> {
    from_gui_thread().or_else(from_ui_automation)
}

#[cfg(not(windows))]
pub fn position() -> Option<(i32, i32)> {
    None
}

#[cfg(windows)]
fn from_gui_thread() -> Option<(i32, i32)> {
    use windows::Win32::{
        Foundation::POINT,
        Graphics::Gdi::ClientToScreen,
        UI::WindowsAndMessaging::{GUITHREADINFO, GetGUIThreadInfo},
    };

    let mut info = GUITHREADINFO { cbSize: std::mem::size_of::<GUITHREADINFO>() as u32, ..Default::default() };
    // SAFETY: `info` is a valid GUITHREADINFO with its size set; 0 asks about the foreground thread.
    unsafe { GetGUIThreadInfo(0, &mut info) }.ok()?;
    if info.hwndCaret.is_invalid() {
        return None;
    }
    let caret = info.rcCaret;
    let mut point = POINT { x: (caret.left + caret.right) / 2, y: (caret.top + caret.bottom) / 2 };
    // SAFETY: the window handle comes from the system and `point` is writable.
    unsafe { ClientToScreen(info.hwndCaret, &mut point) }.as_bool().then_some((point.x, point.y))
}

#[cfg(windows)]
fn from_ui_automation() -> Option<(i32, i32)> {
    use std::ffi::c_void;

    use windows::{
        core::BOOL,
        Win32::System::{
            Com::{CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx},
            Ole::{SafeArrayDestroy, SafeArrayGetElement, SafeArrayGetLBound, SafeArrayGetUBound},
        },
        Win32::{
            UI::Accessibility::{CUIAutomation, IUIAutomation, IUIAutomationTextPattern2, UIA_TextPattern2Id},
        },
        core::Interface,
    };

    // SAFETY: standard COM use on this thread; every interface is released when dropped, and the
    // SAFEARRAY returned by UI Automation is read within its bounds and then destroyed.
    unsafe {
        // Already initialised on this thread is fine (S_FALSE) and a different mode is too.
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
        let focused = automation.GetFocusedElement().ok()?;
        let pattern: IUIAutomationTextPattern2 = focused.GetCurrentPattern(UIA_TextPattern2Id).ok()?.cast().ok()?;
        let mut active = BOOL(0);
        let range = pattern.GetCaretRange(&mut active).ok()?;
        let rectangles = range.GetBoundingRectangles().ok()?;
        if rectangles.is_null() {
            return None;
        }
        // The array holds left, top, width, height for each line the range covers.
        let low = SafeArrayGetLBound(rectangles, 1).ok()?;
        let high = SafeArrayGetUBound(rectangles, 1).ok()?;
        let mut values = Vec::new();
        for index in low..=high {
            let mut value = 0.0_f64;
            SafeArrayGetElement(rectangles, &index, (&mut value as *mut f64).cast::<c_void>()).ok()?;
            values.push(value);
        }
        let _ = SafeArrayDestroy(rectangles);
        let [left, top, width, height, ..] = values[..] else {
            return None;
        };
        Some(((left + width / 2.0).round() as i32, (top + height / 2.0).round() as i32))
    }
}
