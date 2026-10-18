mod agent;
mod backend;
mod bridge;
mod commands;
mod cursor;
#[cfg(debug_assertions)]
mod debug;
mod error;
mod events;
mod exec;
mod hotkeys;
mod model;
mod observe;
mod overlay;
mod safety;
mod settings;
mod voice;
mod window;

#[cfg(test)]
mod probe_m15;
#[cfg(test)]
mod system_tests;

use std::sync::Mutex;

use tauri::{
    Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tauri_plugin_global_shortcut::ShortcutState;

use events::{AgentState, AppState};
use hotkeys::Hotkeys;

/// Entry point of the `rune-studio` command (see `bridge::cli`). Returns the exit code.
#[cfg(feature = "agent-bridge")]
pub fn cli_main() -> i32 {
    bridge::cli::run()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {

    let _ = dotenvy::from_filename("../.env").or_else(|_| dotenvy::dotenv());

    let mut builder = tauri::Builder::default();

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
        .plugin(tauri_plugin_process::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let hotkeys = app.state::<AppState>().hotkeys.lock().expect("hotkeys lock poisoned").clone();
                    if shortcut == &hotkeys.toggle {
                        window::summon(app);
                    } else if shortcut == &hotkeys.stop {
                        commands::stop(app.clone());
                    }
                })
                .build(),
        )
        .setup(|app| {
            // A pointer left changed by an earlier run that did not shut down cleanly.
            cursor::reset();
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let loaded = settings::load(app.handle());
            let hotkeys = Hotkeys::from_settings(&loaded);
            app.manage(AppState {
                steer: Default::default(),
                settings: Mutex::new(loaded),
                hotkeys: Mutex::new(hotkeys.clone()),
                mic: Mutex::new(None),
                cancel: exec::CancelToken::default(),
                agent_state: Mutex::new(AgentState::Idle),
                lease: Mutex::new(bridge::lease::LeaseBook::default()),
                confirm: Mutex::new(None),
                http: reqwest::Client::new(),
            });

            if let Some(pill) = app.get_webview_window(window::MAIN) {
                window::exclude_from_capture(&pill)?;
                // Windows clamps the configured size to its minimum window width; an explicit
                // resize is exact.
                window::set_mode(app.handle(), window::Mode::Ambient)?;
                window::place_bottom_right(&pill)?;
            }

            #[cfg(feature = "agent-bridge")]
            bridge::server::start(app.handle());

            // Failures are already logged; the pill stays reachable from the tray.
            let _ = hotkeys::apply(app.handle(), None, &hotkeys);

            let show = MenuItem::with_id(app, "show", "Show / hide", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
            // Stop lives in the tray too: it works while the window is click-through and does
            // not depend on the Stop hotkey being free.
            let stop = MenuItem::with_id(app, "stop", "Stop", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &settings, &stop, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("Rune Studio")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => window::toggle_visibility(app),
                    "settings" => window::open_settings(app),
                    "stop" => commands::stop(app.clone()),
                    "quit" => app.exit(0),
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::has_api_key,
            commands::set_api_key,
            commands::check_backend,
            commands::set_mode,
            commands::fit_height,
            commands::pill_rect,
            commands::stop,
            commands::start_task,
            commands::steer,
            commands::toggle_listening,
            commands::confirm,
            commands::bridge_info,
            commands::get_control_label,
            #[cfg(debug_assertions)]
            commands::debug_set_state,
            #[cfg(debug_assertions)]
            debug::debug_monitors,
            #[cfg(debug_assertions)]
            debug::debug_capture,
            #[cfg(debug_assertions)]
            debug::debug_click,
            #[cfg(debug_assertions)]
            debug::debug_type,
            #[cfg(debug_assertions)]
            debug::debug_keys,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_, event| {
            if let tauri::RunEvent::Exit = event {
                cursor::reset();
            }
        });
}
