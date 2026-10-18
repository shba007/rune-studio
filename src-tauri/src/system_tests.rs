//! End-to-end checks for M1 against a real window. They move the real mouse and type real keys, so
//! they are `#[ignore]`d and run through `scripts/m1-system-test.ps1`, which starts the target
//! window these tests drive and passes its geometry in through environment variables.
//!
//!     pwsh scripts/m1-system-test.ps1

use std::{
    env, fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use windows::Win32::{
    Foundation::POINT,
    UI::{
        HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext},
        WindowsAndMessaging::{GetCursorPos, GetForegroundWindow, GetWindowThreadProcessId},
    },
};

use crate::{
    exec::{CancelToken, ExecError, input::{Input, MouseButton}},
    model::coords::{model_to_pixels, pixels_to_model},
    observe::screenshot::{self, MODEL_JPEG_QUALITY, MODEL_MAX_EDGE},
};

/// The real app is per-monitor DPI aware; the test process must be too, or coordinates would be
/// silently virtualised on a scaled display.
fn make_dpi_aware() {
    // Failing just means the process is already aware.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

fn number(name: &str) -> f64 {
    env::var(name)
        .unwrap_or_else(|_| panic!("{name} is not set; run these through scripts/m1-system-test.ps1"))
        .parse()
        .unwrap_or_else(|_| panic!("{name} is not a number"))
}

fn target_dir() -> PathBuf {
    PathBuf::from(env::var("RUNE_TARGET_DIR").expect("RUNE_TARGET_DIR is not set"))
}

/// Mouse-down events the target window has logged: `down,<x>,<y>,<button>,<clicks>`.
fn mouse_downs() -> Vec<(i32, i32, String, u32)> {
    fs::read_to_string(target_dir().join("events.log"))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let mut parts = line.split(',');
            (parts.next()? == "down").then_some(())?;
            Some((
                parts.next()?.parse().ok()?,
                parts.next()?.parse().ok()?,
                parts.next()?.to_string(),
                parts.next()?.parse().ok()?,
            ))
        })
        .collect()
}

fn textbox() -> String {
    fs::read_to_string(target_dir().join("textbox.txt")).unwrap_or_default()
}

/// The target publishes its state every ~100 ms; wait for `condition` to hold.
fn eventually<T>(what: &str, mut probe: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(value) = probe() {
            return value;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        thread::sleep(Duration::from_millis(50));
    }
}

fn cursor() -> (i32, i32) {
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point).expect("GetCursorPos failed") };
    (point.x, point.y)
}

/// Typing goes to whatever has focus, so refuse to type unless the target window does. A missed
/// focus click must never send keystrokes to the user's own windows.
fn assert_target_has_focus() {
    let mut process_id = 0_u32;
    unsafe { GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut process_id)) };
    let target = number("RUNE_TARGET_PID") as u32;
    assert_eq!(process_id, target, "the target window does not have focus; refusing to type");
}

fn focus_textbox(input: &mut Input, cancel: &CancelToken) {
    let monitor = screenshot::monitor(None).unwrap();
    let (x, y) = model_to_pixels(&monitor, number("RUNE_TEXT_X"), number("RUNE_TEXT_Y"));
    input.click(x, y, MouseButton::Left, false, cancel).unwrap();
    thread::sleep(Duration::from_millis(200));
    assert_target_has_focus();
}

#[test]
#[ignore = "moves the real mouse"]
fn cursor_lands_exactly_on_every_probe_point() {
    make_dpi_aware();
    let monitor = screenshot::monitor(None).unwrap();
    let cancel = CancelToken::default();
    let mut input = Input::new().unwrap();

    // Corners, edges, the centre, and awkward fractions that expose rounding errors.
    let probes = [
        (0.0, 0.0),
        (1000.0, 0.0),
        (0.0, 1000.0),
        (1000.0, 1000.0),
        (500.0, 500.0),
        (333.3, 666.6),
        (123.4, 876.5),
        (999.9, 0.1),
        (50.0, 950.0),
        (741.7, 259.3),
    ];

    let mut worst = 0;
    for (x, y) in probes {
        let (px, py) = model_to_pixels(&monitor, x, y);
        input.move_to(px, py, &cancel).unwrap();
        let (cx, cy) = cursor();
        let error = (cx - px).abs().max((cy - py).abs());
        worst = worst.max(error);
        println!("model ({x:>6.1},{y:>6.1}) -> pixel ({px:>4},{py:>4}) cursor ({cx:>4},{cy:>4}) error {error}px");
        assert_eq!((cx, cy), (px, py), "cursor is off target at model ({x}, {y})");
    }
    println!("worst error: {worst}px over {} points", probes.len());
}

#[test]
#[ignore = "clicks in the target window"]
fn clicks_arrive_at_the_requested_pixel_with_the_right_button() {
    make_dpi_aware();
    let monitor = screenshot::monitor(None).unwrap();
    let cancel = CancelToken::default();
    let mut input = Input::new().unwrap();
    let (px, py) = model_to_pixels(&monitor, number("RUNE_CLICK_X"), number("RUNE_CLICK_Y"));

    for (button, double, expect_button, expect_clicks) in [
        (MouseButton::Left, false, "Left", 1),
        (MouseButton::Right, false, "Right", 1),
        (MouseButton::Left, true, "Left", 2),
    ] {
        let before = mouse_downs().len();
        input.click(px, py, button, double, &cancel).unwrap();
        let expected_events = before + if double { 2 } else { 1 };
        let events = eventually("the click to arrive", || {
            let events = mouse_downs();
            (events.len() >= expected_events).then_some(events)
        });
        let last = events.last().unwrap();
        println!("{button:?} double={double}: target saw ({}, {}) {} x{}", last.0, last.1, last.2, last.3);
        assert_eq!((last.0, last.1), (px, py), "click landed on the wrong pixel");
        assert_eq!(last.2, expect_button);
        assert_eq!(last.3, expect_clicks, "click count");
    }
}

#[test]
#[ignore = "types into the target window"]
fn typing_and_key_combos_reach_the_focused_field() {
    make_dpi_aware();
    let cancel = CancelToken::default();
    let mut input = Input::new().unwrap();
    focus_textbox(&mut input, &cancel);

    // Select everything and clear, so the test does not depend on what was there before.
    input.press_keys(&["ctrl".into(), "a".into()], &cancel).unwrap();
    input.press_keys(&["backspace".into()], &cancel).unwrap();

    let sample = "Hello, wörld! 123 #@% é你好";
    input.type_text(sample, &cancel).unwrap();
    let typed = eventually("the text to appear", || {
        let text = textbox();
        (text == sample).then_some(text)
    });
    println!("typed ok: {typed:?}");

    // A combination: Ctrl+A selects all, then typing replaces the selection.
    input.press_keys(&["ctrl".into(), "a".into()], &cancel).unwrap();
    input.type_text("X", &cancel).unwrap();
    eventually("Ctrl+A then X to replace the text", || (textbox() == "X").then_some(()));

    // A modifier must not stay stuck down after a combination.
    thread::sleep(Duration::from_millis(100));
    let ctrl_down = unsafe {
        windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(
            windows::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL.0 as i32,
        )
    } as u16
        & 0x8000
        != 0;
    assert!(!ctrl_down, "Ctrl was left held down");
}

#[test]
#[ignore = "types into the target window"]
fn stop_interrupts_typing_mid_sentence() {
    make_dpi_aware();
    let cancel = CancelToken::default();
    let mut input = Input::new().unwrap();
    focus_textbox(&mut input, &cancel);
    input.press_keys(&["ctrl".into(), "a".into()], &cancel).unwrap();
    input.press_keys(&["backspace".into()], &cancel).unwrap();

    let long = "stop me ".repeat(120);
    let stopper = {
        let cancel = cancel.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(250));
            cancel.cancel();
        })
    };

    let started = Instant::now();
    let outcome = input.type_text(&long, &cancel);
    let elapsed = started.elapsed();
    stopper.join().unwrap();

    assert!(matches!(outcome, Err(ExecError::Cancelled)), "typing was not cancelled: {outcome:?}");
    thread::sleep(Duration::from_millis(300));
    let typed = textbox();
    println!("typed {} of {} characters before Stop, in {elapsed:?}", typed.chars().count(), long.chars().count());
    assert!(!typed.is_empty(), "nothing was typed before Stop");
    assert!(typed.chars().count() < long.chars().count(), "Stop did not interrupt typing");
    assert!(elapsed < Duration::from_millis(900), "Stop took too long to take effect: {elapsed:?}");
}

#[test]
#[ignore = "needs the target window on screen"]
fn the_model_sees_the_target_where_the_mapping_says() {
    make_dpi_aware();
    let monitor = screenshot::monitor(None).unwrap();
    let frame = screenshot::capture(None, MODEL_MAX_EDGE, MODEL_JPEG_QUALITY).unwrap();
    println!(
        "captured {}x{} from a {}x{} monitor in {} ms ({} bytes of JPEG)",
        frame.width,
        frame.height,
        monitor.width,
        monitor.height,
        frame.elapsed_ms,
        frame.jpeg.len()
    );

    let (left, top, width, height) = (
        number("RUNE_RECT_L") as i32,
        number("RUNE_RECT_T") as i32,
        number("RUNE_RECT_W") as i32,
        number("RUNE_RECT_H") as i32,
    );
    let is_magenta = |x: i32, y: i32| {
        let (mx, my) = pixels_to_model(&monitor, x, y);
        let [r, g, b] = frame.pixel_at_model(mx, my).0;
        (r as i32 - 255).abs() < 20 && g < 30 && (b as i32 - 255).abs() < 20
    };

    // Well inside the rectangle: magenta. Just outside every edge: not magenta. This pins the
    // mapping to a few pixels even after the downscale.
    let margin = 10;
    for (x, y) in [
        (left + width / 2, top + height / 2),
        (left + margin, top + margin),
        (left + width - margin, top + height - margin),
    ] {
        assert!(is_magenta(x, y), "expected magenta at pixel ({x}, {y})");
    }
    for (x, y) in [
        (left - margin, top + height / 2),
        (left + width + margin, top + height / 2),
        (left + width / 2, top - margin),
        (left + width / 2, top + height + margin),
    ] {
        assert!(!is_magenta(x, y), "unexpected magenta at pixel ({x}, {y}) outside the target");
    }
}

#[test]
#[ignore = "timing probe, no input"]
fn capture_speed() {
    make_dpi_aware();
    let mut rows = Vec::new();
    for round in 0..6 {
        let started = Instant::now();
        let frame = screenshot::capture(None, MODEL_MAX_EDGE, MODEL_JPEG_QUALITY).unwrap();
        rows.push((round, started.elapsed().as_millis(), frame.jpeg.len()));
    }
    for (round, ms, bytes) in &rows {
        println!("capture #{round}: {ms} ms, {} KB", bytes / 1024);
    }
}

/// Title of the window that has keyboard focus.
fn foreground_title() -> String {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
    let mut buffer = [0_u16; 256];
    // SAFETY: the buffer is valid for 256 UTF-16 units.
    let length = unsafe { GetWindowTextW(GetForegroundWindow(), &mut buffer) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

#[test]
#[ignore = "opens Notepad and types into it"]
fn the_caret_is_found_in_notepad() {
    use windows::{
        Win32::{Foundation::RECT, UI::WindowsAndMessaging::{FindWindowW, GetWindowRect}},
        core::w,
    };
    make_dpi_aware();
    let cancel = CancelToken::default();
    crate::exec::apps::open("notepad").unwrap();
    thread::sleep(Duration::from_millis(3500));

    // Click inside Notepad so it has the keyboard, and refuse to type anywhere else.
    let mut rect = RECT::default();
    // SAFETY: plain Win32 calls with a valid out-pointer.
    unsafe {
        let window = FindWindowW(w!("Notepad"), None).expect("no Notepad window");
        GetWindowRect(window, &mut rect).unwrap();
    }
    let mut input = Input::new().unwrap();
    input.click((rect.left + rect.right) / 2, (rect.top + rect.bottom) / 2, MouseButton::Left, false, &cancel).unwrap();
    thread::sleep(Duration::from_millis(500));
    let title = foreground_title();
    assert!(title.contains("Notepad"), "refusing to type: the focused window is \"{title}\", not Notepad");

    input.type_text("caret test", &cancel).unwrap();
    thread::sleep(Duration::from_millis(400));
    let caret = crate::exec::caret::position();
    println!("window: ({}, {}, {}, {}), caret: {caret:?}, mouse: {:?}", rect.left, rect.top, rect.right, rect.bottom, crate::exec::input::cursor_position());
    assert!(caret.is_some(), "the caret could not be located in Notepad");
}
