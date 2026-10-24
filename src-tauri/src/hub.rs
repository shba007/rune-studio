use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const WINDOW: &str = "settings";
pub const SECTION_EVENT: &str = "settings_section";
pub const CHANGED_EVENT: &str = "settings_changed";

const SECTIONS: [&str; 6] = [
    "general",
    "agents",
    "tools",
    "artifacts",
    "shortcuts",
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
    let section = clean(section);
    if let Some(window) = app.get_webview_window(WINDOW) {
        let _ = window.unminimize();
        window.show()?;
        window.set_focus()?;
        window.emit(SECTION_EVENT, json!({ "section": section }))?;
        return Ok(());
    }
    let url = format!("index.html?view=settings&section={section}");
    let window = WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::App(url.into()))
        .title("Rune Studio Settings")
        .inner_size(960.0, 640.0)
        .min_inner_size(760.0, 520.0)
        .resizable(true)
        .decorations(false)
        .center()
        .build()?;
    crate::window::protect(&window)?;
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
        assert_eq!(clean(Some("artifacts")), "artifacts");
        assert_eq!(clean(Some("nonsense")), "general");
        assert_eq!(clean(None), "general");
    }
}
