use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const WINDOW: &str = "workspace";
pub const SECTION_EVENT: &str = "workspace_section";

fn clean(section: Option<&str>) -> &'static str {
    match section {
        Some("goals") => "goals",
        Some("personalities") => "personalities",
        Some("overview") => "overview",
        _ => "tasks",
    }
}

pub fn open(app: &AppHandle, section: Option<&str>) -> tauri::Result<()> {
    open_view(app, section, None)
}

pub fn open_task(app: &AppHandle, id: &str) -> tauri::Result<()> {
    open_view(app, Some("tasks"), Some(id))
}

fn open_view(app: &AppHandle, section: Option<&str>, run: Option<&str>) -> tauri::Result<()> {
    let section = clean(section);
    if let Some(window) = app.get_webview_window(WINDOW) {
        crate::window::show_panel(&window)?;
        window.emit(SECTION_EVENT, json!({ "section": section, "runId": run }))?;
        return Ok(());
    }
    let url = match run {
        Some(id) => format!(
            "index.html?view=workspace&section={section}&run={}",
            crate::flight::url_safe(id)
        ),
        None => format!("index.html?view=workspace&section={section}"),
    };
    let window = WebviewWindowBuilder::new(app, WINDOW, WebviewUrl::App(url.into()))
        .title("Workspace")
        .inner_size(crate::window::PANEL_WIDTH, crate::window::PANEL_HEIGHT)
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
        assert_eq!(super::clean(Some("overview")), "overview");
    }
}
