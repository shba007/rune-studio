use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const WINDOW: &str = "workspace";
pub const SECTION_EVENT: &str = "workspace_section";

fn clean(section: Option<&str>) -> &'static str {
    match section {
        Some("goals") => "goals",
        _ => "tasks",
    }
}

pub fn open(app: &AppHandle, section: Option<&str>) -> tauri::Result<()> {
    let section = clean(section);
    if let Some(window) = app.get_webview_window(WINDOW) {
        let _ = window.unminimize();
        window.set_always_on_top(true)?;
        window.show()?;
        window.set_focus()?;
        window.emit(SECTION_EVENT, json!({ "section": section }))?;
        return Ok(());
    }
    let url = format!("index.html?view=workspace&section={section}");
    let window = WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::App(url.into()))
        .title("Rune Goals & Tasks")
        .inner_size(1100.0, 740.0)
        .min_inner_size(760.0, 520.0)
        .resizable(true)
        .always_on_top(true)
        .decorations(false)
        .center()
        .build()?;
    crate::window::protect(&window)?;
    window.set_focus()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn defaults_and_legacy_sections_land_on_tasks() {
        assert_eq!(super::clean(None), "tasks");
        assert_eq!(super::clean(Some("artifacts")), "tasks");
        assert_eq!(super::clean(Some("unknown")), "tasks");
        assert_eq!(super::clean(Some("goals")), "goals");
    }
}
