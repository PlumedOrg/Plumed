//! When the usage statistics question comes back after "Plus tard".
//!
//! Plain local counters in `telemetry_ask.json` (declines, date of the last
//! one, launches and dictations since), never sent. The file only exists
//! after a first "Plus tard" and any definitive answer deletes it.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

const STATE_FILE: &str = "telemetry_ask.json";

/// The question comes back once all three minimums are reached since the
/// last "Plus tard".
const MIN_DAYS: i64 = 7;
const MIN_TRANSCRIPTIONS: u32 = 10;
const MIN_LAUNCHES: u32 = 3;

/// From this many declines on, the question also offers "Ne plus jamais
/// demander" (so on the third time it is asked).
const NEVER_OFFER_AFTER_DECLINES: u32 = 2;

static FILE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct AskState {
    declined: u32,
    last_declined: Option<NaiveDate>,
    launches: u32,
    transcriptions: u32,
}

impl AskState {
    /// Whether the question is asked now: always before the first
    /// "Plus tard", then only once 7 days, 10 dictations and 3 launches
    /// passed since the last one.
    pub fn due(&self, today: NaiveDate) -> bool {
        match self.last_declined {
            None => true,
            Some(last) => {
                (today - last).num_days() >= MIN_DAYS
                    && self.transcriptions >= MIN_TRANSCRIPTIONS
                    && self.launches >= MIN_LAUNCHES
            }
        }
    }

    /// Whether this ask also offers "Ne plus jamais demander".
    pub fn offers_never_again(&self) -> bool {
        self.declined >= NEVER_OFFER_AFTER_DECLINES
    }

    pub fn decline(&mut self, today: NaiveDate) {
        self.declined = self.declined.saturating_add(1);
        self.last_declined = Some(today);
        self.launches = 0;
        self.transcriptions = 0;
    }

    fn count_launch(&mut self) {
        self.launches = self.launches.saturating_add(1);
    }

    fn count_transcription(&mut self) {
        self.transcriptions = self.transcriptions.saturating_add(1);
    }
}

/// What the startup wizard needs to know.
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
pub struct Question {
    pub due: bool,
    pub offer_never_again: bool,
}

fn path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(STATE_FILE))
}

fn read(path: &PathBuf) -> Option<AskState> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

/// The saved state (the default one when missing or unreadable).
pub fn load(app: &AppHandle) -> Result<AskState, String> {
    Ok(read(&path(app)?).unwrap_or_default())
}

fn save(path: &PathBuf, state: &AskState) -> Result<(), String> {
    let content = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

/// Records a "Plus tard".
pub fn decline(app: &AppHandle, today: NaiveDate) -> Result<(), String> {
    let _guard = FILE_LOCK.lock().map_err(|e| e.to_string())?;
    let path = path(app)?;
    let mut state = read(&path).unwrap_or_default();
    state.decline(today);
    save(&path, &state)
}

/// Counts a launch or a dictation, only once the user declined (before, the
/// question is asked anyway, so nothing is written).
fn count(app: &AppHandle, change: impl FnOnce(&mut AskState)) {
    let Ok(_guard) = FILE_LOCK.lock() else {
        return;
    };
    let Ok(path) = path(app) else {
        return;
    };
    let Some(mut state) = read(&path) else {
        return;
    };
    change(&mut state);
    if let Err(e) = save(&path, &state) {
        log::warn!("Could not save the usage statistics question counters: {e}");
    }
}

pub fn count_launch(app: &AppHandle) {
    count(app, AskState::count_launch)
}

pub fn count_transcription(app: &AppHandle) {
    count(app, AskState::count_transcription)
}

/// Deletes the counters (a definitive answer was given).
pub fn delete(app: &AppHandle) -> Result<(), String> {
    let _guard = FILE_LOCK.lock().map_err(|e| e.to_string())?;
    match fs::remove_file(path(app)?) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    /// Declined on 2026-11-01, then `launches` launches and `transcriptions` dictations.
    fn declined(launches: u32, transcriptions: u32) -> AskState {
        let mut state = AskState::default();
        state.decline(date(2026, 11, 1));
        state.launches = launches;
        state.transcriptions = transcriptions;
        state
    }

    #[test]
    fn asked_until_the_first_later() {
        assert!(AskState::default().due(date(2026, 11, 1)));
    }

    #[test]
    fn comes_back_only_when_days_dictations_and_launches_all_reached() {
        let in_8_days = date(2026, 11, 9);
        assert!(declined(3, 10).due(in_8_days));
        assert!(!declined(2, 10).due(in_8_days), "2 launches");
        assert!(!declined(3, 9).due(in_8_days), "9 dictations");
        assert!(!declined(3, 10).due(date(2026, 11, 7)), "6 days");
        assert!(declined(3, 10).due(date(2026, 11, 8)), "exactly 7 days");
    }

    #[test]
    fn declining_resets_the_counters() {
        let mut state = declined(5, 50);
        state.decline(date(2026, 11, 20));
        assert_eq!((state.launches, state.transcriptions), (0, 0));
        assert!(!state.due(date(2026, 11, 21)));
    }

    #[test]
    fn never_again_is_offered_the_third_time() {
        let mut state = AskState::default();
        assert!(!state.offers_never_again(), "first ask");
        state.decline(date(2026, 11, 1));
        assert!(!state.offers_never_again(), "second ask");
        state.decline(date(2026, 11, 20));
        assert!(state.offers_never_again(), "third ask");
        assert!(!state.due(date(2026, 11, 21)));
    }

    #[test]
    fn counters_only_go_up() {
        let mut state = AskState::default();
        state.count_launch();
        state.count_transcription();
        state.count_transcription();
        assert_eq!((state.launches, state.transcriptions), (1, 2));
    }
}
