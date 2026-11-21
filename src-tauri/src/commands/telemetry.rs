use crate::telemetry;
use tauri::{command, AppHandle};

/// Whether this build can send usage statistics (website id set at build time).
#[command]
pub fn is_telemetry_available() -> bool {
    telemetry::is_available()
}

/// The exact JSON that would be sent now, pretty-printed.
#[command]
pub fn get_telemetry_preview(app: AppHandle) -> Result<String, String> {
    telemetry::preview(&app)
}

/// Records the user's answer; turning it off deletes the id and the counters.
#[command]
pub fn set_telemetry_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    telemetry::set_enabled(&app, enabled)
}
