//! Downloaded medical data sets on disk, in `medical-data/` of the app data
//! folder: one `<version>/` folder per set (its signed manifest and its
//! files) and `active.json` naming the one in use. Written with Claude Code.

use super::manifest::{self, parse_signed, Manifest, ManifestFile};
use super::SetError;
use log::warn;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const ACTIVE_FILE: &str = "active.json";
pub const MANIFEST_FILE: &str = "manifest.json";
pub const SIGNATURE_FILE: &str = "manifest.json.minisig";
const STAGING_PREFIX: &str = ".staging-";

#[derive(Serialize, Deserialize)]
struct ActivePointer {
    version: String,
}

/// The active version, `None` when no downloaded set is active.
pub fn read_active(dir: &Path) -> Result<Option<String>, String> {
    let path = dir.join(ACTIVE_FILE);
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str::<ActivePointer>(&text)
            .map(|p| Some(p.version))
            .map_err(|e| format!("Invalid {ACTIVE_FILE}: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Writes `active.json` through a temp file and a rename, so a crash never
/// leaves half a file.
pub fn write_active(dir: &Path, version: &str) -> Result<(), String> {
    let json = serde_json::to_string(&ActivePointer {
        version: version.to_string(),
    })
    .map_err(|e| e.to_string())?;
    let tmp = dir.join(format!("{ACTIVE_FILE}.tmp"));
    fs::write(&tmp, json).map_err(|e| e.to_string())?;
    fs::rename(&tmp, dir.join(ACTIVE_FILE)).map_err(|e| e.to_string())
}

/// The files of the set in `set_dir`, after checking the manifest signature
/// and every file's size and SHA-256 again (the disk may have changed).
pub fn read_verified(
    set_dir: &Path,
    version: &str,
    key: &str,
    app_version: &semver::Version,
) -> Result<HashMap<String, String>, String> {
    let read = |name: &str| fs::read(set_dir.join(name)).map_err(|e| format!("{name}: {e}"));
    let signature = String::from_utf8(read(SIGNATURE_FILE)?).map_err(|e| e.to_string())?;
    let (manifest, _) = parse_signed(key, &read(MANIFEST_FILE)?, &signature, app_version)?;
    if manifest.version != version {
        return Err(format!(
            "Folder {version} holds version {}",
            manifest.version
        ));
    }
    let mut files = HashMap::new();
    for file in &manifest.files {
        let bytes = read(&file.name)?;
        if bytes.len() as u64 != file.size || manifest::sha256_hex(&bytes) != file.sha256 {
            return Err(format!("{} does not match the manifest", file.name));
        }
        let text = String::from_utf8(bytes).map_err(|e| format!("{}: {e}", file.name))?;
        files.insert(file.name.clone(), text);
    }
    Ok(files)
}

/// Saves the signed manifest next to the files it lists.
pub fn write_manifest(set_dir: &Path, manifest: &[u8], signature: &str) -> Result<(), String> {
    fs::write(set_dir.join(MANIFEST_FILE), manifest).map_err(|e| e.to_string())?;
    fs::write(set_dir.join(SIGNATURE_FILE), signature).map_err(|e| e.to_string())
}

/// One file of a set being written into a staging folder, downloaded or
/// copied from files the user picked: refused unless its size and SHA-256
/// are those of its manifest entry.
pub struct StagedFile<'a> {
    entry: &'a ManifestFile,
    out: fs::File,
    hash: ring::digest::Context,
    written: u64,
}

impl<'a> StagedFile<'a> {
    /// `entry` comes from a checked manifest, so its name is allowlisted.
    pub fn create(staging: &Path, entry: &'a ManifestFile) -> Result<Self, SetError> {
        Ok(Self {
            entry,
            out: fs::File::create(staging.join(&entry.name)).map_err(SetError::other)?,
            hash: ring::digest::Context::new(&ring::digest::SHA256),
            written: 0,
        })
    }

    pub fn write(&mut self, chunk: &[u8]) -> Result<(), SetError> {
        self.written += chunk.len() as u64;
        if self.written > self.entry.size {
            return Err(self.mismatch());
        }
        self.out.write_all(chunk).map_err(SetError::other)?;
        self.hash.update(chunk);
        Ok(())
    }

    pub fn finish(self) -> Result<(), SetError> {
        let mismatch = self.mismatch();
        let Self {
            entry,
            out,
            hash,
            written,
        } = self;
        if written != entry.size || manifest::hex_of(hash.finish().as_ref()) != entry.sha256 {
            return Err(mismatch);
        }
        out.sync_all().map_err(SetError::other)
    }

    fn mismatch(&self) -> SetError {
        SetError::FileMismatch {
            name: self.entry.name.clone(),
        }
    }
}

/// Copies the files of `manifest` from `picked` (file name to path, as the
/// user chose them) into `staging`, checking each one.
pub fn stage_picked(
    staging: &Path,
    manifest: &Manifest,
    picked: &HashMap<String, PathBuf>,
) -> Result<(), SetError> {
    for entry in &manifest.files {
        let path = picked.get(&entry.name).ok_or(SetError::MissingFile {
            name: entry.name.clone(),
        })?;
        let mut staged = StagedFile::create(staging, entry)?;
        // Size first, so a wrong file is never read whole.
        if fs::metadata(path).map_err(SetError::other)?.len() != entry.size {
            return Err(staged.mismatch());
        }
        staged.write(&fs::read(path).map_err(SetError::other)?)?;
        staged.finish()?;
    }
    Ok(())
}

/// A new empty folder in `dir` to download a set into.
pub fn new_staging(dir: &Path) -> Result<PathBuf, String> {
    let staging = dir.join(format!("{STAGING_PREFIX}{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    Ok(staging)
}

/// Renames the complete `staging` folder to `dir/<version>` (replacing a
/// leftover folder of the same version, never the active one).
pub fn commit_staging(dir: &Path, staging: &Path, version: &str) -> Result<(), String> {
    let target = dir.join(version);
    if read_active(dir).ok().flatten().as_deref() == Some(version) {
        return Err(format!("Version {version} is already active"));
    }
    if target.exists() {
        fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
    }
    fs::rename(staging, &target).map_err(|e| e.to_string())
}

fn remove_dir(path: &Path) {
    if let Err(e) = fs::remove_dir_all(path) {
        warn!("Could not delete {}: {e}", path.display());
    }
}

pub fn remove_set(dir: &Path, version: &str) {
    remove_dir(&dir.join(version));
}

/// Deletes every set and leftover staging folder but `keep`.
pub fn remove_others(dir: &Path, keep: &str) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && entry.file_name() != keep {
            remove_dir(&path);
        }
    }
}

/// Deletes `active.json` then every set.
pub fn remove_all(dir: &Path) -> Result<(), String> {
    match fs::remove_file(dir.join(ACTIVE_FILE)) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.to_string()),
    }
    remove_others(dir, "");
    Ok(())
}
