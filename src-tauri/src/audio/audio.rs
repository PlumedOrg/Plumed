use crate::audio::helpers::{cleanup_recordings, ensure_recordings_dir, generate_unique_wav_name};
use crate::audio::pipeline::DictationAudio;
use crate::audio::recorder::AudioRecorder;
use crate::audio::types::{AudioState, RecorderStartError};
use crate::audio::ChunkPipeline;
use crate::clipboard;
use crate::engine::ParakeetEngine;
use crate::model::Model;
use crate::overlay::overlay;
use anyhow::Result;
use log::{debug, error, info, warn};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

pub fn record_audio(app: &AppHandle) {
    // Plumed fork: the model is downloaded on first launch, say so instead
    // of failing at the end of the dictation.
    if !app.state::<Arc<Model>>().is_available() {
        warn!("Recording refused: no speech model downloaded");
        crate::show_main_window(app);
        let _ = app.emit("model-missing", ());
        return;
    }
    // Plumed fork (written with Claude Code): the Réunion page uses the model
    // and the microphone, a dictation would compete with it.
    if app
        .state::<crate::meeting::state::MeetingState>()
        .is_running()
    {
        warn!("Recording refused: a meeting is being transcribed");
        show_recording_notification(app, "meeting-busy", String::new());
        return;
    }
    let state = app.state::<AudioState>();
    let my_gen = state.begin_session();
    state.set_session_active(true);

    let settings = crate::settings::load_settings(app);
    let preview =
        crate::audio::chunking::PreviewLink::from_state(&state, settings.streaming_preview);
    *state.chunk_pipeline.lock() = Some(ChunkPipeline::start(app, preview));

    internal_record_audio(app, my_gen);
}

fn internal_record_audio(app: &AppHandle, my_gen: u64) {
    debug!("Starting audio recording...");
    let state = app.state::<AudioState>();

    overlay::release_overlay_for_recording(app);
    crate::audio::sound::prewarm(app);
    let prewarm_at = std::time::Instant::now();

    match start_new_recorder(app, true, my_gen, prewarm_at) {
        Ok(sample_rate) => {
            crate::overlay::tray::set_tray_recording(app);
            crate::audio::streaming::start_streaming(app, &state, sample_rate);
        }
        Err(RecorderStartError::InitFailed) => {
            state.set_session_active(false);
            notify_recording_error(app);
        }
        Err(RecorderStartError::Superseded) => {
            state.set_session_active(false);
            let _ = state.chunk_pipeline.lock().take();
            overlay::hide_overlay_if_idle(app);
            debug!("Recorder start superseded by a concurrent stop; aborted");
        }
        // Another session still owns the recorder, so it also owns the session
        // flag and the overlay: its own stop path releases both.
        Err(RecorderStartError::Busy) => {}
        Err(_) => {
            state.set_session_active(false);
            overlay::hide_overlay_if_idle(app);
        }
    }
}

fn start_new_recorder(
    app: &AppHandle,
    play_sound: bool,
    my_gen: u64,
    prewarm_at: std::time::Instant,
) -> Result<u32, RecorderStartError> {
    let state = app.state::<AudioState>();

    let recordings_dir = match ensure_recordings_dir(app) {
        Ok(dir) => dir,
        Err(e) => {
            error!("Failed to initialize recordings directory: {}", e);
            return Err(RecorderStartError::DirUnavailable);
        }
    };

    let file_name = generate_unique_wav_name();
    let file_path = recordings_dir.join(&file_name);

    // Hold the lock across check-and-install to serialize concurrent callers.
    let mut recorder_guard = state.recorder.lock();
    if state.current_session() != my_gen {
        debug!("Recorder start superseded before install; aborting");
        return Err(RecorderStartError::Superseded);
    }
    if recorder_guard.is_some() {
        warn!("Already recording");
        return Err(RecorderStartError::Busy);
    }

    let mic_open_start = std::time::Instant::now();
    let mut recorder = match AudioRecorder::new(app.clone(), &file_path) {
        Ok(recorder) => recorder,
        Err(e) => {
            error!("Failed to initialize recorder: {}", e);
            let _ = std::fs::remove_file(&file_path);
            return Err(RecorderStartError::InitFailed);
        }
    };
    let mic_open = mic_open_start.elapsed();

    if state.current_session() == my_gen {
        overlay::begin_overlay_session(app);
    }

    let beep_wait = match recorder.start(play_sound, prewarm_at) {
        Ok(beep_wait) => beep_wait,
        Err(e) => {
            error!("Failed to start recording: {}", e);
            let _ = std::fs::remove_file(&file_path);
            return Err(RecorderStartError::StartFailed);
        }
    };
    debug!(
        "Recording started (mic open {} ms, beep wait {} ms)",
        mic_open.as_millis(),
        beep_wait.as_millis()
    );

    let sample_rate = recorder.sample_rate();
    *state.current_file_name.lock() = Some(file_name);
    *recorder_guard = Some(recorder);
    Ok(sample_rate)
}

fn show_recording_notification(app: &AppHandle, event: &'static str, payload: String) {
    overlay::show_recording_overlay(app);
    let app_clone = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let _ = app_clone.emit(event, payload);
        std::thread::sleep(std::time::Duration::from_millis(1500));
        overlay::hide_recording_overlay(&app_clone);
    });
}

fn notify_recording_error(app: &AppHandle) {
    crate::telemetry::count_error(app, crate::telemetry::ErrorKind::Microphone);
    let s = crate::settings::load_settings(app);
    let mic_name = s.mic_label.or(s.mic_id).unwrap_or_default();
    show_recording_notification(app, "recording-error", mic_name);
}

pub fn stop_recording(app: &AppHandle) -> Option<std::path::PathBuf> {
    debug!("Stopping audio recording...");
    let state = app.state::<AudioState>();
    state.invalidate_session();
    let my_gen = state.current_session();

    crate::audio::sound::prewarm(app);

    {
        let mut recorder_guard = state.recorder.lock();
        if let Some(recorder) = recorder_guard.as_mut() {
            recorder.close_input();
        }
    }

    crate::audio::sound::play_sound(app, crate::audio::sound::Sound::StopRecording);
    crate::audio::streaming::stop_streaming(&state);

    // Stopping the recorder drains the writer thread, so every chunk (including
    // the final remainder it flushes) is queued before the pipeline finalizes.
    {
        let mut recorder_guard = state.recorder.lock();
        if let Some(recorder) = recorder_guard.as_mut() {
            if let Err(e) = recorder.stop(false) {
                error!("Failed to stop recorder: {}", e);
            }
        }
        *recorder_guard = None;
    }

    let pipeline = state.chunk_pipeline.lock().take();

    let file_name_opt = state.current_file_name.lock().take();
    let path = ensure_recordings_dir(app)
        .ok()
        .zip(file_name_opt)
        .map(|(dir, name)| dir.join(name));

    match (path.as_ref(), pipeline) {
        (Some(p), Some(pipeline)) => {
            info!(
                "Audio recording stopped; file written to temporary path: {}",
                p.display()
            );
            finalize_chunked_session(app, &state, pipeline, p, my_gen);
        }
        _ => {
            debug!("Recording stopped (no active file or pipeline)");
            if state.current_session() == my_gen {
                state.set_session_active(false);
            }
            reset_recording_ui(app);
        }
    }

    path
}

fn finalize_chunked_session(
    app: &AppHandle,
    state: &AudioState,
    pipeline: ChunkPipeline,
    path: &std::path::Path,
    my_gen: u64,
) {
    let _ = app.emit("transcription-processing-start", ());
    let processing_started = std::time::Instant::now();
    let chunks = pipeline.finalize();

    let mut trace = crate::history::StepTrace::on();
    let text = crate::audio::pipeline::post_process_chunks(app, chunks, &mut trace);
    // Plumed fork (written with Claude Code): the overlay spinner lasts until
    // the corrected text is ready, not only the speech recognition.
    let _ = app.emit("transcription-processing-end", ());
    crate::audio::pipeline::record_dictation(
        app,
        &text,
        trace.into_steps(),
        DictationAudio::from_wav(path),
        processing_started.elapsed(),
    );
    if let Err(e) = write_transcription(app, &text) {
        error!("Failed to use clipboard: {}", e);
    }
    let is_current = state.current_session() == my_gen;
    if is_current && text.trim().is_empty() {
        show_no_speech_heard(app);
    } else {
        if is_current {
            overlay::show_result_panel_if_allowed(app, &text);
        }
        reset_recording_ui(app);
    }

    // A newer session took ownership while this one was post-processing: it now
    // owns the flag and releases it on its own stop path.
    if state.current_session() == my_gen {
        state.set_session_active(false);
    }
}

pub fn cancel_recording(app: &AppHandle) {
    info!("Cancelling audio recording...");
    let state = app.state::<AudioState>();
    state.invalidate_session();

    crate::audio::sound::prewarm(app);
    crate::audio::streaming::stop_streaming(&state);

    // Stop recorder without processing
    {
        let mut recorder_guard = state.recorder.lock();
        if let Some(recorder) = recorder_guard.as_mut() {
            if let Err(e) = recorder.stop(true) {
                error!("Failed to stop recorder on cancel: {}", e);
            }
        }
        *recorder_guard = None;
    }

    // Cancel before drop: the flag suppresses any freeze-segment the worker would
    // still emit while draining the queue, preventing ghost text in a next session.
    if let Some(pipeline) = state.chunk_pipeline.lock().take() {
        pipeline.cancel();
    }

    // Remove temporary WAV file
    let file_name_opt = state.current_file_name.lock().take();
    if let Some(file_name) = file_name_opt {
        if let Ok(dir) = ensure_recordings_dir(app) {
            let path = dir.join(&file_name);
            if let Err(e) = std::fs::remove_file(&path) {
                error!("Failed to remove cancelled recording file: {}", e);
            }
        }
    }

    state.set_session_active(false);
    reset_recording_ui(app);
    info!("Recording cancelled by user");
}

/// How long the overlay says that nothing was heard before it hides.
const NO_SPEECH_MESSAGE_MS: u64 = 2500;

/// Plumed fork (written with Claude Code): a dictation that gave no text (a
/// muted microphone, silence) says so in the overlay instead of vanishing.
/// The overlay hides afterwards unless a new dictation started meanwhile.
fn show_no_speech_heard(app: &AppHandle) {
    reset_recording_indicators(app);
    let _ = app.emit("no-speech-heard", ());
    let session = app.state::<AudioState>().current_session();
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(NO_SPEECH_MESSAGE_MS));
        // a newer dictation owns the overlay (and maybe its result card)
        if app.state::<AudioState>().current_session() == session {
            overlay::hide_overlay_if_idle(&app);
        }
    });
}

fn reset_recording_indicators(app: &AppHandle) {
    let _ = app.emit("mic-level", 0.0f32);
    crate::overlay::tray::set_tray_idle(app);
    crate::audio::streaming::reset_overlay_preview(app);
}

fn reset_recording_ui(app: &AppHandle) {
    reset_recording_indicators(app);
    let s = crate::settings::load_settings(app);
    if s.overlay_mode.as_str() == "recording" && !overlay::has_pending_result() {
        overlay::hide_recording_overlay(app);
    }
}

pub fn write_transcription(app: &AppHandle, transcription: &str) -> Result<()> {
    if transcription.trim().is_empty() {
        debug!("Empty transcription, skipping paste");
        if let Err(e) = cleanup_recordings(app) {
            error!("Failed to cleanup recordings: {}", e);
        }
        return Ok(());
    }

    // Wayland: hide before paste so KWin/Mutter returns focus before Ctrl+V.
    // paste_with_delay's 400 ms settle relies on millis_since_last_overlay_hide.
    #[cfg(target_os = "linux")]
    {
        if crate::utils::platform::is_wayland_session() {
            let s = crate::settings::load_settings(app);
            if s.auto_insert && s.overlay_mode.as_str() == "recording" {
                overlay::hide_recording_overlay(app);
            }
        }
    }

    if let Err(e) = clipboard::paste(transcription, app) {
        error!("Failed to paste text: {}", e);
        crate::telemetry::count_error(app, crate::telemetry::ErrorKind::Paste);
    }

    if let Err(e) = cleanup_recordings(app) {
        error!("Failed to cleanup recordings: {}", e);
    }

    debug!("Transcription written ({} bytes)", transcription.len());
    Ok(())
}

pub fn write_last_transcription(app: &AppHandle, transcription: &str) -> Result<()> {
    if let Err(e) = clipboard::paste_last_transcript(transcription, app) {
        error!("Failed to paste last transcription: {}", e);
    }

    debug!("Last transcription written ({} bytes)", transcription.len());
    Ok(())
}

/// Drops the loaded engine (Plumed fork: the model in use changed).
pub fn unload_engine(app: &AppHandle) {
    *app.state::<AudioState>().engine.lock() = None;
}

pub fn preload_engine(app: &AppHandle) -> Result<()> {
    let state = app.state::<AudioState>();
    let mut engine = state.engine.lock();

    if engine.is_none() {
        let model = app.state::<Arc<Model>>();
        let model_path = model
            .get_model_path()
            .map_err(|e| anyhow::anyhow!("Failed to get model path: {}", e))?;

        let mut new_engine = ParakeetEngine::load_int8(&model_path, model.get_tokenizer_path())
            .map_err(|e| {
                crate::telemetry::count_error(app, crate::telemetry::ErrorKind::ModelLoad);
                anyhow::anyhow!("Failed to load model: {}", e)
            })?;
        crate::medical::engine_lists::arm(app, &mut new_engine);

        *engine = Some(new_engine);
        info!("Model loaded and cached in memory");
    }

    Ok(())
}
