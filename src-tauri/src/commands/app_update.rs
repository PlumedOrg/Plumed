//! App updates (Plumed fork, see `app_update`). Written with Claude Code.

use crate::app_update::{self, UpdateChannel};
use tauri::ipc::Channel;
use tauri::{command, AppHandle};

/// Saves the channel (Settings > Online, and the startup wizard).
#[command]
pub fn set_app_update_channel(app: AppHandle, channel: UpdateChannel) -> Result<(), String> {
    app_update::set_channel(&app, channel)
}

/// The newer version on the user's channel, if any. The automatic check at
/// startup (`manual` false) does nothing when updates are disabled.
#[command]
pub async fn check_app_update(app: AppHandle, manual: bool) -> Result<Option<String>, String> {
    app_update::check(&app, manual).await
}

/// Installs the update the last check found, sending the download percent.
#[command]
pub async fn install_app_update(on_progress: Channel<u8>) -> Result<(), String> {
    app_update::install(|percent| {
        if let Err(e) = on_progress.send(percent) {
            log::warn!("Failed to send update progress: {e}");
        }
    })
    .await
}
