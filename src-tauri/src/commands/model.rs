use crate::model::download::DownloadError;
use crate::model::manager::{self, ModelStatus};
use crate::model::Model;
use std::sync::Arc;
use tauri::{command, AppHandle, State};

#[command]
pub fn is_model_available(model: State<Arc<Model>>) -> bool {
    model.is_available()
}

#[command]
pub fn get_model_path(model: State<Arc<Model>>) -> Result<String, String> {
    let path = model.get_model_path().map_err(|e| format!("{:#}", e))?;

    Ok(path.to_string_lossy().to_string())
}

// Plumed fork (written with Claude Code): the downloadable models.

#[command]
pub fn get_model_status(app: AppHandle) -> ModelStatus {
    manager::status(&app)
}

#[command]
pub async fn download_model(app: AppHandle, variant: String) -> Result<ModelStatus, DownloadError> {
    manager::download(&app, &variant).await
}

#[command]
pub fn cancel_model_download() {
    manager::cancel();
}

#[command]
pub async fn set_active_model(app: AppHandle, variant: String) -> Result<ModelStatus, String> {
    manager::set_active(&app, &variant).await
}

#[command]
pub async fn delete_model(app: AppHandle, variant: String) -> Result<ModelStatus, String> {
    manager::delete(&app, &variant).await
}

/// The speaker diarization model of the Réunion page, downloaded on first use.
#[command]
pub fn get_diarization_model_status(app: AppHandle) -> manager::VariantStatus {
    manager::auxiliary_status(&app, &crate::model::catalog::DIARIZATION)
}

#[command]
pub async fn download_diarization_model(
    app: AppHandle,
) -> Result<manager::VariantStatus, DownloadError> {
    manager::download_auxiliary(&app, &crate::model::catalog::DIARIZATION).await
}
