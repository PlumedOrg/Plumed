//! Opt-in anonymous usage statistics (Plumed fork only, not for upstream).
//!
//! Nothing is counted or sent unless the user said yes. Counters are
//! aggregated per period (see `counters`), sent at most once per calendar day
//! and only on days the app is used, to the maintainer's self-hosted Umami.
//! Builds without `PLUMED_TELEMETRY_WEBSITE_ID` are inert: no dialog, no
//! setting, nothing sent. Written with Claude Code.

mod counters;
mod payload;
mod state;

pub use counters::ErrorKind;

use crate::formatting_rules::BuiltInOptions;
use crate::settings::AppSettings;
use chrono::{Local, NaiveDate};
use log::{info, warn};
use payload::{count_bucket, known_value, Environment, FeatureFlags};
use serde_json::Value;
use state::TelemetryState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Manager};

const DEFAULT_ENDPOINT: &str = "https://metrics.plumed.fr/api/send";

/// Umami 3 collect endpoint, set at build time.
const ENDPOINT: &str = match option_env!("PLUMED_TELEMETRY_URL") {
    Some(url) => url,
    None => DEFAULT_ENDPOINT,
};

/// Umami website id, set at build time (`PLUMED_TELEMETRY_WEBSITE_ID`, a
/// repository secret in the release workflows). Unset means inert.
const WEBSITE_ID: Option<&str> = option_env!("PLUMED_TELEMETRY_WEBSITE_ID");

const SEND_TIMEOUT: Duration = Duration::from_secs(15);

/// A due send waits a random delay up to this long, so its time does not
/// reveal when the app was started, opted in, or used.
const MAX_SEND_DELAY_SECS: u128 = 15 * 60;

/// Uniform random delay in `[0, MAX_SEND_DELAY_SECS)`, from a v4 UUID's random bits.
fn random_send_delay() -> Duration {
    Duration::from_secs((uuid::Uuid::new_v4().as_u128() % MAX_SEND_DELAY_SECS) as u64)
}

/// One send at a time (startup and end of a transcription may race).
static SENDING: AtomicBool = AtomicBool::new(false);

fn website_id() -> Option<&'static str> {
    WEBSITE_ID.filter(|id| !id.trim().is_empty())
}

/// Whether this build can send usage statistics at all.
pub fn is_available() -> bool {
    website_id().is_some()
}

fn is_enabled(app: &AppHandle) -> bool {
    is_available() && crate::settings::load_settings(app).telemetry_enabled
}

fn today() -> NaiveDate {
    Local::now().date_naive()
}

fn environment(app: &AppHandle, settings: &AppSettings) -> Environment {
    Environment {
        version: app.package_info().version.to_string(),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        install_kind: tauri::utils::platform::bundle_type().map(|kind| kind.to_string()),
        language: known_value(&settings.language, &["default", "en", "fr"]).to_string(),
        model: crate::model::MODEL_FILENAME,
    }
}

fn feature_flags(app: &AppHandle, settings: &AppSettings) -> FeatureFlags {
    let (rule_count, built_in) = crate::formatting_rules::load(app)
        .map(|formatting| (formatting.rules.len(), formatting.built_in))
        .unwrap_or((0, BuiltInOptions::default()));
    let dictionary_size = app.state::<crate::dictionary::Dictionary>().get().len();
    FeatureFlags {
        user_rules: count_bucket(rule_count, &[(10, "1-10"), (50, "11-50")], ">50"),
        medical_rules_edited: crate::medical::get_edits(app).is_ok_and(|e| !e.is_empty()),
        dictionary_words: count_bucket(dictionary_size, &[(10, "1-10"), (100, "11-100")], ">100"),
        record_mode: known_value(&settings.record_mode, &["push_to_talk", "toggle_to_talk"]),
        remote_mic: settings.remote_mic_enabled,
        confusion_rescoring: built_in.confusion_rescoring,
        phrase_boosting: built_in.phrase_boosting,
    }
}

/// The request body and User-Agent for `state`.
fn request(
    app: &AppHandle,
    website_id: &str,
    state: &TelemetryState,
    today: NaiveDate,
) -> ([Value; 2], String) {
    let settings = crate::settings::load_settings(app);
    let env = environment(app, &settings);
    let data = payload::event_data(
        &env,
        &feature_flags(app, &settings),
        &state.counters,
        state.days_since_install(today),
    );
    let bodies = [
        payload::umami_pageview(website_id, &state.id, &env.language),
        payload::umami_body(website_id, &state.id, &env.language, data),
    ];
    (bodies, payload::user_agent(&env))
}

/// Counts a finished transcription (`processing` is the time from the end
/// of the recording to the final text). Does nothing unless enabled.
pub fn record_transcription(app: &AppHandle, audio_secs: f64, text: &str, processing: Duration) {
    if text.trim().is_empty() || !is_enabled(app) {
        return;
    }
    let chars = text.chars().count();
    let today = today();
    let updated = state::update(app, today, |s| {
        s.counters
            .add_transcription(audio_secs, chars, processing.as_secs_f64())
    });
    match updated {
        Ok(s) if s.send_due(today) => send_if_due(app),
        Ok(_) => {}
        Err(e) => warn!("Could not count the transcription for usage statistics: {e}"),
    }
}

/// Counts an error. Does nothing unless enabled.
pub fn count_error(app: &AppHandle, kind: ErrorKind) {
    if !is_enabled(app) {
        return;
    }
    if let Err(e) = state::update(app, today(), |s| s.counters.add_error(kind)) {
        warn!("Could not count the error for usage statistics: {e}");
    }
}

/// Sends the counters in the background when enabled and due, after a random
/// delay of up to 15 minutes (the "due" check runs again after the wait).
pub fn send_if_due(app: &AppHandle) {
    let Some(website_id) = website_id() else {
        return;
    };
    if !is_enabled(app) || SENDING.swap(true, Ordering::AcqRel) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(random_send_delay()).await;
        if let Err(e) = send(&app, website_id).await {
            warn!("Usage statistics not sent: {e}");
        }
        SENDING.store(false, Ordering::Release);
    });
}

async fn send(app: &AppHandle, website_id: &str) -> Result<(), String> {
    let today = today();
    // Saves a renewed id before it is sent.
    let state = state::update(app, today, |_| {})?;
    if !state.send_due(today) {
        return Ok(());
    }
    let (bodies, user_agent) = request(app, website_id, &state, today);
    let client = crate::utils::http::client(ENDPOINT, SEND_TIMEOUT, &user_agent, 0)?;
    for body in &bodies {
        let response = client
            .post(ENDPOINT)
            .json(body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }
    }
    // Turned off while sending: the state file is gone and must stay so.
    if !is_enabled(app) {
        return Ok(());
    }
    state::update(app, today, |s| {
        s.counters.subtract(&state.counters);
        s.last_sent = Some(today);
    })?;
    info!("Usage statistics sent");
    Ok(())
}

/// The exact JSON that would be sent now, pretty-printed.
pub fn preview(app: &AppHandle) -> Result<String, String> {
    let website_id = website_id().ok_or("Usage statistics are not available in this build")?;
    let today = today();
    let state = state::load(app, today)?;
    let (body, _) = request(app, website_id, &state, today);
    serde_json::to_string_pretty(&body).map_err(|e| e.to_string())
}

/// Saves the user's answer. Turning telemetry off deletes the id and the
/// counters; turning it on starts the "days since install" count.
pub fn set_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let enabled = enabled && is_available();
    let mut settings = crate::settings::load_settings(app);
    settings.telemetry_enabled = enabled;
    settings.telemetry_asked = true;
    crate::settings::save_settings(app, &settings)?;
    match enabled {
        true => {
            state::update(app, today(), |_| {})?;
            send_if_due(app);
            Ok(())
        }
        false => state::delete(app),
    }
}

#[cfg(test)]
mod delay_tests {
    use super::*;

    #[test]
    fn send_delay_stays_under_15_minutes_and_varies() {
        let delays: Vec<Duration> = (0..200).map(|_| random_send_delay()).collect();
        assert!(delays.iter().all(|d| d.as_secs() < 15 * 60));
        assert!(delays.iter().any(|d| *d != delays[0]));
    }
}
