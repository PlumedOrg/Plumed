//! Plumed fork: the automatic update checks (app releases, medical rules),
//! after the model loaded and then once a day while the app stays open, so
//! a user who never quits Plumed still gets them. Each check keeps its own
//! opt-in (update channel, medical rules toggle). Only the GUI app starts
//! this: the headless `transcribe` and `serve` commands return before
//! (`lib.rs`). Written with Claude Code.

use log::warn;
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Emitter};

const PERIOD: Duration = Duration::from_secs(24 * 60 * 60);
/// Wall-clock polling rather than one 24 h sleep: a sleep does not count
/// the time the computer is suspended.
const TICK: Duration = Duration::from_secs(60 * 60);
const APP_UPDATE_EVENT: &str = "app-update-available";

pub fn spawn(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut last = None;
        loop {
            let now = SystemTime::now();
            if is_due(last, now) {
                last = Some(now);
                crate::medical::data_update::check_automatically(&app);
                check_app_update(&app).await;
            }
            tokio::time::sleep(TICK).await;
        }
    });
}

/// Due at the first tick, then a day after the last run (or at once if the
/// clock went back).
fn is_due(last: Option<SystemTime>, now: SystemTime) -> bool {
    last.is_none_or(|t| now.duration_since(t).map_or(true, |d| d >= PERIOD))
}

/// Tells the frontend (sidebar button) when the user's channel has a newer
/// version. Nothing when updates are disabled.
async fn check_app_update(app: &AppHandle) {
    match crate::app_update::check(app, false).await {
        Ok(Some(version)) => {
            if let Err(e) = app.emit(APP_UPDATE_EVENT, version) {
                warn!("Could not announce the app update: {e}");
            }
        }
        Ok(None) => {}
        Err(e) => warn!("App update check failed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn due_at_start_then_once_a_day() {
        let t0 = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        assert!(is_due(None, t0));
        assert!(!is_due(Some(t0), t0 + PERIOD - TICK));
        assert!(is_due(Some(t0), t0 + PERIOD));
    }

    #[test]
    fn due_when_the_clock_went_back() {
        let t0 = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        assert!(is_due(Some(t0), t0 - TICK));
    }
}
