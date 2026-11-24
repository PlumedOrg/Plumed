//! Medical data updates (Plumed fork, see `medical::data_update`). Written
//! with Claude Code.

use crate::medical::data_set::{diff::FileDiff, SetError};
use crate::medical::data_update::{self, DataChange, DataStatus, UpdateOffer};
use std::path::PathBuf;
use tauri::{command, AppHandle};

#[command]
pub async fn get_medical_data_status(app: AppHandle) -> Result<DataStatus, String> {
    data_update::status(&app).await
}

/// The update found by the automatic check, until the user answers it.
#[command]
pub fn get_medical_data_offer() -> Option<UpdateOffer> {
    data_update::startup_offer()
}

/// "Not now": the automatic check asks again (next launch, or a day later).
#[command]
pub fn dismiss_medical_data_offer() {
    data_update::dismiss_offer()
}

#[command]
pub fn skip_medical_data_version(app: AppHandle, version: String) -> Result<(), String> {
    data_update::skip_version(&app, &version)
}

/// "Check now": `None` when the active data is the newest.
#[command]
pub async fn check_medical_data_update() -> Result<Option<UpdateOffer>, String> {
    data_update::check_now().await
}

/// Downloads and activates the offered `version`, only ever on a click.
#[command]
pub async fn download_medical_data_update(
    app: AppHandle,
    version: String,
) -> Result<DataChange, String> {
    data_update::download(&app, &version).await
}

/// Downloads the offered `version` without installing it and lists what it
/// changes, for the optional review in the update dialog.
#[command]
pub async fn preview_medical_data_update(
    app: AppHandle,
    version: String,
) -> Result<Vec<FileDiff>, String> {
    data_update::preview(&app, &version).await
}

/// Deletes the downloaded data and goes back to the bundled data.
#[command]
pub async fn reset_medical_data(app: AppHandle) -> Result<DataChange, String> {
    data_update::reset(&app).await
}

/// The first launch answer or the settings toggle.
#[command]
pub fn set_medical_data_update_check(app: AppHandle, enabled: bool) -> Result<(), String> {
    data_update::set_check_enabled(&app, enabled)
}

/// Whether to ask if the check may run at each start (not answered yet).
#[command]
pub fn medical_data_question_due(app: AppHandle) -> bool {
    data_update::question_due_now(&app)
}

/// Why the automatic check failed, once per launch.
#[command]
pub fn take_medical_data_check_failure() -> Option<String> {
    data_update::take_check_failure()
}

/// "Install a downloaded update" from the files the user picked.
#[command]
pub async fn install_medical_data_files(
    app: AppHandle,
    paths: Vec<PathBuf>,
) -> Result<DataChange, SetError> {
    data_update::install_picked(&app, paths).await
}
