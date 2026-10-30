use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const WINDOW: &str = "settings";
pub const SECTION_EVENT: &str = "settings_section";
pub const CHANGED_EVENT: &str = "settings_changed";

const SECTIONS: [&str; 9] = [
    "general",
    "models",
    "agents",
    "tools",
    "capture",
    "data",
    "shortcuts",
    "developer",
    "about",
];

fn clean(section: Option<&str>) -> &'static str {
    SECTIONS
        .iter()
        .copied()
        .find(|known| Some(*known) == section)
        .unwrap_or("general")
}

pub fn open(app: &AppHandle, section: Option<&str>) -> tauri::Result<()> {
    if matches!(section, Some("artifacts" | "tasks" | "goals")) {
        return crate::workspace::open(app, section);
    }
    let section = clean(section);
    if let Some(window) = app.get_webview_window(WINDOW) {
        crate::window::show_panel(&window)?;
        window.emit(SECTION_EVENT, json!({ "section": section }))?;
        return Ok(());
    }
    let url = format!("index.html?view=settings&section={section}");
    let window = WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::App(url.into()))
        .title("Rune Studio Settings")
        .inner_size(crate::window::PANEL_WIDTH, crate::window::PANEL_HEIGHT)
        .min_inner_size(760.0, 520.0)
        .resizable(true)
        .always_on_top(true)
        .decorations(false)
        .transparent(true)
        .center()
        .build()?;
    crate::window::protect(&window)?;
    window.set_focus()?;
    Ok(())
}

pub fn changed(app: &AppHandle) {
    let _ = app.emit(CHANGED_EVENT, ());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_section_opens_general() {
        assert_eq!(clean(Some("about")), "about");
        for section in ["models", "capture", "data", "developer"] {
            assert_eq!(clean(Some(section)), section);
        }
        assert_eq!(clean(Some("artifacts")), "general");
        assert_eq!(clean(Some("nonsense")), "general");
        assert_eq!(clean(None), "general");
    }
}
