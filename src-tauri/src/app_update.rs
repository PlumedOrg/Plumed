//! App updates (Plumed fork, written with Claude Code): which releases the
//! user follows, and the Tauri updater pointed at them.
//!
//! - Channel ([`UpdateChannel`], setting `app_update_channel`): `stable`
//!   (default, also for settings saved before the setting existed) reads the
//!   latest release's `latest.json`; `beta` reads the GitHub listing of the 10
//!   newest releases and takes the newest one carrying `latest.json`,
//!   pre-release or not; `disabled` makes no check unless the user asks.
//! - The updater installs only a newer version (semver: `0.2.0-beta.2` is
//!   newer than `0.2.0-beta.1` and older than `0.2.0`), so beta users get the
//!   final release and going back to stable never downgrades.
//! - GitHub first; when that fails (blocked network, timeout, error answer)
//!   the same files through the Plumed relay (`plumed_relay_protocol::releases`),
//!   with the installer URL inside `latest.json` rewritten to the relay too. An
//!   installer download that fails on GitHub is retried through the relay.
//!   The updater checks the installer's signature against the public key in
//!   `tauri.conf.json`, so a relay cannot forge an update.
//! - A release whose `latest.json` lacks this platform (its build is late,
//!   see AGENTS.FORK.md "Releases") is "no update yet", not an error, and
//!   is not retried through the relay (it serves the same file).

use crate::model::download::relay_base;
use plumed_relay_protocol::models::safe_segment;
use plumed_relay_protocol::releases::{
    ReleaseFile, GITHUB_API_URL, GITHUB_URL, LATEST_JSON, LISTING_API_PATH, RELAY_LISTING_PATH,
};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Url};
use tauri_plugin_updater::{Update, UpdaterExt};

/// No version, no system: nothing tells users apart.
const USER_AGENT: &str = "Plumed";
/// For the listing and `latest.json` (not the installer download).
const TIMEOUT: Duration = Duration::from_secs(20);
/// Tells the frontend the channel changed (payload: the new channel).
const CHANNEL_EVENT: &str = "app-update-channel";

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateChannel {
    #[default]
    Stable,
    Beta,
    Disabled,
}

impl UpdateChannel {
    /// The serialized name, also sent in the anonymous statistics.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Disabled => "disabled",
        }
    }
}

/// The update found by the last check, until it is installed.
static PENDING: Mutex<Option<Update>> = Mutex::new(None);

/// The channel a check follows: none for an automatic check when updates
/// are disabled, stable for the manual check offered then.
fn channel_to_check(channel: UpdateChannel, manual: bool) -> Option<UpdateChannel> {
    match (channel, manual) {
        (UpdateChannel::Disabled, false) => None,
        (UpdateChannel::Disabled, true) => Some(UpdateChannel::Stable),
        (channel, _) => Some(channel),
    }
}

/// Where the release files come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Route {
    GitHub,
    Relay,
}

impl Route {
    fn file_url(self, file: &ReleaseFile, relay: &str) -> String {
        match self {
            Route::GitHub => format!("{GITHUB_URL}{}", file.github_path()),
            Route::Relay => format!("{}{}", relay.trim_end_matches('/'), file.path()),
        }
    }

    fn listing_url(self, relay: &str) -> String {
        match self {
            Route::GitHub => format!("{GITHUB_API_URL}{LISTING_API_PATH}"),
            Route::Relay => format!("{}{RELAY_LISTING_PATH}", relay.trim_end_matches('/')),
        }
    }
}

/// The part of a GitHub release the beta channel reads.
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
}

/// The `latest.json` of the newest non-draft release that has one (GitHub
/// lists newest first). The URL is built from the tag, never taken from the
/// listing.
fn newest_manifest(releases: &[Release]) -> Option<ReleaseFile> {
    releases
        .iter()
        .find(|r| {
            !r.draft && safe_segment(&r.tag_name) && r.assets.iter().any(|a| a.name == LATEST_JSON)
        })
        .map(|r| ReleaseFile::Asset {
            tag: r.tag_name.clone(),
            asset: LATEST_JSON.to_owned(),
        })
}

/// The same file through the relay, when `url` is one of our GitHub release files.
fn through_relay(url: &Url, relay: &str) -> Option<Url> {
    let file = ReleaseFile::from_github_url(url.as_str())?;
    Route::Relay.file_url(&file, relay).parse().ok()
}

async fn fetch_listing(url: &str) -> Result<Vec<Release>, String> {
    let client = crate::utils::http::client(url, TIMEOUT, USER_AGENT, 5)?;
    client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

/// The `latest.json` the channel follows.
async fn manifest(
    channel: UpdateChannel,
    route: Route,
    relay: &str,
) -> Result<ReleaseFile, String> {
    match channel {
        UpdateChannel::Beta => {
            let releases = fetch_listing(&route.listing_url(relay)).await?;
            newest_manifest(&releases).ok_or_else(|| "no release carries latest.json".to_owned())
        }
        UpdateChannel::Stable | UpdateChannel::Disabled => Ok(ReleaseFile::LatestJson),
    }
}

async fn check_on(
    app: &AppHandle,
    channel: UpdateChannel,
    route: Route,
    relay: &str,
) -> Result<Option<Update>, String> {
    let file = manifest(channel, route, relay).await?;
    let endpoint = Url::parse(&route.file_url(&file, relay)).map_err(|e| e.to_string())?;
    let update = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .and_then(|b| b.timeout(TIMEOUT).build())
        .map_err(|e| e.to_string())?
        .check()
        .await;
    let mut update = no_build_yet_is_no_update(update)?;
    if let (Route::Relay, Some(found)) = (route, update.as_mut()) {
        if let Some(url) = through_relay(&found.download_url, relay) {
            found.download_url = url;
        }
    }
    Ok(update)
}

/// A newer version released without this platform's build (yet) is no update.
fn no_build_yet_is_no_update<T>(
    checked: Result<Option<T>, tauri_plugin_updater::Error>,
) -> Result<Option<T>, String> {
    match checked {
        Err(tauri_plugin_updater::Error::TargetNotFound(target)) => {
            log::info!("A newer version exists, but not for {target} yet");
            Ok(None)
        }
        other => other.map_err(|e| e.to_string()),
    }
}

/// Looks for an update on `channel`: GitHub, then the relay.
async fn check_channel(app: &AppHandle, channel: UpdateChannel) -> Result<Option<Update>, String> {
    let relay = relay_base();
    match check_on(app, channel, Route::GitHub, relay).await {
        Ok(update) => Ok(update),
        Err(github) => {
            log::warn!("Update check on GitHub failed ({github}), trying the relay");
            check_on(app, channel, Route::Relay, relay)
                .await
                .map_err(|relay| format!("GitHub: {github}; relay: {relay}"))
        }
    }
}

/// The newer version, if any, on the user's channel. An automatic check
/// (`manual` false) does nothing when updates are disabled.
pub async fn check(app: &AppHandle, manual: bool) -> Result<Option<String>, String> {
    let settings = crate::settings::load_settings(app);
    let Some(channel) = channel_to_check(settings.app_update_channel, manual) else {
        return Ok(None);
    };
    let update = check_channel(app, channel).await?;
    let version = update.as_ref().map(|u| u.version.clone());
    *PENDING.lock().map_err(|e| e.to_string())? = update;
    Ok(version)
}

/// The version the last check found, for a frontend that missed the event.
pub fn pending_version() -> Option<String> {
    PENDING.lock().ok()?.as_ref().map(|u| u.version.clone())
}

/// Downloads and installs the update found by the last check, reporting the
/// download in percent (each value once).
pub async fn install(on_progress: impl Fn(u8)) -> Result<(), String> {
    let update = PENDING
        .lock()
        .map_err(|e| e.to_string())?
        .take()
        .ok_or_else(|| "no update to install".to_owned())?;
    let bytes = match download(&update, &on_progress).await {
        Ok(bytes) => bytes,
        Err(e) => {
            let Some(url) = through_relay(&update.download_url, relay_base()) else {
                return Err(e.to_string());
            };
            log::warn!("Update download failed ({e}), trying the relay");
            let mut relayed = update.clone();
            relayed.download_url = url;
            download(&relayed, &on_progress)
                .await
                .map_err(|e| e.to_string())?
        }
    };
    update.install(bytes).map_err(|e| e.to_string())
}

async fn download(
    update: &Update,
    on_progress: &impl Fn(u8),
) -> Result<Vec<u8>, tauri_plugin_updater::Error> {
    let mut downloaded = 0u64;
    let mut last = None;
    update
        .download(
            |chunk, total| {
                downloaded += chunk as u64;
                if let Some(percent) = total.filter(|t| *t > 0).map(|t| percent(downloaded, t)) {
                    if last != Some(percent) {
                        last = Some(percent);
                        on_progress(percent);
                    }
                }
            },
            || {},
        )
        .await
}

fn percent(done: u64, total: u64) -> u8 {
    (done.saturating_mul(100) / total).min(100) as u8
}

/// The user's choice, from the startup wizard or the settings: it also
/// counts as answering the wizard question.
fn apply_channel_choice(settings: &mut crate::settings::AppSettings, channel: UpdateChannel) {
    settings.app_update_channel = channel;
    settings.app_update_channel_asked = true;
}

/// Saves the channel and tells the frontend.
pub fn set_channel(app: &AppHandle, channel: UpdateChannel) -> Result<(), String> {
    let mut settings = crate::settings::load_settings(app);
    apply_channel_choice(&mut settings, channel);
    crate::settings::save_settings(app, &settings)?;
    if let Err(e) = app.emit(CHANNEL_EVENT, channel) {
        log::warn!("Failed to announce the update channel: {e}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::AppSettings;

    #[test]
    fn a_release_without_our_platform_is_no_update_yet() {
        use tauri_plugin_updater::Error;
        let missing =
            no_build_yet_is_no_update::<()>(Err(Error::TargetNotFound("darwin-aarch64".into())));
        assert_eq!(missing, Ok(None));
        let other = no_build_yet_is_no_update::<()>(Err(Error::EmptyEndpoints));
        assert!(other.is_err());
    }

    #[test]
    fn channel_name_matches_its_serialized_form() {
        for channel in [
            UpdateChannel::Stable,
            UpdateChannel::Beta,
            UpdateChannel::Disabled,
        ] {
            let json = serde_json::to_string(&channel).unwrap();
            assert_eq!(json, format!("\"{}\"", channel.as_str()));
        }
    }

    #[test]
    fn choosing_a_channel_marks_the_question_answered() {
        let mut settings = AppSettings::default();
        assert!(!settings.app_update_channel_asked);
        apply_channel_choice(&mut settings, UpdateChannel::Stable);
        assert!(settings.app_update_channel_asked);
        assert_eq!(settings.app_update_channel, UpdateChannel::Stable);
    }

    const RELAY: &str = "https://relay.test/";

    fn release(tag: &str, draft: bool, assets: &[&str]) -> Release {
        Release {
            tag_name: tag.to_owned(),
            draft,
            assets: assets
                .iter()
                .map(|name| Asset {
                    name: (*name).to_owned(),
                })
                .collect(),
        }
    }

    #[test]
    fn the_channel_defaults_to_stable_and_saves_in_snake_case() {
        let old: AppSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.app_update_channel, UpdateChannel::Stable);
        let beta: AppSettings = serde_json::from_str(r#"{"app_update_channel": "beta"}"#).unwrap();
        assert_eq!(beta.app_update_channel, UpdateChannel::Beta);
        assert_eq!(
            serde_json::to_string(&UpdateChannel::Disabled).unwrap(),
            "\"disabled\""
        );
    }

    #[test]
    fn disabled_only_checks_when_asked_and_then_on_stable() {
        assert_eq!(channel_to_check(UpdateChannel::Disabled, false), None);
        assert_eq!(
            channel_to_check(UpdateChannel::Disabled, true),
            Some(UpdateChannel::Stable)
        );
        assert_eq!(
            channel_to_check(UpdateChannel::Beta, false),
            Some(UpdateChannel::Beta)
        );
    }

    #[test]
    fn beta_takes_the_newest_published_release_with_a_manifest() {
        let releases = [
            release("0.3.0-beta.1", true, &[LATEST_JSON]),
            release("medical-data", false, &["manifest.json"]),
            release("bad/tag", false, &[LATEST_JSON]),
            release(
                "0.2.0-beta.2",
                false,
                &["Plumed_x64-setup.exe", LATEST_JSON],
            ),
            release("0.1.0", false, &[LATEST_JSON]),
        ];
        assert_eq!(
            newest_manifest(&releases),
            Some(ReleaseFile::Asset {
                tag: "0.2.0-beta.2".into(),
                asset: LATEST_JSON.into()
            })
        );
        assert_eq!(newest_manifest(&releases[..3]), None);
    }

    #[test]
    fn routes_point_at_github_or_the_relay() {
        let file = ReleaseFile::LatestJson;
        assert_eq!(
            Route::GitHub.file_url(&file, RELAY),
            "https://github.com/PlumedOrg/Plumed/releases/latest/download/latest.json"
        );
        assert_eq!(
            Route::Relay.file_url(&file, RELAY),
            "https://relay.test/releases/latest/download/latest.json"
        );
        assert_eq!(
            Route::GitHub.listing_url(RELAY),
            "https://api.github.com/repos/PlumedOrg/Plumed/releases?per_page=10"
        );
        assert_eq!(
            Route::Relay.listing_url(RELAY),
            "https://relay.test/releases/list"
        );
    }

    #[test]
    fn only_our_github_installers_are_rewritten_to_the_relay() {
        let ours: Url =
            "https://github.com/PlumedOrg/Plumed/releases/download/0.2.0/Plumed_amd64.deb"
                .parse()
                .unwrap();
        assert_eq!(
            through_relay(&ours, RELAY).map(String::from).as_deref(),
            Some("https://relay.test/releases/download/0.2.0/Plumed_amd64.deb")
        );
        let other: Url = "https://example.org/Plumed_amd64.deb".parse().unwrap();
        assert_eq!(through_relay(&other, RELAY), None);
    }

    /// `scripts/make-latest-json.py` reads the repository from this endpoint.
    #[test]
    fn the_configured_endpoint_is_the_stable_one() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(
            conf["plugins"]["updater"]["endpoints"][0],
            Route::GitHub.file_url(&ReleaseFile::LatestJson, RELAY)
        );
    }

    #[test]
    fn percent_is_capped_and_rounded_down() {
        assert_eq!(percent(0, 200), 0);
        assert_eq!(percent(199, 200), 99);
        assert_eq!(percent(300, 200), 100);
    }
}
