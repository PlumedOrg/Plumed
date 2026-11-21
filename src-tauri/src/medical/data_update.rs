//! Checks for, and on the user's explicit request downloads, a newer signed
//! medical data set published on the `medical-data` release (Plumed fork,
//! see `medical-data/SOURCE.md`). The startup check fetches only the small
//! signed manifest, with no identifier and no cookie; nothing else is
//! downloaded without a click. Written with Claude Code.

use super::data_set::{self, manifest, manifest::DataVersion, manifest::Manifest};
use log::warn;
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Assets of the `medical-data` release, which must never be the "latest"
/// release (the app updater reads `releases/latest/download/latest.json`).
const RELEASE_URL: &str = "https://github.com/PlumedOrg/Plumed/releases/download/medical-data/";
const MANIFEST_NAME: &str = "manifest.json";
const SIGNATURE_NAME: &str = "manifest.json.minisig";
const MAX_MANIFEST_SIZE: u64 = 64 * 1024;
const MAX_SIGNATURE_SIZE: u64 = 4 * 1024;
const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
/// Per file: the dictionary rules are about 10 MB, hospital networks are slow.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);
/// GitHub sends release assets through a redirect to its storage host.
const MAX_REDIRECTS: usize = 5;
/// No version, no system: nothing that tells users apart.
const USER_AGENT: &str = "Plumed";

const OFFER_EVENT: &str = "medical-data-update-available";
const PROGRESS_EVENT: &str = "medical-data-download-progress";

/// What the update dialog shows.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateOffer {
    pub version: String,
    pub notes: String,
    pub file_count: usize,
    pub total_size: u64,
}

/// The data now in use after a download or a reset.
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

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);
static CHECKED_AT_STARTUP: AtomicBool = AtomicBool::new(false);
static DOWNLOADING: AtomicBool = AtomicBool::new(false);

fn pending() -> std::sync::MutexGuard<'static, Option<Pending>> {
    PENDING.lock().unwrap_or_else(PoisonError::into_inner)
}

fn offer_of(manifest: &Manifest) -> UpdateOffer {
    UpdateOffer {
        version: manifest.version.clone(),
        notes: manifest.notes.clone(),
        file_count: manifest.files.len(),
        total_size: manifest.total_size(),
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

/// The online manifest when it is valid and newer than the active data.
async fn fetch_newer() -> Result<Option<Pending>, String> {
    let key = data_set::public_key().ok_or("Medical data updates are not available")?;
    let app_version = data_set::app_version().ok_or("No app data folder")?;
    let client = crate::utils::http::client(RELEASE_URL, CHECK_TIMEOUT, USER_AGENT, MAX_REDIRECTS)?;
    let bytes = fetch_small(&client, MANIFEST_NAME, MAX_MANIFEST_SIZE).await?;
    let signature = fetch_small(&client, SIGNATURE_NAME, MAX_SIGNATURE_SIZE).await?;
    let signature = String::from_utf8(signature).map_err(|e| e.to_string())?;
    let (manifest, version) = manifest::parse_signed(key, &bytes, &signature, app_version)?;
    let active = tauri::async_runtime::spawn_blocking(|| data_set::current().version().to_string())
        .await
        .map_err(|e| e.to_string())?;
    match DataVersion::parse(&active) {
        Some(active) if version <= active => Ok(None),
        _ => Ok(Some(Pending {
            manifest,
            bytes,
            signature,
            prompt: false,
        })),
    }
}

/// Once per launch, after the model loaded: looks for a newer data set and
/// tells the frontend, which asks the user. Does nothing when the check is
/// off, in builds without a public key, or for the skipped version.
pub fn check_at_startup(app: &AppHandle) {
    if !data_set::is_available() || CHECKED_AT_STARTUP.swap(true, Ordering::AcqRel) {
        return;
    }
    let settings = crate::settings::load_settings(app);
    if !settings.medical_data_update_check {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match fetch_newer().await {
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
            Err(e) => warn!("Medical data update check failed: {e}"),
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
    let found = fetch_newer().await?;
    let offer = found.as_ref().map(|p| offer_of(&p.manifest));
    *pending() = found;
    Ok(offer)
}

/// Downloads the files of the offered `version` into `staging`, checking
/// each one's size and SHA-256 as it arrives.
async fn download_files(
    app: &AppHandle,
    manifest: &Manifest,
    staging: &std::path::Path,
) -> Result<(), String> {
    let client =
        crate::utils::http::client(RELEASE_URL, DOWNLOAD_TIMEOUT, USER_AGENT, MAX_REDIRECTS)?;
    let total = manifest.total_size();
    let mut downloaded = 0u64;
    let mut last_percent = None;
    for file in &manifest.files {
        let mut out = File::create(staging.join(&file.name)).map_err(|e| e.to_string())?;
        let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
        let url = format!("{RELEASE_URL}{}", file.name);
        let size = fetch(&client, &url, file.size, |chunk| {
            out.write_all(chunk).map_err(|e| e.to_string())?;
            hash.update(chunk);
            downloaded += chunk.len() as u64;
            let percent = (downloaded * 100).checked_div(total);
            if percent != last_percent {
                last_percent = percent;
                let _ = app.emit(PROGRESS_EVENT, Progress { downloaded, total });
            }
            Ok(())
        })
        .await?;
        if size != file.size || manifest::hex_of(hash.finish().as_ref()) != file.sha256 {
            return Err(format!("{} does not match the manifest", file.name));
        }
        out.sync_all().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Downloads, verifies, installs and activates the offered `version`, live:
/// the drug rules and the engine lists switch without a restart.
pub async fn download(app: &AppHandle, version: &str) -> Result<DataChange, String> {
    if DOWNLOADING.swap(true, Ordering::AcqRel) {
        return Err("A download is already running".to_string());
    }
    let result = download_and_install(app, version).await;
    DOWNLOADING.store(false, Ordering::Release);
    result
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
    let app = app.clone();
    let version = version.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let installed = data_set::install(&staging, &version, &bytes, &signature);
        if installed.is_err() {
            let _ = std::fs::remove_dir_all(&staging);
        }
        installed?;
        *pending() = None;
        Ok(reactivate(&app))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// "Return to the bundled data": deletes every downloaded set.
pub async fn reset(app: &AppHandle) -> Result<DataChange, String> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        data_set::reset()?;
        Ok(reactivate(&app))
    })
    .await
    .map_err(|e| e.to_string())?
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
    /// Updates possible in this build (public key set at build time).
    pub available: bool,
    pub bundled_version: &'static str,
    pub downloaded_version: Option<String>,
    pub check_enabled: bool,
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
        check_enabled: crate::settings::load_settings(app).medical_data_update_check,
    })
}

pub fn set_check_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = crate::settings::load_settings(app);
    settings.medical_data_update_check = enabled;
    crate::settings::save_settings(app, &settings)
}
