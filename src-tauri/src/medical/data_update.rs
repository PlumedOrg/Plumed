//! Checks for, and on the user's explicit request downloads, a newer signed
//! medical data set published on the `medical-data` release (Plumed fork,
//! see `medical-data/SOURCE.md`). The startup check only runs once the user
//! agreed to it, and fetches only the small signed manifest, with no
//! identifier and no cookie; nothing else is downloaded without a click.
//! The same set can also be installed from files downloaded by hand, with
//! the same checks. Written with Claude Code.

use super::data_set::{self, manifest::Manifest, ActiveData, SetError, StagedFile};
use crate::settings::AppSettings;
use log::warn;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

/// Assets of the `medical-data` release, which must never be the "latest"
/// release (the app updater reads `releases/latest/download/latest.json`).
const RELEASE_URL: &str = "https://github.com/PlumedOrg/Plumed/releases/download/medical-data/";
const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
/// Per file: the dictionary rules are about 10 MB, institutional networks slow.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);
/// GitHub sends release assets through a redirect to its storage host.
const MAX_REDIRECTS: usize = 5;
/// No version, no system: nothing tells users apart.
const USER_AGENT: &str = "Plumed";

const OFFER_EVENT: &str = "medical-data-update-available";
const PROGRESS_EVENT: &str = "medical-data-download-progress";
const CHECK_FAILED_EVENT: &str = "medical-data-check-failed";

/// What the update dialog shows.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateOffer {
    pub version: String,
    pub notes: String,
    pub file_count: usize,
    pub total_size: u64,
}

/// The data now in use after a download, a manual install or a reset.
#[derive(Debug, Clone, Serialize)]
pub struct DataChange {
    pub version: String,
    /// Saved edits whose rule the new data no longer has (still saved).
    pub orphaned_edits: usize,
}

#[derive(Debug, Clone, Serialize)]
struct Progress {
    downloaded: u64,
    total: u64,
}

/// A verified manifest newer than the active data.
struct Pending {
    manifest: Manifest,
    bytes: Vec<u8>,
    signature: String,
    /// Found by the startup check and not answered yet.
    prompt: bool,
}

/// What the last check found.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum CheckOutcome {
    UpToDate,
    UpdateFound { version: String },
    Failed { error: String },
}

/// The last check of this launch, shown in the settings (kept in memory
/// only).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CheckReport {
    /// Unix time in milliseconds.
    pub checked_at: u64,
    #[serde(flatten)]
    pub outcome: CheckOutcome,
}

/// The checks of this launch, and the failure notice the frontend shows at
/// most once per launch when the startup check failed.
#[derive(Default)]
struct CheckHistory {
    last: Option<CheckReport>,
    failure_notice: Option<String>,
    notice_given: bool,
}

impl CheckHistory {
    fn record(&mut self, report: CheckReport, at_startup: bool) {
        if let (true, false, CheckOutcome::Failed { error }) =
            (at_startup, self.notice_given, &report.outcome)
        {
            self.failure_notice = Some(error.clone());
        }
        self.last = Some(report);
    }

    /// The startup failure to tell the user about, once.
    fn take_failure_notice(&mut self) -> Option<String> {
        let notice = self.failure_notice.take();
        self.notice_given |= notice.is_some();
        notice
    }
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);
static HISTORY: Mutex<CheckHistory> = Mutex::new(CheckHistory {
    last: None,
    failure_notice: None,
    notice_given: false,
});
static CHECKED_AT_STARTUP: AtomicBool = AtomicBool::new(false);
static INSTALLING: AtomicBool = AtomicBool::new(false);

fn pending() -> std::sync::MutexGuard<'static, Option<Pending>> {
    PENDING.lock().unwrap_or_else(PoisonError::into_inner)
}

fn history() -> std::sync::MutexGuard<'static, CheckHistory> {
    HISTORY.lock().unwrap_or_else(PoisonError::into_inner)
}

fn offer_of(manifest: &Manifest) -> UpdateOffer {
    UpdateOffer {
        version: manifest.version.clone(),
        notes: manifest.notes.clone(),
        file_count: manifest.files.len(),
        total_size: manifest.total_size(),
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

fn report_of(result: &Result<Option<Pending>, String>) -> CheckReport {
    let outcome = match result {
        Ok(Some(found)) => CheckOutcome::UpdateFound {
            version: found.manifest.version.clone(),
        },
        Ok(None) => CheckOutcome::UpToDate,
        Err(error) => CheckOutcome::Failed {
            error: error.clone(),
        },
    };
    CheckReport {
        checked_at: now_ms(),
        outcome,
    }
}

/// Streams `url` to `on_chunk`, refusing more than `max` bytes. Returns the
/// byte count.
async fn fetch(
    client: &reqwest::Client,
    url: &str,
    max: u64,
    mut on_chunk: impl FnMut(&[u8]) -> Result<(), String>,
) -> Result<u64, String> {
    let mut response = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {} for {url}", response.status()));
    }
    if response.content_length().is_some_and(|len| len > max) {
        return Err(format!("{url} is larger than announced"));
    }
    let mut received = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        received += chunk.len() as u64;
        if received > max {
            return Err(format!("{url} is larger than announced"));
        }
        on_chunk(&chunk)?;
    }
    Ok(received)
}

async fn fetch_small(client: &reqwest::Client, name: &str, max: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    fetch(client, &format!("{RELEASE_URL}{name}"), max, |chunk| {
        bytes.extend_from_slice(chunk);
        Ok(())
    })
    .await?;
    Ok(bytes)
}

/// The online manifest if it is valid and newer than the active data.
async fn fetch_newer() -> Result<Option<Pending>, String> {
    let key = data_set::public_key().ok_or("Medical data updates are not available")?;
    let app_version = data_set::app_version().ok_or("No app data folder")?;
    let client = crate::utils::http::client(RELEASE_URL, CHECK_TIMEOUT, USER_AGENT, MAX_REDIRECTS)?;
    let bytes = fetch_small(
        &client,
        data_set::MANIFEST_FILE,
        data_set::manifest::MAX_MANIFEST_SIZE,
    )
    .await?;
    let signature = fetch_small(
        &client,
        data_set::SIGNATURE_FILE,
        data_set::manifest::MAX_SIGNATURE_SIZE,
    )
    .await?;
    let signature = String::from_utf8(signature).map_err(|e| e.to_string())?;
    let active = tauri::async_runtime::spawn_blocking(|| data_set::current().version().to_string())
        .await
        .map_err(|e| e.to_string())?;
    let (manifest, newer) =
        data_set::checked_manifest(key, &bytes, &signature, app_version, &active)?;
    Ok(newer.then_some(Pending {
        manifest,
        bytes,
        signature,
        prompt: false,
    }))
}

/// Whether the startup check may run: only once the user agreed to it, on
/// the first launch question or with the toggle.
fn startup_check_allowed(settings: &AppSettings) -> bool {
    settings.medical_data_update_asked && settings.medical_data_update_check
}

/// The first launch question is due until the user answered it, including
/// on installs from before the question existed.
fn question_due(settings: &AppSettings) -> bool {
    !settings.medical_data_update_asked
}

/// The answer to the first launch question, or the toggle: both count as
/// the user's choice.
fn apply_check_choice(settings: &mut AppSettings, enabled: bool) {
    settings.medical_data_update_check = enabled;
    settings.medical_data_update_asked = true;
}

/// Once per launch, after the model loaded: looks for a newer data set and
/// tells the frontend, which asks the user, or tells it the check failed.
/// Does nothing until the user agreed to the check, in builds without a
/// public key, or for the skipped version.
pub fn check_at_startup(app: &AppHandle) {
    let settings = crate::settings::load_settings(app);
    if !data_set::is_available()
        || !startup_check_allowed(&settings)
        || CHECKED_AT_STARTUP.swap(true, Ordering::AcqRel)
    {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = fetch_newer().await;
        history().record(report_of(&result), true);
        match result {
            Ok(Some(found)) if found.manifest.version != settings.medical_data_skipped_version => {
                let offer = offer_of(&found.manifest);
                *pending() = Some(Pending {
                    prompt: true,
                    ..found
                });
                if let Err(e) = app.emit(OFFER_EVENT, offer) {
                    warn!("Could not announce the medical data update: {e}");
                }
            }
            Ok(_) => {}
            Err(e) => {
                warn!("Medical data update check failed: {e}");
                if let Err(e) = app.emit(CHECK_FAILED_EVENT, ()) {
                    warn!("Could not announce the failed medical data check: {e}");
                }
            }
        }
    });
}

/// The update found at startup, while the user has not answered it.
pub fn startup_offer() -> Option<UpdateOffer> {
    pending()
        .as_ref()
        .filter(|p| p.prompt)
        .map(|p| offer_of(&p.manifest))
}

/// Why the startup check failed, returned once per launch so the notice is
/// never shown twice.
pub fn take_check_failure() -> Option<String> {
    history().take_failure_notice()
}

/// "Not now": asked again at the next launch.
pub fn dismiss_offer() {
    if let Some(p) = pending().as_mut() {
        p.prompt = false;
    }
}

/// "Skip this version": not offered at startup again, a newer one will be.
pub fn skip_version(app: &AppHandle, version: &str) -> Result<(), String> {
    dismiss_offer();
    let mut settings = crate::settings::load_settings(app);
    settings.medical_data_skipped_version = version.to_string();
    crate::settings::save_settings(app, &settings)
}

/// "Check now" in the settings: also offers a skipped version.
pub async fn check_now() -> Result<Option<UpdateOffer>, String> {
    let result = fetch_newer().await;
    history().record(report_of(&result), false);
    let found = result?;
    let offer = found.as_ref().map(|p| offer_of(&p.manifest));
    *pending() = found;
    Ok(offer)
}

/// Downloads the files of the offered `version` into `staging`, checking
/// each one's size and SHA-256 as it arrives.
async fn download_files(
    app: &AppHandle,
    manifest: &Manifest,
    staging: &Path,
) -> Result<(), String> {
    let client =
        crate::utils::http::client(RELEASE_URL, DOWNLOAD_TIMEOUT, USER_AGENT, MAX_REDIRECTS)?;
    let total = manifest.total_size();
    let mut downloaded = 0u64;
    let mut last_percent = None;
    for file in &manifest.files {
        let mut staged = StagedFile::create(staging, file)?;
        let url = format!("{RELEASE_URL}{}", file.name);
        fetch(&client, &url, file.size, |chunk| {
            staged.write(chunk)?;
            downloaded += chunk.len() as u64;
            let percent = (downloaded * 100).checked_div(total);
            if percent != last_percent {
                last_percent = percent;
                let _ = app.emit(PROGRESS_EVENT, Progress { downloaded, total });
            }
            Ok(())
        })
        .await?;
        staged.finish()?;
    }
    Ok(())
}

/// Runs one install at a time (download or manual install).
async fn one_install_at_a_time<E: From<String>>(
    job: impl std::future::Future<Output = Result<DataChange, E>>,
) -> Result<DataChange, E> {
    if INSTALLING.swap(true, Ordering::AcqRel) {
        return Err(E::from("An install is already running".to_string()));
    }
    let result = job.await;
    INSTALLING.store(false, Ordering::Release);
    result
}

/// Runs `install` off the async runtime, then switches the drug rules and
/// the engine lists to the new data, live.
async fn activate<E>(
    app: &AppHandle,
    install: impl FnOnce() -> Result<Arc<ActiveData>, E> + Send + 'static,
) -> Result<DataChange, E>
where
    E: From<String> + Send + 'static,
{
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        install()?;
        *pending() = None;
        Ok(reactivate(&app))
    })
    .await
    .map_err(|e| E::from(e.to_string()))?
}

/// Downloads, verifies, installs and activates the offered `version`.
pub async fn download(app: &AppHandle, version: &str) -> Result<DataChange, String> {
    one_install_at_a_time(download_and_install(app, version)).await
}

async fn download_and_install(app: &AppHandle, version: &str) -> Result<DataChange, String> {
    let (manifest, bytes, signature) = pending()
        .as_ref()
        .filter(|p| p.manifest.version == version)
        .map(|p| (p.manifest.clone(), p.bytes.clone(), p.signature.clone()))
        .ok_or("This update is no longer offered, check again")?;
    let staging = data_set::new_staging()?;
    if let Err(e) = download_files(app, &manifest, &staging).await {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e);
    }
    let version = version.to_string();
    activate(app, move || {
        data_set::install(&staging, &version, &bytes, &signature)
    })
    .await
}

/// "Install a downloaded update": verifies and activates a set from the
/// files the user picked (the release assets downloaded with a browser).
pub async fn install_picked(app: &AppHandle, paths: Vec<PathBuf>) -> Result<DataChange, SetError> {
    one_install_at_a_time(activate(app, move || data_set::install_picked(&paths))).await
}

/// "Return to the bundled data": deletes every downloaded set.
pub async fn reset(app: &AppHandle) -> Result<DataChange, String> {
    activate(app, data_set::reset).await
}

fn reactivate(app: &AppHandle) -> DataChange {
    let orphaned_edits = super::reactivate(app);
    DataChange {
        version: data_set::current().version().to_string(),
        orphaned_edits,
    }
}

/// What the settings show about the medical data.
#[derive(Debug, Clone, Serialize)]
pub struct DataStatus {
    /// Updates are possible in this build (public key set at build time).
    pub available: bool,
    pub bundled_version: &'static str,
    pub downloaded_version: Option<String>,
    /// The startup check runs (the user agreed to it).
    pub check_enabled: bool,
    /// The last check of this launch, `None` before the first one.
    pub last_check: Option<CheckReport>,
}

pub async fn status(app: &AppHandle) -> Result<DataStatus, String> {
    let downloaded_version =
        tauri::async_runtime::spawn_blocking(|| data_set::current().downloaded.clone())
            .await
            .map_err(|e| e.to_string())?;
    Ok(DataStatus {
        available: data_set::is_available(),
        bundled_version: data_set::BUNDLED_VERSION,
        downloaded_version,
        check_enabled: startup_check_allowed(&crate::settings::load_settings(app)),
        last_check: history().last.clone(),
    })
}

/// Whether to ask, this launch, if the check may run at each start.
pub fn question_due_now(app: &AppHandle) -> bool {
    data_set::is_available() && question_due(&crate::settings::load_settings(app))
}

/// Saves the user's choice (first launch question or toggle). A "yes"
/// checks from the next start on, so this launch shows no second dialog.
pub fn set_check_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = crate::settings::load_settings(app);
    apply_check_choice(&mut settings, enabled);
    crate::settings::save_settings(app, &settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_check_before_the_user_answered() {
        let fresh = AppSettings::default();
        assert!(question_due(&fresh));
        assert!(!startup_check_allowed(&fresh));

        // An install from before the question: the check was on by default.
        let older: AppSettings =
            serde_json::from_str(r#"{"medical_data_update_check": true}"#).unwrap();
        assert!(question_due(&older));
        assert!(!startup_check_allowed(&older));

        let mut agreed = older.clone();
        apply_check_choice(&mut agreed, true);
        assert!(!question_due(&agreed));
        assert!(startup_check_allowed(&agreed));

        let mut refused = fresh;
        apply_check_choice(&mut refused, false);
        assert!(!question_due(&refused));
        assert!(!startup_check_allowed(&refused));
    }

    fn report(outcome: CheckOutcome) -> CheckReport {
        CheckReport {
            checked_at: 1,
            outcome,
        }
    }

    fn failed(error: &str) -> CheckOutcome {
        CheckOutcome::Failed {
            error: error.to_string(),
        }
    }

    #[test]
    fn the_failure_notice_is_given_once_per_launch() {
        let mut history = CheckHistory::default();
        assert_eq!(history.take_failure_notice(), None);

        // "Check now" shows its own error, no notice.
        history.record(report(failed("offline")), false);
        assert_eq!(history.take_failure_notice(), None);
        assert_eq!(history.last, Some(report(failed("offline"))));

        history.record(report(failed("timeout")), true);
        assert_eq!(history.take_failure_notice().as_deref(), Some("timeout"));
        assert_eq!(history.take_failure_notice(), None);

        // Never twice in a launch.
        history.record(report(failed("again")), true);
        assert_eq!(history.take_failure_notice(), None);

        history.record(report(CheckOutcome::UpToDate), false);
        assert_eq!(history.last, Some(report(CheckOutcome::UpToDate)));
    }

    #[test]
    fn a_successful_startup_check_gives_no_notice() {
        let mut history = CheckHistory::default();
        history.record(report(CheckOutcome::UpToDate), true);
        assert_eq!(history.take_failure_notice(), None);
    }
}
