//! Plumed fork (written with Claude Code): Tauri commands of the Réunion page.
//! Loading the diarization model and waiting for a previous recording block,
//! so they run off the main thread.

use crate::meeting::state::{MeetingSnapshot, MeetingState};
use std::path::PathBuf;
use tauri::{command, AppHandle, Manager};

async fn blocking<T: Send + 'static>(
    app: AppHandle,
    f: impl FnOnce(&AppHandle, &MeetingState) -> anyhow::Result<T> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || f(&app, &app.state::<MeetingState>()))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

#[command]
pub async fn meeting_start_mic(app: AppHandle) -> Result<(), String> {
    blocking(app, |app, state| state.start_mic(app)).await
}

#[command]
pub async fn meeting_start_file(app: AppHandle, path: PathBuf) -> Result<(), String> {
    blocking(app, move |app, state| state.start_file(app, &path)).await
}

#[command]
pub async fn meeting_stop(app: AppHandle) -> Result<(), String> {
    blocking(app, |_, state| {
        state.stop();
        Ok(())
    })
    .await
}

#[command]
pub async fn meeting_reset(app: AppHandle) -> Result<(), String> {
    blocking(app, |_, state| {
        state.reset();
        Ok(())
    })
    .await
}

#[command]
pub async fn meeting_snapshot(app: AppHandle) -> Result<MeetingSnapshot, String> {
    blocking(app, |_, state| Ok(state.snapshot())).await
}

/// PhoneMic recordings go to the meeting (instead of being pasted) until
/// `meeting_stop`.
#[command]
pub async fn meeting_start_phone(app: AppHandle) -> Result<(), String> {
    blocking(app, |app, state| state.start_phone(app)).await
}
