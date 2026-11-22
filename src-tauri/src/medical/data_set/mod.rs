//! The French medical data in use: the set bundled with the app, or a newer
//! signed set downloaded from the `medical-data` release (Plumed fork, see
//! `medical-data/SOURCE.md`). The one place that knows which: the drug rules
//! and the engine lists both read [`current`]. The bundled set stays the
//! fallback whenever a downloaded one is missing, corrupt, for another model
//! or does not parse. Written with Claude Code.

mod disk;
pub mod manifest;
#[cfg(test)]
mod tests;

use super::drug_rules::Bundled;
use crate::engine::confusion::ConfusionGroups;
use crate::engine::phrase_boost::list::parse_boost_list;
use log::{info, warn};
use manifest::DataVersion;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use tauri::{AppHandle, Manager};

/// The model the bundled rules fix the errors of (`scripts/fetch-ultimed-model.sh`).
/// A downloaded set for another model is refused.
pub const MODEL_ID: &str = "ultimed-v2";
/// Version of the bundled data, the same shape as a manifest version.
pub const BUNDLED_VERSION: &str = "2026.11.22";

/// Rule file names, in the order they apply: UltiMed's, then Plumed's own
/// (last, so it also respells what an UltiMed rule wrote). A downloaded set
/// must have all of them.
pub const RULE_FILES: [&str; 7] = [
    "drug_fix_rules.jsonl",
    "dictionary_fix_rules.jsonl",
    "parhaf_fix_rules.jsonl",
    "parrot_fix_rules.jsonl",
    "acronyms_fix_rules.jsonl",
    "manual_fix_rules.jsonl",
    "plumed_fix_rules.jsonl",
];
/// Optional in a downloaded set: missing means the bundled one is kept.
pub const CONFUSION_FILE: &str = "confusion_groups.jsonl";
pub const PHRASE_FILE: &str = "french_medical.txt";

/// Plumed-only rules, same JSONL shape as UltiMed's `manual_fix_rules.jsonl`.
pub(super) const PLUMED_RULES: &str =
    include_str!("../../../medical-data/plumed-rules/plumed_fix_rules.jsonl");
/// The bundled rule files, in the order of [`RULE_FILES`].
const BUNDLED_RULES: [&str; 7] = [
    include_str!("../../../medical-data/regex-fixes/drug_fix_rules.jsonl"),
    include_str!("../../../medical-data/regex-fixes/dictionary_fix_rules.jsonl"),
    include_str!("../../../medical-data/regex-fixes/parhaf_fix_rules.jsonl"),
    include_str!("../../../medical-data/regex-fixes/parrot_fix_rules.jsonl"),
    include_str!("../../../medical-data/regex-fixes/acronyms_fix_rules.jsonl"),
    include_str!("../../../medical-data/regex-fixes/manual_fix_rules.jsonl"),
    PLUMED_RULES,
];
pub(super) const BUNDLED_CONFUSION_GROUPS: &str =
    include_str!("../../../medical-data/regex-fixes/confusion_groups.jsonl");
pub(super) const BUNDLED_PHRASE_LIST: &str =
    include_str!("../../../medical-data/phrase-boosting/french_medical.txt");

/// Minisign public key of the `medical-data` release, set at build time
/// (base64 key line of `plumed-rules.pub`). TODO: generate the key pair and
/// set the `PLUMED_RULES_PUBKEY` repository secret on GitHub, release
/// builds never look for medical data updates until then.
const PUBLIC_KEY: Option<&str> = option_env!("PLUMED_RULES_PUBKEY");

/// Highest share of unusable lines a downloaded rule set may have.
const MAX_UNUSABLE_SHARE: f64 = 0.05;

pub fn public_key() -> Option<&'static str> {
    PUBLIC_KEY.filter(|key| !key.trim().is_empty())
}

/// Whether this build can update its medical data (public key set at build
/// time). Without it nothing is downloaded, loaded or shown.
pub fn is_available() -> bool {
    public_key().is_some()
}

/// The data the medical post-processing and the engine use.
pub struct ActiveData {
    /// Downloaded version, `None` for the bundled data.
    pub downloaded: Option<String>,
    pub rules: Arc<Bundled>,
    pub phrase_list: Cow<'static, str>,
    pub confusion_groups: Cow<'static, str>,
}

impl ActiveData {
    fn bundled() -> Self {
        Self {
            downloaded: None,
            rules: Arc::new(Bundled::from_jsonl(&BUNDLED_RULES)),
            phrase_list: Cow::Borrowed(BUNDLED_PHRASE_LIST),
            confusion_groups: Cow::Borrowed(BUNDLED_CONFUSION_GROUPS),
        }
    }

    /// A downloaded set from its verified files, refused when a list does
    /// not parse well enough to replace the bundled one.
    fn from_files(version: &str, mut files: HashMap<String, String>) -> Result<Self, String> {
        let sources = RULE_FILES
            .iter()
            .map(|name| files.get(*name).map(String::as_str))
            .collect::<Option<Vec<&str>>>()
            .ok_or("Incomplete rule set")?;
        let rules = Bundled::from_jsonl(&sources);
        let unusable = rules.unusable() as f64;
        if rules.is_empty() || unusable > rules.len() as f64 * MAX_UNUSABLE_SHARE {
            return Err(format!(
                "{} of {} rules unusable",
                rules.unusable(),
                rules.len()
            ));
        }
        let phrase_list = match files.remove(PHRASE_FILE) {
            Some(text) if parse_boost_list(&text).phrases.is_empty() => {
                return Err("Empty phrase list".to_string())
            }
            Some(text) => Cow::Owned(text),
            None => Cow::Borrowed(BUNDLED_PHRASE_LIST),
        };
        let confusion_groups = match files.remove(CONFUSION_FILE) {
            Some(text) if ConfusionGroups::parse(&text).is_empty() => {
                return Err("No confusion group".to_string())
            }
            Some(text) => Cow::Owned(text),
            None => Cow::Borrowed(BUNDLED_CONFUSION_GROUPS),
        };
        Ok(Self {
            downloaded: Some(version.to_string()),
            rules: Arc::new(rules),
            phrase_list,
            confusion_groups,
        })
    }

    /// Version shown to the user and compared with online sets.
    pub fn version(&self) -> &str {
        self.downloaded.as_deref().unwrap_or(BUNDLED_VERSION)
    }
}

/// Where downloaded sets live and what this app is, set once at startup.
struct Context {
    dir: PathBuf,
    app_version: semver::Version,
}

static CONTEXT: OnceLock<Context> = OnceLock::new();
static ACTIVE: Mutex<Option<Arc<ActiveData>>> = Mutex::new(None);

/// Records the data folder (`medical-data/` in the app data folder). Call
/// before anything reads [`current`], or only the bundled data is seen.
pub fn init(app: &AppHandle) {
    match app.path().app_data_dir() {
        Ok(dir) => {
            let _ = CONTEXT.set(Context {
                dir: dir.join("medical-data"),
                app_version: app.package_info().version.clone(),
            });
        }
        Err(e) => warn!("No app data folder, using the bundled medical data: {e}"),
    }
}

fn set_active(data: Arc<ActiveData>) {
    *ACTIVE.lock().unwrap_or_else(PoisonError::into_inner) = Some(data);
}

/// The data in use, loaded on first call (the downloaded set if a valid one
/// is installed, the bundled one otherwise).
pub fn current() -> Arc<ActiveData> {
    let mut active = ACTIVE.lock().unwrap_or_else(PoisonError::into_inner);
    active.get_or_insert_with(|| Arc::new(load())).clone()
}

fn load() -> ActiveData {
    match (public_key(), CONTEXT.get()) {
        (Some(key), Some(context)) => load_or_bundled(&context.dir, key, &context.app_version),
        _ => ActiveData::bundled(),
    }
}

fn load_or_bundled(dir: &Path, key: &str, app_version: &semver::Version) -> ActiveData {
    match load_downloaded(dir, key, app_version) {
        Ok(Some(data)) => {
            info!("Using downloaded medical data {}", data.version());
            data
        }
        Ok(None) => ActiveData::bundled(),
        Err(e) => {
            warn!("Downloaded medical data unusable, using the bundled data: {e}");
            ActiveData::bundled()
        }
    }
}

fn bundled_version() -> DataVersion {
    DataVersion::parse(BUNDLED_VERSION).unwrap_or(DataVersion::MIN)
}

/// The active downloaded set in `dir`, `None` when there is none or when
/// the bundled data is as recent (after an app update).
fn load_downloaded(
    dir: &Path,
    key: &str,
    app_version: &semver::Version,
) -> Result<Option<ActiveData>, String> {
    let Some(version) = disk::read_active(dir)? else {
        return Ok(None);
    };
    let parsed = DataVersion::parse(&version).ok_or("Invalid active version")?;
    if parsed <= bundled_version() {
        info!("Bundled medical data is as recent as the downloaded {version}");
        return Ok(None);
    }
    load_set(dir, &version, key, app_version).map(Some)
}

fn load_set(
    dir: &Path,
    version: &str,
    key: &str,
    app_version: &semver::Version,
) -> Result<ActiveData, String> {
    let files = disk::read_verified(&dir.join(version), version, key, app_version)?;
    ActiveData::from_files(version, files)
}

fn context() -> Result<(&'static Context, &'static str), String> {
    let key = public_key().ok_or("Medical data updates are not available in this build")?;
    let context = CONTEXT.get().ok_or("No app data folder")?;
    Ok((context, key))
}

/// This app's version, which a manifest's `min_app_version` must not exceed.
pub fn app_version() -> Option<&'static semver::Version> {
    CONTEXT.get().map(|c| &c.app_version)
}

/// A fresh empty folder to download a set into, next to the installed ones
/// (same file system, so [`install`] can rename it).
pub fn new_staging() -> Result<PathBuf, String> {
    let (context, _) = context()?;
    disk::new_staging(&context.dir)
}

/// Installs the verified files in `staging`, with the signed manifest they
/// were checked against, as `version` and makes them the active data. The
/// previous set stays active, and on disk, unless the new one loads fine;
/// older sets are deleted after the switch.
pub fn install(
    staging: &Path,
    version: &str,
    manifest: &[u8],
    signature: &str,
) -> Result<Arc<ActiveData>, String> {
    let (context, key) = context()?;
    disk::write_manifest(staging, manifest, signature)?;
    disk::commit_staging(&context.dir, staging, version)?;
    let data = match load_set(&context.dir, version, key, &context.app_version) {
        Ok(data) => Arc::new(data),
        Err(e) => {
            disk::remove_set(&context.dir, version);
            return Err(e);
        }
    };
    disk::write_active(&context.dir, version)?;
    set_active(data.clone());
    disk::remove_others(&context.dir, version);
    Ok(data)
}

/// Deletes every downloaded set and goes back to the bundled data.
pub fn reset() -> Result<Arc<ActiveData>, String> {
    let (context, _) = context()?;
    disk::remove_all(&context.dir)?;
    let data = Arc::new(ActiveData::bundled());
    set_active(data.clone());
    Ok(data)
}
