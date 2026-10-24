mod agent;
pub const VERSION: &str = env!("RUNE_VERSION");

mod analysis;
mod archive;
mod backend;
mod bridge;
mod commands;
mod credentials;
mod cursor;
mod debug;
mod error;
#[cfg(feature = "eval")]
mod eval;
mod events;
mod exec;
mod flight;
mod hotkeys;
mod hub;
mod kit;
pub mod life;
mod model;
mod observe;
mod overlay;
mod prompts;
mod reflect;
mod review;
mod safety;
mod settings;
mod sync;
mod trace;
mod video;
mod voice;
mod window;
mod workflow;

#[cfg(test)]
mod grounding_probe;
#[cfg(all(test, windows))]
mod system_tests;

use crate::sync::LockExt;
use std::sync::Mutex;

use tauri::{
    Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tauri_plugin_global_shortcut::ShortcutState;

use events::{AgentState, AppState};
use hotkeys::Hotkeys;

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
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::summon(app)
        }));
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
                    let hotkeys = app.state::<AppState>().hotkeys.locked().clone();
                    if shortcut == &hotkeys.toggle {
                        window::summon(app);
                    } else if shortcut == &hotkeys.stop {
                        commands::stop(app.clone());
                    }
                })
                .build(),
        )
        .setup(|app| {
            cursor::reset();
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .targets([
                            tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                            tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Folder {
                                path: app.path().app_data_dir()?.join("logs"),
                                file_name: None,
                            }),
                        ])
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let _ = archive::init(app.handle());
            let trace = match app.path().app_data_dir() {
                Ok(dir) => trace::Recorder::start(&trace::db_path(&dir)),
                Err(err) => {
                    log::error!("no app data folder, runs will not be recorded: {err}");
                    trace::Recorder::noop()
                }
            };

            let loaded = settings::load(app.handle());
            let hotkeys = Hotkeys::from_settings(&loaded);
            let keep_days = loaded.archive_keep_days;
            let retention_trace = trace.clone();
            if keep_days > 0 {
                std::thread::spawn(move || {
                    let removed = archive::purge_expired(keep_days);
                    if !removed.is_empty() {
                        retention_trace.purge_sessions(removed);
                    }
                });
            }
            app.manage(AppState {
                steer: Default::default(),
                settings: Mutex::new(loaded),
                hotkeys: Mutex::new(hotkeys.clone()),
                mic: Mutex::new(None),
                run: Mutex::new(exec::RunScope::new()),
                verifier: Mutex::new(exec::CancelToken::default()),
                analysis: Default::default(),
                screen_requests: Default::default(),
                external: Default::default(),
                reflect: Default::default(),
                agent_state: Mutex::new(AgentState::Idle),
                lease: Mutex::new(bridge::lease::LeaseBook::default()),
                confirm: Mutex::new(None),
                #[cfg(feature = "agent-bridge")]
                act_turn: Default::default(),
                trace,
                http: reqwest::Client::new(),
                capabilities: std::sync::Arc::new(
                    crate::model::capabilities::CapabilityCache::default(),
                ),
            });

            {
                let handle = app.handle().clone();
                observe::mask::set_source(move || window::masked_rects(&handle));
            }
            reflect::start_watcher(app.handle());
            video::start_worker(app.handle());
            review::start_watcher(app.handle());
            video::start_motion_recorder(app.handle());
            if let Some(pill) = app.get_webview_window(window::MAIN) {
                window::protect(&pill)?;
                window::set_mode(app.handle(), window::Mode::Ambient)?;
                window::place_bottom_right(&pill)?;
            }

            #[cfg(feature = "agent-bridge")]
            bridge::server::start(app.handle());
            #[cfg(all(feature = "agent-bridge", unix))]
            bridge::path_link::ensure();

            let _ = hotkeys::apply(app.handle(), None, &hotkeys);

            let show = MenuItem::with_id(app, "show", "Show / hide", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
            let flight = MenuItem::with_id(app, "artifacts", "Open Sessions", true, None::<&str>)?;
            let stop = MenuItem::with_id(app, "stop", "Stop", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &settings, &flight, &stop, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("Rune Studio")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => window::toggle_visibility(app),
                    "settings" => window::open_settings(app),
                    "artifacts" => window::open_artifacts(app),
                    "stop" => commands::stop(app.clone()),
                    "quit" => app.exit(0),
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            #[cfg(feature = "eval")]
            eval::start(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            kit::client::kit_settings,
            kit::client::kit_save,
            kit::client::kit_discover,
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
            commands::analyze_screen,
            commands::run_workflow,
            commands::screen_insight_targets,
            commands::screen_request_state,
            commands::cancel_screen_request,
            commands::analyze_again,
            commands::analysis_state,
            commands::cancel_analysis,
            commands::close_analysis,
            commands::select_finding,
            commands::overlay_info,
            commands::external_agent,
            commands::reflection_answer,
            commands::pill_activity,
            commands::bridge_info,
            commands::get_control_label,
            commands::session_feedback,
            commands::open_settings_window,
            commands::flight_runs,
            commands::flight_run,
            commands::flight_steps,
            commands::flight_events,
            commands::flight_calls,
            commands::flight_call_body,
            commands::flight_file,
            commands::flight_reveal,
            commands::open_flight,
            commands::debug_set_state,
            debug::debug_monitors,
            debug::debug_capture,
            debug::debug_click,
            debug::debug_move,
            debug::debug_type,
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

#[cfg(test)]
mod version_tests {
    #[test]
    fn the_version_is_package_jsons_and_not_the_empty_cargo_default() {
        let package: serde_json::Value =
            serde_json::from_str(include_str!("../../package.json")).unwrap();
        assert_eq!(super::VERSION, package["version"].as_str().unwrap());
        assert_ne!(super::VERSION, "0.0.0");
    }
}
