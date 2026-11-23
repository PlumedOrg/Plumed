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

/// Whether the startup wizard asks about usage statistics now (never once
/// answered, and after "Plus tard" only when the re-ask counters allow it).
#[command]
pub fn get_telemetry_question(app: AppHandle) -> Result<telemetry::Question, String> {
    telemetry::question(&app)
}

/// "Plus tard": asks again later, see `telemetry::reask`.
#[command]
pub fn postpone_telemetry_question(app: AppHandle) -> Result<(), String> {
    telemetry::decline_for_now(&app)
}

/// Records the user's answer; turning it off deletes the id and the counters.
#[command]
pub fn set_telemetry_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    telemetry::set_enabled(&app, enabled)
}
