use super::ExecError;

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Shell(String),
    FileUrl(String),
}

#[cfg_attr(not(windows), allow(dead_code))]
const ALIASES: [(&str, &str); 12] = [
    ("calculator", "calc"),
    ("settings", "ms-settings:"),
    ("windows settings", "ms-settings:"),
    ("file explorer", "explorer"),
    ("explorer", "explorer"),
    ("paint", "mspaint"),
    ("task manager", "taskmgr"),
    ("command prompt", "cmd"),
    ("control panel", "control"),
    ("snipping tool", "snippingtool"),
    ("wordpad", "write"),
    ("registry editor", "regedit"),
];

#[cfg_attr(not(windows), allow(dead_code))]
pub fn plan(name: &str) -> Vec<Target> {
    let name = name.trim();
    let lower = name.to_lowercase();
    if lower.starts_with("file:") {
        return vec![Target::FileUrl(name.to_string())];
    }
    if let Some((_, program)) = ALIASES.iter().find(|(alias, _)| *alias == lower) {
        return vec![Target::Shell((*program).to_string())];
    }
    let bare = !name.is_empty() && !name.contains(['\\', '/', ':', '.', ' ']);
    if bare {
        vec![Target::Shell(format!("{name}.exe"))]
    } else {
        vec![Target::Shell(name.to_string())]
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn reg_value(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let (_, value) = line.split_once("REG_SZ")?;
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn exe_of_command(command: &str) -> Option<String> {
    let command = command.trim();
    if let Some(rest) = command.strip_prefix('"') {
        return rest
            .split('"')
            .next()
            .filter(|exe| !exe.is_empty())
            .map(str::to_string);
    }
    let end = command.to_lowercase().find(".exe").map(|at| at + 4)?;
    Some(command[..end].to_string())
}

#[cfg(windows)]
fn default_browser() -> Option<std::path::PathBuf> {
    use std::os::windows::process::CommandExt;

    let query = |key: &str, extra: &[&str]| -> Option<String> {
        let output = std::process::Command::new("reg.exe")
            .arg("query")
            .arg(key)
            .args(extra)
            .creation_flags(0x0800_0000)
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| reg_value(&String::from_utf8_lossy(&output.stdout)))
            .flatten()
    };
    let progid = query(
        r"HKCU\Software\Microsoft\Windows\Shell\Associations\UrlAssociations\https\UserChoice",
        &["/v", "ProgId"],
    )?;
    let command = query(&format!(r"HKCR\{progid}\shell\open\command"), &["/ve"])?;
    let exe = std::path::PathBuf::from(exe_of_command(&command)?);
    exe.is_file().then_some(exe)
}

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
    let mut last = format!("could not open \"{name}\"");
    for target in plan(name) {
        match target {
            Target::FileUrl(url) => {
                use std::os::windows::process::CommandExt;
                if url.contains(['?', '#'])
                    && let Some(browser) = default_browser()
                    && std::process::Command::new(browser)
                        .arg(&url)
                        .spawn()
                        .is_ok()
                {
                    return Ok(());
                }
                std::process::Command::new("rundll32.exe")
                    .arg("url.dll,FileProtocolHandler")
                    .arg(&url)
                    .creation_flags(0x0800_0000)
                    .spawn()
                    .map_err(|err| ExecError::Input(format!("could not open \"{name}\": {err}")))?;
                return Ok(());
            }
            Target::Shell(argument) => {
                let result = unsafe {
                    ShellExecuteW(
                        None,
                        &HSTRING::from("open"),
                        &HSTRING::from(argument.as_str()),
                        PCWSTR::null(),
                        PCWSTR::null(),
                        SW_SHOWNORMAL,
                    )
                };
                if result.0 as isize > 32 {
                    return Ok(());
                }
                last = format!("could not open \"{name}\" (error {})", result.0 as isize);
            }
        }
    }
    Err(ExecError::Input(last))
}

#[cfg(not(windows))]
pub fn open(_name: &str) -> Result<(), ExecError> {
    Err(ExecError::Input(
        "opening applications is only supported on Windows".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell(argument: &str) -> Vec<Target> {
        vec![Target::Shell(argument.into())]
    }

    #[test]
    fn a_bare_name_is_opened_as_an_exe_never_as_an_extensionless_file() {
        assert_eq!(plan("notepad"), shell("notepad.exe"));
        assert_eq!(plan("  Notepad "), shell("Notepad.exe"));
        assert_eq!(plan("chrome"), shell("chrome.exe"));
    }

    #[test]
    fn paths_extensions_protocols_and_addresses_are_left_alone() {
        for name in [
            "notepad.exe",
            r"C:\Windows\notepad.exe",
            "C:/x/y.txt",
            "ms-settings:display",
            "https://example.com/a?b=1",
            "mailto:a@b.c",
            r"\\server\share",
            "my notes",
        ] {
            assert_eq!(plan(name), shell(name), "{name}");
        }
    }

    #[test]
    fn friendly_names_map_to_what_windows_understands() {
        assert_eq!(plan("Calculator"), shell("calc"));
        assert_eq!(
            plan("calc"),
            shell("calc.exe"),
            "calc.exe exists and is fine"
        );
        assert_eq!(plan("settings"), shell("ms-settings:"));
        assert_eq!(plan("Windows Settings"), shell("ms-settings:"));
        assert_eq!(plan("File Explorer"), shell("explorer"));
        assert_eq!(plan("task manager"), shell("taskmgr"));
    }

    #[test]
    fn registry_output_and_open_commands_are_read() {
        let progid = "\r\nHKEY_CURRENT_USER\\Software\\X\\UserChoice\r\n    ProgId    REG_SZ    BraveHTML\r\n";
        assert_eq!(reg_value(progid).as_deref(), Some("BraveHTML"));
        assert_eq!(reg_value("nothing useful"), None);
        let command =
            r#"    (Default)    REG_SZ    "C:\Program Files\Brave\brave.exe" --single-argument %1"#;
        assert_eq!(
            reg_value(command).as_deref(),
            Some(r#""C:\Program Files\Brave\brave.exe" --single-argument %1"#)
        );
        assert_eq!(
            exe_of_command(r#""C:\Program Files\Brave\brave.exe" --single-argument %1"#).as_deref(),
            Some(r"C:\Program Files\Brave\brave.exe")
        );
        assert_eq!(
            exe_of_command(r#"C:\Apps\ff.exe -osint -url "%1""#).as_deref(),
            Some(r"C:\Apps\ff.exe")
        );
        assert_eq!(exe_of_command("   "), None);
    }

    #[test]
    fn a_local_page_keeps_its_query_string_by_going_to_the_url_handler_whole() {
        let url = "file:///C:/eval/fixtures/click.html?run=20261019-3";
        assert_eq!(plan(url), vec![Target::FileUrl(url.into())]);
        assert_eq!(
            plan("FILE:///x.html?a=1&b=2"),
            vec![Target::FileUrl("FILE:///x.html?a=1&b=2".into())]
        );
    }
}
