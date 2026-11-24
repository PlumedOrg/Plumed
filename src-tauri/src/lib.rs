#![allow(clippy::module_inception)]

mod app_update;
mod audio;
pub mod cli;
mod clipboard;
mod commands;
mod dictionary;
mod engine;
mod formatting_rules;
mod grammar;
mod history;
mod local_api;
mod medical;
mod model;
mod overlay;
mod remote_mic;
mod settings;
mod shortcuts;
mod stats;
mod telemetry;
mod utils;

#[cfg(target_os = "linux")]
pub use utils::platform::is_wayland_session;

use crate::shortcuts::init_shortcuts;
use audio::preload_engine;
use audio::types::AudioState;
use commands::*;
use dictionary::Dictionary;
use log::{error, info, warn};
use model::Model;
use overlay::tray::setup_tray;
use std::str::FromStr;
use std::sync::Arc;
use tauri::{DeviceEventFilter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_log::{Target, TargetKind, TimezoneStrategy};

pub(crate) fn show_main_window(app: &tauri::AppHandle) {
    if let Some(main_window) = app.get_webview_window("main") {
        // Wayland compositors flag hidden-to-tray windows as minimised;
        // `show()` alone leaves the webview frozen (Handy pattern).
        match main_window.unminimize() {
            Ok(_) => (),
            Err(e) => warn!("Failed to unminimize window: {}", e),
        }
        match main_window.show() {
            Ok(_) => (),
            Err(e) => error!("Failed to show window: {}", e),
        }
        match main_window.set_focus() {
            Ok(_) => (),
            Err(e) => error!("Failed to focus window: {}", e),
        }
    } else {
        warn!("Main window not found");
    }
}

pub fn run() {
    // rustls 0.23 panics on first TLS load without an explicit CryptoProvider.
    if rustls::crypto::ring::default_provider()
        .install_default()
        .is_err()
    {
        log::warn!("Rustls crypto provider was already installed");
    }

    // `transcribe` and `serve` run headless, beside a running app if any.
    let is_headless = matches!(
        cli::parse_raw_args(&std::env::args().collect::<Vec<_>>()),
        Ok(Some(
            cli::CliCommand::Transcribe { .. } | cli::CliCommand::Serve { .. }
        ))
    );

    // transcribe keeps stdout for the transcription only; logs go to stderr.
    let log_targets = if is_headless {
        vec![Target::new(TargetKind::Stderr)]
    } else {
        vec![
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::Webview),
            Target::new(TargetKind::LogDir { file_name: None }),
        ]
    };

    let mut builder = tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets(log_targets)
                .timezone_strategy(TimezoneStrategy::UseLocal)
                .max_file_size(crate::utils::log_watchdog::MAX_LOG_FILE_SIZE as u128)
                .level(log::LevelFilter::Trace)
                .level_for("ort", log::LevelFilter::Warn)
                .level_for("ort::logging", log::LevelFilter::Warn)
                .level_for("zbus", log::LevelFilter::Warn)
                .level_for("tracing", log::LevelFilter::Warn)
                .level_for("symphonia_core", log::LevelFilter::Warn)
                .level_for("symphonia_bundle_mp3", log::LevelFilter::Warn)
                .level_for("enigo", log::LevelFilter::Info)
                .level_for("reqwest", log::LevelFilter::Info)
                .level_for("hyper_util", log::LevelFilter::Info)
                .level_for("tauri_plugin_updater", log::LevelFilter::Info)
                .level_for("arboard", log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build());

    if !is_headless {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            match cli::parse_raw_args(&args) {
                Ok(Some(cli::CliCommand::Import {
                    file_path,
                    strategy,
                })) => match cli::import::execute_import(app, &file_path, &strategy) {
                    Ok(msg) => {
                        info!("CLI import (hot-reload): {}", msg);
                        cli::import::apply_hot_reload_side_effects(app);
                    }
                    Err(msg) => {
                        error!("CLI import failed: {}", msg);
                    }
                },
                Ok(Some(cmd)) => {
                    info!("CLI dispatch (hot): {:?}", cmd);
                    crate::shortcuts::cli_dispatch::dispatch(app, &cmd);
                }
                Ok(None) => {
                    if !args.iter().any(|arg| arg == "--hidden") {
                        show_main_window(app);
                    }
                }
                Err(msg) => {
                    // Hot path: a live instance must survive a malformed
                    // external CLI call. Log and show the window so the
                    // user sees the app instead of nothing happening.
                    log::error!("{}", msg);
                    show_main_window(app);
                }
            }
        }));
    }

    builder
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .arg("--autostart")
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_process::init())
        .device_event_filter(DeviceEventFilter::Never)
        .setup(|app| {
            // The folder's mode guards every file inside, whatever its own
            match app.path().app_data_dir() {
                Ok(dir) => {
                    if let Err(e) = crate::utils::private_dir::ensure_private_app_data_dir(&dir) {
                        log::warn!("Could not make {} private: {}", dir.display(), e);
                    }
                }
                Err(e) => log::warn!("No app data dir: {}", e),
            }

            let start_hidden =
                std::env::args().any(|arg| arg == "--autostart" || arg == "--hidden");
            if start_hidden {
                info!("Starting minimized to tray (--autostart or --hidden)");
                if let Some(main_window) = app.get_webview_window("main") {
                    let _ = main_window.hide();
                }
            }

            // Re-register autostart with --autostart for users who enabled it before this update.
            if let Ok(true) = app.autolaunch().is_enabled() {
                let _ = app.autolaunch().enable();
            }

            // Early CLI detection, before heavy initialization.
            let raw_args: Vec<String> = std::env::args().collect();
            let pending_cli_action = match cli::parse_raw_args(&raw_args) {
                Ok(Some(cli::CliCommand::Import {
                    file_path,
                    strategy,
                })) => {
                    if let Some(main_window) = app.get_webview_window("main") {
                        let _ = main_window.hide();
                    }
                    match cli::import::execute_import(app.handle(), &file_path, &strategy) {
                        Ok(msg) => {
                            println!("{}", msg);
                            app.handle().exit(0);
                        }
                        Err(msg) => {
                            eprintln!("{}", msg);
                            app.handle().exit(1);
                        }
                    }
                    return Ok(());
                }
                Ok(Some(cli::CliCommand::Quit)) => {
                    if let Some(main_window) = app.get_webview_window("main") {
                        let _ = main_window.hide();
                    }
                    app.handle().exit(0);
                    return Ok(());
                }
                Ok(Some(cmd)) => Some(cmd),
                Ok(None) => None,
                Err(msg) => {
                    // Cold path: preserve the historical shell contract,
                    // print to stderr and exit non-zero so scripts detect typos.
                    eprintln!("{}", msg);
                    app.handle().exit(1);
                    return Ok(());
                }
            };

            let model =
                Arc::new(Model::new(app.handle().clone()).expect("Failed to initialize model"));
            app.manage(model);
            app.manage(AudioState::new());

            let mut s = settings::load_settings(app.handle());

            #[cfg(target_os = "macos")]
            {
                use tauri::ActivationPolicy;
                let policy = if s.show_in_dock {
                    ActivationPolicy::Regular
                } else {
                    ActivationPolicy::Accessory
                };
                app.set_activation_policy(policy);
            }

            if matches!(pending_cli_action, Some(cli::CliCommand::Transcribe { .. })) {
                let verbose = std::env::args().any(|a| a == "-v" || a == "--verbose");
                log::set_max_level(if verbose {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Error
                });
            } else if matches!(pending_cli_action, Some(cli::CliCommand::Serve { .. })) {
                let verbose = std::env::args().any(|a| a == "-v" || a == "--verbose");
                log::set_max_level(if verbose {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                });
            } else if let Ok(level) = log::LevelFilter::from_str(&s.log_level) {
                log::set_max_level(level);
            }

            // `--no-dictionary` and `--dictionary-file` only apply to `transcribe`.
            // They leave the stored dictionary untouched, skipping the migration.
            let is_transcribe =
                matches!(pending_cli_action, Some(cli::CliCommand::Transcribe { .. }));
            let no_dictionary = is_transcribe && std::env::args().any(|a| a == "--no-dictionary");
            let dictionary_file = if is_transcribe {
                let args: Vec<String> = std::env::args().collect();
                args.iter()
                    .position(|a| a == "--dictionary-file")
                    .and_then(|i| args.get(i + 1))
                    .cloned()
            } else {
                None
            };

            let dictionary = if no_dictionary {
                Vec::new()
            } else if let Some(path) = &dictionary_file {
                match std::fs::read_to_string(path) {
                    Ok(content) => content
                        .lines()
                        .map(str::trim)
                        .filter(|l| !l.is_empty())
                        .map(str::to_string)
                        .collect(),
                    Err(e) => {
                        eprintln!("Error: cannot read --dictionary-file {}: {}", path, e);
                        app.handle().exit(1);
                        return Ok(());
                    }
                }
            } else if !s.dictionary.is_empty() {
                let dictionary_from_settings = s.dictionary.clone();
                s = settings::remove_dictionary_from_settings(app.handle(), s)?;
                dictionary::migrate_and_load(app.handle(), dictionary_from_settings)?
            } else {
                dictionary::load(app.handle())?
            };
            app.manage(Dictionary::new(dictionary.clone()));

            if let Some(cli::CliCommand::Transcribe { file_path }) = &pending_cli_action {
                if let Some(main_window) = app.get_webview_window("main") {
                    let _ = main_window.hide();
                }
                match audio::pipeline::transcribe_file_chunked(
                    app.handle(),
                    std::path::Path::new(file_path),
                ) {
                    Ok(text) if text.is_empty() => {
                        eprintln!("Transcription produced no text");
                        app.handle().exit(1);
                    }
                    Ok(text) => {
                        println!("{}", text);
                        app.handle().exit(0);
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        app.handle().exit(1);
                    }
                }
                return Ok(());
            }

            app.manage(local_api::LocalApiState::default());
            if let Some(cli::CliCommand::Serve { port }) = &pending_cli_action {
                if let Some(main_window) = app.get_webview_window("main") {
                    let _ = main_window.hide();
                }
                crate::medical::init(app.handle());
                match local_api::serve_headless(app.handle(), *port) {
                    Ok(()) => app.handle().exit(0),
                    Err(e) => {
                        eprintln!("{}", e);
                        app.handle().exit(1);
                    }
                }
                return Ok(());
            }
            if let Err(e) = local_api::apply_settings(app.handle()) {
                error!("Local API not started: {}", e);
            }

            app.manage(remote_mic::session::RemoteMicState::default());
            app.manage(utils::enigo_session::EnigoState::default());

            // Plumed: parse the medical rules now, not on the first transcription
            crate::medical::init(app.handle());

            // Before the startup wizard can ask anything this launch.
            telemetry::count_launch(app.handle());

            let startup_handle = app.handle().clone();
            std::thread::spawn(move || {
                // Open `/dev/uinput` before the model loads, holding the
                // engine lock so the first paste cannot race init.
                // Emits `wayland-inject-unavailable` on failure.
                #[cfg(target_os = "linux")]
                if crate::utils::platform::is_wayland_session() {
                    let state = startup_handle.state::<AudioState>();
                    let _engine_guard = state.engine.lock();
                    if let Err(e) = crate::utils::wayland_inject::init() {
                        warn!("wayland_inject init failed: {}", e);
                        use tauri::Emitter;
                        if let Err(err) = startup_handle.emit("wayland-inject-unavailable", ()) {
                            warn!("failed to emit wayland-inject-unavailable event: {}", err);
                        }
                    }
                    // Char-map failures only degrade Direct paste (chars are
                    // skipped, never pasted), they do not block setup.
                    if let Err(e) = crate::utils::wayland_xkb::init_char_map(&startup_handle) {
                        warn!("wayland_xkb init_char_map failed: {}", e);
                    }
                }

                // Plumed fork: without a downloaded model the rest still runs.
                match preload_engine(&startup_handle) {
                    Ok(_) => info!("Transcription engine initialized and ready"),
                    Err(e) => {
                        info!("Transcription engine will be loaded on first use: {}", e);
                        crate::model::manager::check_after_load_failure(&startup_handle);
                    }
                }
                crate::grammar::warm_up(&startup_handle);
                telemetry::send_if_due(&startup_handle);
                crate::medical::data_update::check_at_startup(&startup_handle);
            });

            setup_tray(app.handle())?;

            overlay::overlay::warmup_overlay(app.handle());
            if s.overlay_mode.as_str() == "always" {
                overlay::overlay::show_recording_overlay(app.handle());
            }

            init_shortcuts(app.handle().clone());

            audio::sound::init_sound_system(app.handle());

            audio::output_volume::restore_pending(app.handle());

            audio::microphone::migrate_mic_id_if_needed(app.handle(), s.mic_id.clone());

            if !is_transcribe {
                crate::utils::log_watchdog::spawn(app.handle());
            }

            if !start_hidden {
                info!("Showing main window (manual launch)");
                show_main_window(app.handle());
            }

            // Cold-start CLI dispatch: apply the command after init so
            // the audio pipeline is ready.
            if let Some(cmd) = pending_cli_action {
                info!("CLI dispatch (cold start): {:?}", cmd);
                let app_handle = app.handle().clone();
                std::thread::spawn(move || {
                    crate::shortcuts::cli_dispatch::dispatch(&app_handle, &cmd);
                });
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                window
                    .state::<crate::shortcuts::types::ShortcutState>()
                    .set_suspended(false);
                window
                    .state::<crate::shortcuts::types::ShortcutState>()
                    .set_capturing(false);
            }
        })
        .invoke_handler(tauri::generate_handler![
            is_model_available,
            get_model_path,
            get_model_status,
            download_model,
            cancel_model_download,
            set_active_model,
            delete_model,
            read_murmure_file,
            write_murmure_file,
            get_all_settings,
            set_show_in_dock,
            get_linux_session_type,
            get_linux_distro_info,
            is_pacman_managed,
            get_output_volume_unsupported_reason,
            dismiss_wayland_notice,
            dismiss_wayland_clipboard_fallback,
            get_recent_transcriptions,
            clear_history,
            get_record_shortcut,
            set_record_shortcut,
            set_dictionary,
            get_dictionary,
            export_dictionary,
            import_dictionary,
            get_last_transcript_shortcut,
            set_last_transcript_shortcut,
            get_cancel_shortcut,
            set_cancel_shortcut,
            cancel_recording,
            set_overlay_mode,
            set_overlay_position,
            set_tray_menu_labels,
            suspend_transcription,
            resume_transcription,
            start_shortcut_capture,
            stop_shortcut_capture,
            set_copy_to_clipboard,
            set_auto_insert,
            set_paste_method,
            get_layout_fallback_state,
            get_usage_stats,
            set_persist_history,
            get_current_language,
            set_current_language,
            get_current_mic_id,
            set_current_mic_id,
            get_current_mic_label,
            get_mic_list,
            set_sound_enabled,
            set_sound_volume,
            set_lower_output_while_recording,
            set_output_volume_while_recording,
            set_record_mode,
            get_formatting_settings,
            set_formatting_settings,
            validate_regex,
            get_medical_rule_edits,
            set_medical_rule_edits,
            search_medical_rules,
            preview_medical_fixes,
            get_orphaned_medical_rule_edits,
            get_default_formatting_rules,
            get_phrase_overlay,
            set_phrase_overlay,
            search_phrases,
            get_confusion_overlay,
            set_confusion_overlay,
            search_confusion_groups,
            get_text_template_overlay,
            set_text_template_overlay,
            search_text_templates,
            get_initial_text_template_overlay,
            get_enabled_text_template_count,
            export_medical_list,
            import_medical_list,
            get_medical_changes,
            import_medical_changes,
            get_grammar_rules,
            set_grammar_rules,
            reset_grammar_rules,
            get_medical_data_status,
            get_medical_data_offer,
            dismiss_medical_data_offer,
            skip_medical_data_version,
            check_medical_data_update,
            download_medical_data_update,
            preview_medical_data_update,
            reset_medical_data,
            set_medical_data_update_check,
            medical_data_question_due,
            take_medical_data_check_failure,
            install_medical_data_files,
            set_log_level,
            set_keep_recordings,
            set_remove_hesitations,
            set_block_cli_import,
            get_local_api_status,
            set_local_api_enabled,
            set_local_api_port,
            regenerate_local_api_token,
            set_short_dictation_context,
            set_remote_mic_url,
            set_remote_mic_enabled,
            remote_mic_start,
            remote_mic_stop,
            remote_mic_confirm,
            get_remote_mic_state,
            remote_mic_prompt_due,
            answer_remote_mic_prompt,
            is_telemetry_available,
            get_telemetry_preview,
            set_telemetry_enabled,
            get_telemetry_question,
            postpone_telemetry_question,
            set_app_update_channel,
            mark_tutorial_opened,
            set_tutorial_completed,
            check_app_update,
            install_app_update,
            get_recordings_dir,
            open_accessibility_settings,
            check_accessibility_permission,
            get_streaming_preview,
            set_streaming_preview,
            set_overlay_size,
            set_streaming_text_settings,
            consume_pending_result,
            ack_result_panel_shown,
            copy_result_text,
            set_result_panel_mode,
            set_result_panel_duration,
            set_overlay_input_region,
            hide_overlay_if_idle
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // macOS: closing the window only hides it (see the
            // CloseRequested handler above). AppKit asks the delegate what
            // to do when the Dock icon is clicked with no visible window;
            // without this the click is a no-op and the only way back to
            // the UI is the tray menu or a full restart.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event {
                show_main_window(app_handle);
            }

            // Explicit UI_DEV_DESTROY on exit, otherwise the device
            // lingers under /proc/bus/input/devices until the kernel
            // reaps us.
            if matches!(event, tauri::RunEvent::Exit) {
                #[cfg(target_os = "linux")]
                crate::utils::wayland_inject::shutdown();
                crate::utils::enigo_session::shutdown(app_handle);
                audio::output_volume::restore_pending(app_handle);
            }
        });
}
