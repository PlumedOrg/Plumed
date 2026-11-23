//! The signed `manifest.json` of a medical data set published online (see
//! `medical-data/SOURCE.md`): what is checked before anything is downloaded
//! or loaded. Written with Claude Code.

use super::{SetError, CONFUSION_FILE, MODEL_ID, PHRASE_FILE, RULE_FILES};
use minisign_verify::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Largest file a manifest may list, so a bad manifest cannot fill the disk.
pub const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;
/// Largest manifest and signature accepted, downloaded or picked by hand.
pub const MAX_MANIFEST_SIZE: u64 = 64 * 1024;
pub const MAX_SIGNATURE_SIZE: u64 = 4 * 1024;
/// Longest release notes shown in the update dialog.
const MAX_NOTES_CHARS: usize = 2_000;

/// A data set version, `YYYY.MM.DD` or `YYYY.MM.DD.N`, compared numerically.
/// The validated shape also makes it safe as a folder name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DataVersion([u32; 4]);

impl DataVersion {
    pub const MIN: Self = Self([0; 4]);

    pub fn parse(text: &str) -> Option<Self> {
        let parts: Vec<&str> = text.split('.').collect();
        let widths: &[usize] = match parts.len() {
            3 => &[4, 2, 2],
            4 => &[4, 2, 2, 0],
            _ => return None,
        };
        let mut numbers = [0u32; 4];
        for (i, (part, width)) in parts.iter().zip(widths).enumerate() {
            let shape_ok = match width {
                0 => (1..=4).contains(&part.len()),
                w => part.len() == *w,
            };
            if !shape_ok || !part.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            numbers[i] = part.parse().ok()?;
        }
        match (numbers[1], numbers[2]) {
            (1..=12, 1..=31) => Some(Self(numbers)),
            _ => None,
        }
    }
}

/// One file of the set, published next to the manifest under `name`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestFile {
    pub name: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub model: String,
    pub min_app_version: String,
    #[serde(default)]
    pub notes: String,
    pub files: Vec<ManifestFile>,
}

fn is_sha256_hex(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_allowed_name(name: &str) -> bool {
    RULE_FILES.contains(&name) || name == CONFUSION_FILE || name == PHRASE_FILE
}

impl Manifest {
    /// Checks everything that does not depend on what is installed: file
    /// allowlist, complete rule set, hashes and sizes, model, app version.
    pub fn check(&self, app_version: &semver::Version) -> Result<DataVersion, SetError> {
        let version = DataVersion::parse(&self.version).ok_or_else(|| {
            SetError::invalid(format!("invalid data version \"{}\"", self.version))
        })?;
        if self.model != MODEL_ID {
            return Err(SetError::WrongModel {
                model: self.model.clone(),
                expected: MODEL_ID.to_string(),
            });
        }
        let min_app = semver::Version::parse(&self.min_app_version)
            .map_err(|e| SetError::invalid(format!("invalid min_app_version: {e}")))?;
        if min_app > *app_version {
            return Err(SetError::AppTooOld {
                min_app_version: min_app.to_string(),
            });
        }
        if self.notes.chars().count() > MAX_NOTES_CHARS {
            return Err(SetError::invalid("release notes too long"));
        }
        let mut names = HashSet::new();
        for file in &self.files {
            if !is_allowed_name(&file.name) {
                return Err(SetError::invalid(format!(
                    "unexpected file \"{}\"",
                    file.name
                )));
            }
            if !names.insert(file.name.as_str()) {
                return Err(SetError::invalid(format!("\"{}\" listed twice", file.name)));
            }
            if !is_sha256_hex(&file.sha256) {
                return Err(SetError::invalid(format!(
                    "invalid SHA-256 for \"{}\"",
                    file.name
                )));
            }
            if file.size > MAX_FILE_SIZE {
                return Err(SetError::invalid(format!("\"{}\" too large", file.name)));
            }
        }
        match RULE_FILES.iter().find(|name| !names.contains(*name)) {
            Some(missing) => Err(SetError::invalid(format!(
                "incomplete rule set, \"{missing}\" missing"
            ))),
            None => Ok(version),
        }
    }

    pub fn total_size(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }
}

/// Checks the minisign `signature` of `message`. `public_key` is the base64
/// key line, or a whole `.pub` file (its last line is used). Legacy (not
/// prehashed) signatures are accepted, like the Tauri updater does.
pub fn verify_signature(public_key: &str, message: &[u8], signature: &str) -> Result<(), SetError> {
    let key_line = public_key
        .lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty())
        .ok_or("No public key")?;
    let key = PublicKey::from_base64(key_line)
        .map_err(|e| SetError::other(format!("Invalid public key: {e}")))?;
    let bad = |e: minisign_verify::Error| SetError::BadSignature {
        detail: e.to_string(),
    };
    let signature = Signature::decode(signature).map_err(bad)?;
    key.verify(message, &signature, true).map_err(bad)
}

/// A verified, parsed and checked manifest.
pub fn parse_signed(
    public_key: &str,
    manifest: &[u8],
    signature: &str,
    app_version: &semver::Version,
) -> Result<(Manifest, DataVersion), SetError> {
    verify_signature(public_key, manifest, signature)?;
    let parsed: Manifest = serde_json::from_slice(manifest).map_err(SetError::invalid)?;
    let version = parsed.check(app_version)?;
    Ok((parsed, version))
}

/// Lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex_of(ring::digest::digest(&ring::digest::SHA256, bytes).as_ref())
}

pub use crate::utils::verified_file::hex_of;
