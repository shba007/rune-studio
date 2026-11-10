mod agent;
mod alarms;
pub const VERSION: &str = env!("RUNE_VERSION");

mod analysis;
mod archive;
mod backend;
mod bridge;
mod commands;
mod connections;
mod credentials;
mod cursor;
mod debug;
mod domain;
#[cfg(all(windows, feature = "embedded-infer"))]
mod embedded_download;
#[cfg(all(windows, feature = "embedded-infer"))]
mod embedded_infer;
mod embedded_kit;
mod error;
#[cfg(feature = "eval")]
mod eval;
mod events;
mod exec;
mod goal_domains;
mod hotkeys;
mod hub;
mod infer_catalog;
mod intake;
mod intake_discovery;
mod intake_domain;
mod intake_pathways;
mod intake_subdomains;
mod kit;
pub mod life;
mod model;
mod observe;
mod onboarding;
mod overlay;
mod personalities;
mod playground;
mod portfolio;
mod preview;
mod prompts;
mod recording;
mod reflect;
mod review;
mod safety;
mod schedule;
mod settings;
mod sync;
mod task_calendar;
mod task_proposals;
mod tasks;
mod trace;
mod video;
mod voice;
mod window;
mod workflow;
mod workspace;
mod workspace_tasks;

#[cfg(test)]
mod grounding_probe;
#[cfg(all(test, windows))]
mod system_tests;

use crate::sync::LockExt;
use std::sync::Mutex;

use tauri::{
    Emitter, Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tauri_plugin_global_shortcut::ShortcutState;
use tauri_plugin_notification::NotificationExt;

use events::{AgentState, AppState};
use hotkeys::Hotkeys;

#[cfg(feature = "agent-bridge")]
pub fn cli_main() -> i32 {
    bridge::cli::run()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = dotenvy::from_filename("../.env").or_else(|_| dotenvy::dotenv());
    #[cfg(windows)]
    window::hide_console();

    let mut builder = tauri::Builder::default();

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if onboarding::prepare(app).unwrap_or(false) {
                if let Err(error) = onboarding::open(app) {
                    log::error!("Could not reopen onboarding: {error}");
                }
                return;
            }
            window::summon(app)
        }));
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
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
        .on_window_event(|window, event| {
            if window.label() == window::MAIN
                && matches!(
                    event,
                    tauri::WindowEvent::Moved(_)
                        | tauri::WindowEvent::Resized(_)
                        | tauri::WindowEvent::ScaleFactorChanged { .. }
                )
            {
                let app = window.app_handle();
                let _ = app.emit("pill_geometry", commands::pill_rect(app.clone()));
            }
        })
        .setup(|app| {
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            let show_onboarding = onboarding::prepare(app.handle()).unwrap_or_else(|error| {
                log::error!("Could not open onboarding safely: {error}");
                false
            });
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
            #[cfg(debug_assertions)]
            let loaded = {
                let mut loaded = loaded;
                if std::env::var_os("RUNE_STUDIO_ONBOARDING_TEST_SHOW_RECORDINGS")
                    .is_some_and(|value| value == "1")
                {
                    loaded.demo_mode = true;
                }
                loaded
            };
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
            app.manage(playground::PlaygroundRuns::default());
            app.manage(embedded_kit::EmbeddedKitManager::default());
            #[cfg(feature = "embedded-kit")]
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(manager) = handle.try_state::<embedded_kit::EmbeddedKitManager>()
                        && let Err(e) = manager.start().await
                    {
                        log::info!("Embedded Kit initialization: {e}");
                    }
                });
            }
            #[cfg(all(windows, feature = "embedded-infer"))]
            {
                app.manage(embedded_infer::EmbeddedManager::default());
                app.manage(embedded_download::Downloads::default());
                app.manage(embedded_download::Imports::default());
                let should_start = app
                    .state::<AppState>()
                    .settings
                    .locked()
                    .connections
                    .iter()
                    .any(|connection| connection.id == "studio-embedded-infer");
                if should_start {
                    let handle = app.handle().clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = handle
                            .state::<embedded_infer::EmbeddedManager>()
                            .start(&handle, false)
                            .await
                        {
                            log::error!("Could not start local Infer: {error}");
                        }
                    });
                }
            }

            {
                let handle = app.handle().clone();
                observe::mask::set_source(move || window::masked_rects(&handle));
            }
            reflect::start_watcher(app.handle());
            video::start_worker(app.handle());
            review::start_watcher(app.handle());
            video::start_motion_recorder(app.handle());
            workspace_tasks::start(app.handle());
            if let Some(pill) = app.get_webview_window(window::MAIN) {
                window::protect(&pill)?;
                window::set_mode(app.handle(), window::Mode::Ambient)?;
                window::place_bottom_right(&pill)?;
            }

            #[cfg(feature = "agent-bridge")]
            bridge::server::start(app.handle());
            #[cfg(all(feature = "agent-bridge", unix))]
            bridge::path_link::ensure();

            #[cfg(windows)]
            if let Err(error) = hotkeys::start_escape_listener(app.handle()) {
                log::warn!("Could not start Escape listener; trying global shortcut: {error}");
            }
            if let Err(error) = hotkeys::apply(app.handle(), None, &hotkeys) {
                log::error!("Could not register Studio shortcuts: {error}");
                let _ = app
                    .notification()
                    .builder()
                    .title("Rune shortcut unavailable")
                    .body("A shortcut could not be registered. Use Stop in the Rune tray menu and review your shortcuts in Settings.")
                    .show();
            }

            let show = MenuItem::with_id(app, "show", "Show / hide", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
            let preview = MenuItem::with_id(app, "artifacts", "Workspace…", true, None::<&str>)?;
            let stop = MenuItem::with_id(app, "stop", "Stop", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &settings, &preview, &stop, &quit])?;
            let mut tray = TrayIconBuilder::with_id("main")
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

            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            if show_onboarding {
                onboarding::open(app.handle())?;
            }

            #[cfg(feature = "eval")]
            eval::start(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cursor::set_cursor_images,
            commands::installed_versions,
            commands::open_project_link,
            tasks::tasks_state,
            personalities::personalities_state,
            personalities::personality_save,
            personalities::personality_propose,
            tasks::submit_independent_task,
            tasks::cancel_independent_task,
            tasks::cancel_planned_request,
            tasks::dismiss_planned_request,
            video::video_recording_state,
            kit::client::kit_settings,
            kit::client::kit_save,
            kit::client::kit_discover,
            kit::client::kit_configuration,
            kit::client::kit_update_configuration,
            kit::client::kit_token_set,
            kit::client::kit_token_present,
            kit::client::kit_remotes,
            kit::client::kit_packages,
            kit::client::kit_icon,
            kit::client::kit_registry,
            kit::client::kit_install_package,
            kit::client::kit_package_operation,
            kit::client::kit_remote_update,
            commands::get_settings,
            commands::save_settings,
            commands::has_api_key,
            commands::set_api_key,
            commands::check_backend,
            commands::embedded_infer_available,
            commands::start_embedded_infer,
            commands::embedded_kit_available,
            commands::start_embedded_kit,
            commands::reload_embedded_infer,
            infer_catalog::infer_select_agent,
            infer_catalog::infer_agent_role,
            infer_catalog::infer_model_icon,
            infer_catalog::infer_model_remove,
            infer_catalog::infer_model_residency,
            infer_catalog::infer_analysis_role,
            infer_catalog::infer_visual_roles,
            commands::embedded_model_candidates,
            commands::embedded_model_download,
            commands::embedded_model_scan,
            commands::pick_folder,
            commands::embedded_model_import,
            commands::embedded_model_download_status,
            commands::embedded_model_download_cancel,
            commands::embedded_model_use,
            infer_catalog::infer_catalog_read,
            infer_catalog::infer_catalog_write,
            connections::connections_state,
            model::activity::model_activity,
            connections::connections_save,
            connections::connections_activate,
            connections::connections_delete,
            connections::connections_test,
            commands::set_mode,
            commands::fit_height,
            commands::pill_rect,
            commands::set_blur_regions,
            commands::stop,
            commands::start_task,
            commands::steer,
            commands::toggle_listening,
            commands::confirm,
            commands::analyze_screen,
            commands::run_workflow,
            commands::visual_analysis_targets,
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
            onboarding::onboarding_complete,
            onboarding::onboarding_platform,
            commands::open_workspace,
            playground::open_playground,
            playground::playground_start_run,
            portfolio::portfolio_raw_preview,
            portfolio::portfolio_develop_raw,
            portfolio::portfolio_developed_preview,
            portfolio::portfolio_suggest_look,
            portfolio::portfolio_export_composite,
            portfolio::portfolio_finish_preview,
            playground::portfolio_classify,
            playground::portfolio_review,
            playground::playground_extract_subject,
            playground::playground_cancel_run,
            playground::playground_image_voice_flow,
            playground::playground_visual_plan,
            playground::playground_visual_branch,
            playground::playground_depth,
            playground::playground_caption,
            playground::playground_upscale,
            playground::playground_create,
            playground::playground_edit,
            playground::playground_variation,
            playground::playground_reference,
            playground::playground_speech,
            playground::playground_transcribe,
            playground::playground_translate_speech,
            crate::intake::decisions_state,
            crate::intake::decision_answer,
            crate::intake::decision_delete,
            crate::intake::decision_refine,
            crate::intake::decision_pathways,
            crate::intake::decision_select_pathway,
            crate::intake::decision_propose_tasks,
            crate::intake::decision_review_task,
            crate::intake::workspace_new_goal,
            crate::intake::workspace_goal_modes,
            crate::intake::goal_measurement,
            workspace_tasks::workspace_tasks_state,
            workspace_tasks::workspace_task_create,
            workspace_tasks::workspace_task_zone,
            workspace_tasks::workspace_task_snooze,
            workspace_tasks::workspace_task_due_action,
            alarms::alarms_autostart,
            alarms::alarms_set_autostart,
            schedule::schedule_interpret,
            workspace_tasks::workspace_task_action,
            workspace_tasks::workspace_task_link,
            workspace_tasks::workspace_task_import,
            workspace_tasks::workspace_task_schedule,
            workspace_tasks::workspace_history_progress,
            workspace_tasks::workspace_calendar,
            commands::task_attempts,
            commands::preview_runs,
            commands::preview_task_history,
            commands::preview_original_prompt,
            commands::preview_delete,
            commands::preview_run,
            commands::preview_steps,
            commands::preview_events,
            commands::preview_calls,
            commands::preview_call_body,
            commands::preview_file,
            commands::preview_reveal,
            commands::open_preview,
            commands::debug_set_state,
            recording::screen_recording_state,
            recording::start_screen_recording,
            recording::stop_screen_recording,
            debug::debug_monitors,
            debug::debug_capture,
            debug::debug_click,
            debug::debug_move,
            debug::debug_type,
            debug::debug_keys,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                tauri::async_runtime::block_on(
                    app.state::<embedded_kit::EmbeddedKitManager>().stop(),
                );
                #[cfg(all(windows, feature = "embedded-infer"))]
                tauri::async_runtime::block_on(
                    app.state::<embedded_infer::EmbeddedManager>().stop(),
                );
                #[cfg(not(all(windows, feature = "embedded-infer")))]
                let _ = app;
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
