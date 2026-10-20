//! What the harness needs from Windows: the visible window titles, the running programs, and
//! closing one of the allowlisted programs. Everything is a stub that says "unsupported" elsewhere.

use super::check::Machine;
use super::spec::kill_allowed;

/// The real machine.
pub struct System;

/// The names (`notepad.exe`) in the output of `tasklist /FO CSV /NH`.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn parse_tasklist(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let first = line.trim().strip_prefix('"')?;
            first.split('"').next().map(str::to_string)
        })
        .filter(|name| !name.is_empty())
        .collect()
}

impl Machine for System {
    fn window_titles(&self) -> Result<Vec<String>, String> {
        imp::window_titles()
    }

    fn process_names(&self) -> Result<Vec<String>, String> {
        imp::process_names()
    }
}

/// Closes every running copy of an allowlisted program. Not running is fine.
pub fn kill_process(name: &str) -> Result<(), String> {
    if !kill_allowed(name) {
        return Err(format!(
            "{name} is not on the list of programs a suite may close"
        ));
    }
    imp::kill_process(name)
}

#[cfg(windows)]
mod imp {
    use std::process::Command;

    use windows::{
        Win32::{
            Foundation::{HWND, LPARAM},
            UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, IsWindowVisible},
        },
        core::BOOL,
    };

    use super::parse_tasklist;

    unsafe extern "system" fn collect(window: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` is the address of the `Vec<String>` that `window_titles` keeps alive
        // for the whole of the `EnumWindows` call, and nothing else touches it meanwhile.
        let titles = unsafe { &mut *(lparam.0 as *mut Vec<String>) };
        // SAFETY: plain Win32 calls with a buffer that is valid for the length passed.
        unsafe {
            if IsWindowVisible(window).as_bool() {
                let mut buffer = [0_u16; 512];
                let length = GetWindowTextW(window, &mut buffer).max(0) as usize;
                if length > 0 {
                    titles.push(String::from_utf16_lossy(&buffer[..length]));
                }
            }
        }
        BOOL(1)
    }

    pub fn window_titles() -> Result<Vec<String>, String> {
        let mut titles: Vec<String> = Vec::new();
        // SAFETY: `collect` only uses `lparam` as the vector passed here, which outlives the call.
        unsafe {
            EnumWindows(
                Some(collect),
                LPARAM(&mut titles as *mut Vec<String> as isize),
            )
        }
        .map_err(|err| format!("could not list the windows: {err}"))?;
        Ok(titles)
    }

    pub fn process_names() -> Result<Vec<String>, String> {
        let output = Command::new("tasklist")
            .args(["/FO", "CSV", "/NH"])
            .output()
            .map_err(|err| format!("could not run tasklist: {err}"))?;
        if !output.status.success() {
            return Err(format!("tasklist failed with {}", output.status));
        }
        Ok(parse_tasklist(&String::from_utf8_lossy(&output.stdout)))
    }

    pub fn kill_process(name: &str) -> Result<(), String> {
        // taskkill exits non-zero when the program is not running, which is exactly the goal.
        Command::new("taskkill")
            .args(["/IM", name, "/F"])
            .output()
            .map(|_| ())
            .map_err(|err| format!("could not run taskkill: {err}"))
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn window_titles() -> Result<Vec<String>, String> {
        Err("listing windows is only supported on Windows".into())
    }

    pub fn process_names() -> Result<Vec<String>, String> {
        Err("listing processes is only supported on Windows".into())
    }

    pub fn kill_process(_name: &str) -> Result<(), String> {
        Err("closing programs is only supported on Windows".into())
    }
}
