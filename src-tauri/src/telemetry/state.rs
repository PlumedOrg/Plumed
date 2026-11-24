//! The telemetry state, in its own file of the app data folder so turning
//! telemetry off can delete it entirely (id and counters).

use super::counters::Counters;
use chrono::{Months, NaiveDate};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

const STATE_FILE: &str = "telemetry.json";

/// Lifetime of the random id (CNIL: at most 13 months, never extended on use).
const ID_LIFETIME_MONTHS: u32 = 13;

/// Serializes the read-modify-write cycles of the transcription thread and
/// of the sending task.
static FILE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TelemetryState {
    pub id: String,
    pub id_created: NaiveDate,
    pub installed: NaiveDate,
    pub last_sent: Option<NaiveDate>,
    #[serde(default)]
    pub counters: Counters,
}

impl TelemetryState {
    pub fn new(today: NaiveDate) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            id_created: today,
            installed: today,
            last_sent: None,
            counters: Counters::default(),
        }
    }

    /// Draws a new random id once the current one is 13 months old.
    pub fn renew_id_if_expired(&mut self, today: NaiveDate) {
        let expires = self
            .id_created
            .checked_add_months(Months::new(ID_LIFETIME_MONTHS));
        if expires.is_none_or(|expires| today >= expires) {
            self.id = uuid::Uuid::new_v4().to_string();
            self.id_created = today;
        }
    }

    /// At most one send per calendar day, and only after the app was used.
    pub fn send_due(&self, today: NaiveDate) -> bool {
        !self.counters.is_empty() && self.last_sent.is_none_or(|last| last < today)
    }

    pub fn days_since_install(&self, today: NaiveDate) -> i64 {
        (today - self.installed).num_days().max(0)
    }
}

fn state_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(STATE_FILE))
}

fn read(path: &PathBuf) -> Option<TelemetryState> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// The saved state (a fresh one when missing or unreadable, not saved).
pub fn load(app: &AppHandle, today: NaiveDate) -> Result<TelemetryState, String> {
    let path = state_path(app)?;
    let mut state = read(&path).unwrap_or_else(|| TelemetryState::new(today));
    state.renew_id_if_expired(today);
    Ok(state)
}

/// Applies `change` to the saved state (created when missing) and saves it.
pub fn update(
    app: &AppHandle,
    today: NaiveDate,
    change: impl FnOnce(&mut TelemetryState),
) -> Result<TelemetryState, String> {
    let _guard = FILE_LOCK.lock().map_err(|e| e.to_string())?;
    let mut state = load(app, today)?;
    change(&mut state);
    let content = serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?;
    fs::write(state_path(app)?, content).map_err(|e| e.to_string())?;
    Ok(state)
}

/// Deletes the id and the counters.
pub fn delete(app: &AppHandle) -> Result<(), String> {
    let _guard = FILE_LOCK.lock().map_err(|e| e.to_string())?;
    match fs::remove_file(state_path(app)?) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::counters::Source;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn id_is_renewed_after_13_months_only() {
        let mut state = TelemetryState::new(date(2026, 1, 15));
        let first = state.id.clone();
        state.renew_id_if_expired(date(2027, 2, 14));
        assert_eq!(state.id, first);
        assert_eq!(state.id_created, date(2026, 1, 15));

        state.renew_id_if_expired(date(2027, 2, 15));
        assert_ne!(state.id, first);
        assert_eq!(state.id_created, date(2027, 2, 15));
        assert_eq!(state.installed, date(2026, 1, 15));
    }

    #[test]
    fn sends_once_per_day_and_only_after_use() {
        let today = date(2026, 11, 21);
        let mut state = TelemetryState::new(date(2026, 11, 1));
        assert!(!state.send_due(today), "no usage, nothing to send");

        state
            .counters
            .add_transcription(Source::Dictation, 12.0, 80, 1.0);
        assert!(state.send_due(today));

        state.last_sent = Some(today);
        assert!(!state.send_due(today), "already sent today");
        assert!(state.send_due(date(2026, 11, 22)));
    }

    #[test]
    fn days_since_install_counts_calendar_days() {
        let state = TelemetryState::new(date(2026, 11, 1));
        assert_eq!(state.days_since_install(date(2026, 11, 21)), 20);
        assert_eq!(state.days_since_install(date(2026, 10, 1)), 0);
    }
}
