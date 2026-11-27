//! Downloaded speech models on disk (Plumed fork, written with Claude Code),
//! in `models/` of the app data folder, so outside the install folder and
//! kept across updates:
//!
//! - one `<variant>-<revision prefix>/` folder per variant, holding its files
//!   under the names the engine loads, `<name>.partial` while downloading;
//! - `model.json` in that folder, written last: a folder without a valid
//!   marker is never used;
//! - `active.json` naming the variant in use.
//!
//! The launch check is cheap (marker and file sizes); the full SHA-256 check
//! runs after a download and on "Vérifier".

use super::catalog::{self, Variant};
use crate::utils::verified_file::file_sha256;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const MARKER_FILE: &str = "model.json";
const ACTIVE_FILE: &str = "active.json";
const PARTIAL_SUFFIX: &str = ".partial";
/// Enough of the revision to tell two pins apart in a folder name.
const REVISION_PREFIX: usize = 12;

/// Where a model was downloaded from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    HuggingFace,
    Relay,
}

#[derive(Debug, Serialize, Deserialize)]
struct Marker {
    variant: String,
    revision: String,
    source: Source,
}

#[derive(Serialize, Deserialize)]
struct ActivePointer {
    variant: String,
}

pub fn folder_name(variant: &Variant) -> String {
    let revision = variant.family.repo.revision;
    format!(
        "{}-{}",
        variant.id,
        &revision[..REVISION_PREFIX.min(revision.len())]
    )
}

pub fn variant_dir(root: &Path, variant: &Variant) -> PathBuf {
    root.join(folder_name(variant))
}

pub fn partial_path(dir: &Path, local_name: &str) -> PathBuf {
    dir.join(format!("{local_name}{PARTIAL_SUFFIX}"))
}

/// The source of an installed `variant` if its marker is valid and every
/// file has its size. Cheap: no file is read.
pub fn installed_source(root: &Path, variant: &Variant) -> Option<Source> {
    let dir = variant_dir(root, variant);
    let marker: Marker =
        serde_json::from_str(&fs::read_to_string(dir.join(MARKER_FILE)).ok()?).ok()?;
    let matches = marker.variant == variant.id && marker.revision == variant.family.repo.revision;
    let sizes_ok = variant.files.iter().all(|f| {
        fs::metadata(dir.join(f.local_name)).is_ok_and(|m| m.is_file() && m.len() == f.size)
    });
    (matches && sizes_ok).then_some(marker.source)
}

/// Marks `variant` installed, after its files were checked. Written to a
/// temporary file then renamed, so a crash never leaves half a marker.
pub fn write_marker(root: &Path, variant: &Variant, source: Source) -> io::Result<()> {
    let dir = variant_dir(root, variant);
    let marker = Marker {
        variant: variant.id.to_owned(),
        revision: variant.family.repo.revision.to_owned(),
        source,
    };
    write_atomic(&dir.join(MARKER_FILE), &serde_json::to_vec_pretty(&marker)?)
}

/// Full check of an installed `variant`: hashes every file. A damaged file
/// and the marker are removed, so the next download fetches only that file
/// and the model is not used meanwhile. Returns whether it was intact.
pub fn verify(root: &Path, variant: &Variant) -> io::Result<bool> {
    let dir = variant_dir(root, variant);
    let mut intact = installed_source(root, variant).is_some();
    for file in variant.files {
        let path = dir.join(file.local_name);
        let good = path.is_file() && file_sha256(&path)? == file.sha256;
        if !good {
            intact = false;
            remove_if_present(&path)?;
        }
    }
    if !intact {
        remove_if_present(&dir.join(MARKER_FILE))?;
    }
    Ok(intact)
}

/// Bytes of `variant` already on disk (finished files and partial ones), so
/// a resumed download starts its progress there and the space check only
/// asks for the rest.
pub fn bytes_on_disk(root: &Path, variant: &Variant) -> u64 {
    let dir = variant_dir(root, variant);
    let size_of = |path: PathBuf| fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    variant
        .files
        .iter()
        .map(|f| {
            let done = size_of(dir.join(f.local_name));
            let partial = size_of(partial_path(&dir, f.local_name));
            done.max(partial).min(f.size)
        })
        .sum()
}

/// The variant in use, if one was chosen and is still in the catalog.
pub fn active(root: &Path) -> Option<&'static Variant> {
    let text = fs::read_to_string(root.join(ACTIVE_FILE)).ok()?;
    let pointer: ActivePointer = serde_json::from_str(&text).ok()?;
    catalog::find(&pointer.variant)
}

pub fn set_active(root: &Path, variant: &Variant) -> io::Result<()> {
    let pointer = ActivePointer {
        variant: variant.id.to_owned(),
    };
    write_atomic(&root.join(ACTIVE_FILE), &serde_json::to_vec(&pointer)?)
}

/// Deletes the folder of `variant` (installed or partly downloaded), and the
/// active choice if it was this one.
pub fn delete(root: &Path, variant: &Variant) -> io::Result<()> {
    if active(root).is_some_and(|a| a.id == variant.id) {
        remove_if_present(&root.join(ACTIVE_FILE))?;
    }
    match fs::remove_dir_all(variant_dir(root, variant)) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Deletes the folders no catalog variant uses any more (older revisions,
/// removed variants). Partial downloads of current variants are kept.
pub fn remove_stale(root: &Path) -> io::Result<()> {
    let current: Vec<String> = catalog::all_variants().map(folder_name).collect();
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type()?.is_dir() && !current.contains(&name) {
            fs::remove_dir_all(entry.path())?;
        }
    }
    Ok(())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)
}

fn remove_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::model::catalog::{Family, ModelFile};
    use crate::utils::verified_file::hex_of;
    use plumed_relay_protocol::models::ModelRepo;

    pub fn temp_root() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("plumed-models-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    pub fn sha(bytes: &[u8]) -> String {
        hex_of(ring::digest::digest(&ring::digest::SHA256, bytes).as_ref())
    }

    /// A two-file variant whose contents are `a.bin` = "alpha", `b.bin` = "beta!".
    pub fn tiny_variant() -> &'static Variant {
        let file = |path: &'static str, body: &[u8]| ModelFile {
            path,
            size: body.len() as u64,
            sha256: Box::leak(sha(body).into_boxed_str()),
            local_name: Box::leak(path.replace('/', "_").into_boxed_str()),
        };
        let files = vec![file("x/a.bin", b"alpha"), file("b.bin", b"beta!")];
        let family = Box::leak(Box::new(Family {
            id: "ultra",
            name: "Tiny",
            repo: ModelRepo {
                repo: "owner/tiny",
                revision: "0123456789abcdef0123456789abcdef01234567",
            },
        }));
        Box::leak(Box::new(Variant {
            id: "ultra-int8",
            family,
            name: "Tiny",
            summary: "",
            files: Box::leak(files.into_boxed_slice()),
            available: true,
            experimental: false,
        }))
    }

    pub fn install(root: &Path, variant: &Variant) {
        let dir = variant_dir(root, variant);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(variant.files[0].local_name), b"alpha").unwrap();
        fs::write(dir.join(variant.files[1].local_name), b"beta!").unwrap();
        write_marker(root, variant, Source::Relay).unwrap();
    }

    #[test]
    fn folder_names_carry_the_variant_and_the_revision() {
        assert_eq!(
            folder_name(&catalog::VARIANTS[0]),
            "ultra-v1-int8-ce5160b5dcc9"
        );
        // Unchanged since v2 was the only model, so existing installs keep it.
        assert_eq!(
            folder_name(catalog::find("ultra-int8").unwrap()),
            "ultra-int8-675b4b8d7ee9"
        );
    }

    #[test]
    fn a_folder_is_used_only_with_a_matching_marker_and_sizes() {
        let root = temp_root();
        let variant = tiny_variant();
        assert_eq!(installed_source(&root, variant), None);
        install(&root, variant);
        assert_eq!(installed_source(&root, variant), Some(Source::Relay));

        let dir = variant_dir(&root, variant);
        fs::write(dir.join(variant.files[1].local_name), b"beta").unwrap();
        assert_eq!(installed_source(&root, variant), None, "wrong size");

        install(&root, variant);
        fs::write(
            dir.join(MARKER_FILE),
            r#"{"variant":"ultra-int8","revision":"old","source":"relay"}"#,
        )
        .unwrap();
        assert_eq!(installed_source(&root, variant), None, "other revision");

        fs::remove_file(dir.join(MARKER_FILE)).unwrap();
        assert_eq!(installed_source(&root, variant), None, "no marker");
    }

    #[test]
    fn verify_removes_a_damaged_file_and_the_marker() {
        let root = temp_root();
        let variant = tiny_variant();
        install(&root, variant);
        assert!(verify(&root, variant).unwrap());

        let dir = variant_dir(&root, variant);
        let damaged = dir.join(variant.files[0].local_name);
        fs::write(&damaged, b"alphA").unwrap();
        assert_eq!(
            installed_source(&root, variant),
            Some(Source::Relay),
            "same size"
        );
        assert!(!verify(&root, variant).unwrap());
        assert!(!damaged.exists());
        assert!(!dir.join(MARKER_FILE).exists());
        assert!(
            dir.join(variant.files[1].local_name).exists(),
            "the good file stays"
        );
    }

    #[test]
    fn bytes_on_disk_counts_finished_and_partial_files() {
        let root = temp_root();
        let variant = tiny_variant();
        assert_eq!(bytes_on_disk(&root, variant), 0);
        let dir = variant_dir(&root, variant);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(variant.files[0].local_name), b"alpha").unwrap();
        fs::write(partial_path(&dir, variant.files[1].local_name), b"be").unwrap();
        assert_eq!(bytes_on_disk(&root, variant), 7);
    }

    #[test]
    fn the_active_choice_survives_until_its_folder_is_deleted() {
        let root = temp_root();
        assert!(active(&root).is_none());
        let variant = &catalog::VARIANTS[1];
        set_active(&root, variant).unwrap();
        assert_eq!(active(&root).map(|v| v.id), Some(variant.id));

        delete(&root, &catalog::VARIANTS[0]).unwrap();
        assert_eq!(active(&root).map(|v| v.id), Some(variant.id), "another one");
        delete(&root, variant).unwrap();
        assert!(active(&root).is_none());

        fs::write(root.join(ACTIVE_FILE), r#"{"variant":"gone"}"#).unwrap();
        assert!(active(&root).is_none(), "no longer in the catalog");
    }

    #[test]
    fn stale_folders_go_and_current_ones_stay() {
        let root = temp_root();
        let current = variant_dir(&root, &catalog::VARIANTS[0]);
        let auxiliary = variant_dir(&root, &catalog::DIARIZATION);
        let old = root.join("ultra-int8-000000000000");
        fs::create_dir_all(&current).unwrap();
        fs::create_dir_all(&auxiliary).unwrap();
        fs::create_dir_all(&old).unwrap();
        set_active(&root, &catalog::VARIANTS[0]).unwrap();
        remove_stale(&root).unwrap();
        assert!(current.exists());
        assert!(auxiliary.exists());
        assert!(!old.exists());
        assert!(root.join(ACTIVE_FILE).exists());
        assert!(remove_stale(&root.join("missing")).is_ok());
    }
}
