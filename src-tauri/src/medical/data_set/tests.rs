//! Tests of the downloaded medical data sets. Written with Claude Code.

use super::manifest::{parse_signed, sha256_hex, verify_signature, Manifest, ManifestFile};
use super::*;
use crate::medical::RuleEdit;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair};
use std::fs;

/// A throwaway minisign key pair made with `ring` (test only). Signs in
/// minisign's legacy mode (plain Ed25519 of the message); the prehashed
/// mode is checked with the fixture of the `minisign-verify` crate below.
struct TestKey {
    pair: Ed25519KeyPair,
    key_id: [u8; 8],
}

impl TestKey {
    fn new(key_id: u8) -> Self {
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
        Self {
            pair: Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap(),
            key_id: [key_id; 8],
        }
    }

    /// The base64 line of a minisign `.pub` file.
    fn public(&self) -> String {
        let mut bytes = b"Ed".to_vec();
        bytes.extend(self.key_id);
        bytes.extend(self.pair.public_key().as_ref());
        B64.encode(bytes)
    }

    /// A `.minisig` file for `message`.
    fn sign(&self, message: &[u8]) -> String {
        let signature = self.pair.sign(message);
        let mut line = b"Ed".to_vec();
        line.extend(self.key_id);
        line.extend(signature.as_ref());
        let trusted = "timestamp:0\tfile:manifest.json";
        let mut global = signature.as_ref().to_vec();
        global.extend(trusted.as_bytes());
        format!(
            "untrusted comment: test\n{}\ntrusted comment: {trusted}\n{}\n",
            B64.encode(line),
            B64.encode(self.pair.sign(&global).as_ref())
        )
    }
}

fn app_version() -> semver::Version {
    semver::Version::new(0, 1, 4)
}

/// One usable rule per UltiMed file, an empty Plumed file.
fn rule_files() -> Vec<(String, String)> {
    let rule = BUNDLED_RULES[4].lines().next().unwrap().to_string();
    RULE_FILES
        .iter()
        .map(|name| match *name {
            "plumed_fix_rules.jsonl" => (name.to_string(), String::new()),
            _ => (name.to_string(), rule.clone()),
        })
        .collect()
}

fn manifest_for(version: &str, files: &[(String, String)]) -> Manifest {
    Manifest {
        version: version.to_string(),
        model: MODEL_ID.to_string(),
        min_app_version: "0.1.4".to_string(),
        notes: "Nouvelles règles".to_string(),
        files: files
            .iter()
            .map(|(name, content)| ManifestFile {
                name: name.clone(),
                sha256: sha256_hex(content.as_bytes()),
                size: content.len() as u64,
            })
            .collect(),
    }
}

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("plumed-medical-data-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes a signed set as the active one in `dir`, like an install does.
fn write_active_set(dir: &Path, key: &TestKey, manifest: &Manifest, files: &[(String, String)]) {
    let set_dir = dir.join(&manifest.version);
    fs::create_dir_all(&set_dir).unwrap();
    let bytes = serde_json::to_vec(manifest).unwrap();
    disk::write_manifest(&set_dir, &bytes, &key.sign(&bytes)).unwrap();
    for (name, content) in files {
        fs::write(set_dir.join(name), content).unwrap();
    }
    disk::write_active(dir, &manifest.version).unwrap();
}

#[test]
fn versions_are_validated_and_compared_numerically() {
    let v = |s| DataVersion::parse(s);
    assert!(v("2026.11.21").is_some());
    assert!(v("2026.11.21.3").is_some());
    for bad in [
        "2026.11.2",
        "2026.13.01",
        "2026.11.00",
        "26.11.21",
        "2026.11.21.",
        "2026.11.21.x",
        "../x",
        "2026-11-21",
        "2026.11.21.1.1",
    ] {
        assert_eq!(v(bad), None, "{bad}");
    }
    assert!(v("2026.11.22") > v("2026.11.21.9"));
    assert!(v("2026.11.21.10") > v("2026.11.21.9"));
    assert!(v("2026.11.21.1") > v("2026.11.21"));
    assert!(v(BUNDLED_VERSION).is_some());
}

#[test]
fn a_complete_manifest_passes() {
    let manifest = manifest_for("2026.12.01", &rule_files());
    assert_eq!(
        manifest.check(&app_version()).ok(),
        DataVersion::parse("2026.12.01")
    );
}

#[test]
fn unknown_files_and_incomplete_rule_sets_are_refused() {
    let mut files = rule_files();
    files.push(("evil.sh".to_string(), String::new()));
    assert!(manifest_for("2026.12.01", &files)
        .check(&app_version())
        .is_err());

    let mut traversal = rule_files();
    traversal.push(("../french_medical.txt".to_string(), String::new()));
    assert!(manifest_for("2026.12.01", &traversal)
        .check(&app_version())
        .is_err());

    let incomplete: Vec<_> = rule_files().into_iter().skip(1).collect();
    assert!(manifest_for("2026.12.01", &incomplete)
        .check(&app_version())
        .is_err());

    let mut optional = rule_files();
    optional.push((PHRASE_FILE.to_string(), "aspirine\n".to_string()));
    optional.push((CONFUSION_FILE.to_string(), String::new()));
    assert!(manifest_for("2026.12.01", &optional)
        .check(&app_version())
        .is_ok());

    let mut twice = rule_files();
    twice.push(rule_files().remove(0));
    assert!(manifest_for("2026.12.01", &twice)
        .check(&app_version())
        .is_err());
}

#[test]
fn other_models_newer_apps_and_bad_hashes_are_refused() {
    let base = manifest_for("2026.12.01", &rule_files());
    let other_model = Manifest {
        model: "ultimed-v3".to_string(),
        ..base.clone()
    };
    assert!(other_model.check(&app_version()).is_err());
    let newer_app = Manifest {
        min_app_version: "0.2.0".to_string(),
        ..base.clone()
    };
    assert!(newer_app.check(&app_version()).is_err());
    assert!(newer_app.check(&semver::Version::new(0, 2, 0)).is_ok());
    let mut bad_hash = base.clone();
    bad_hash.files[0].sha256 = "ABC".to_string();
    assert!(bad_hash.check(&app_version()).is_err());
    let mut huge = base;
    huge.files[0].size = manifest::MAX_FILE_SIZE + 1;
    assert!(huge.check(&app_version()).is_err());
}

#[test]
fn signatures_are_checked() {
    let key = TestKey::new(1);
    let manifest = serde_json::to_vec(&manifest_for("2026.12.01", &rule_files())).unwrap();
    let signature = key.sign(&manifest);
    assert!(parse_signed(&key.public(), &manifest, &signature, &app_version()).is_ok());
    // A whole `.pub` file works too.
    let pub_file = format!("untrusted comment: minisign public key\n{}\n", key.public());
    assert!(verify_signature(&pub_file, &manifest, &signature).is_ok());

    let mut tampered = manifest.clone();
    tampered[10] ^= 1;
    assert!(verify_signature(&key.public(), &tampered, &signature).is_err());
    let other = TestKey::new(1);
    assert!(verify_signature(&other.public(), &manifest, &signature).is_err());
    assert!(verify_signature(&key.public(), &manifest, "garbage").is_err());
}

#[test]
fn prehashed_signatures_from_minisign_are_accepted() {
    // Fixture of the minisign-verify crate: what `minisign -S` writes.
    let signature = "untrusted comment: signature from minisign secret key
RUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=
trusted comment: timestamp:1556193335\tfile:test
y/rUw2y8/hOUYjZU71eHp/Wo1KZ40fGy2VJEDl34XMJM+TX48Ss/17u3IvIfbVR1FkZZSNCisQbuQY+bHwhEBg==";
    let key = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
    assert!(verify_signature(key, b"test", signature).is_ok());
    assert!(verify_signature(key, b"Test", signature).is_err());
}

#[test]
fn a_valid_downloaded_set_is_used() {
    let dir = temp_dir();
    let key = TestKey::new(2);
    let mut files = rule_files();
    files.push((
        CONFUSION_FILE.to_string(),
        BUNDLED_CONFUSION_GROUPS.to_string(),
    ));
    write_active_set(&dir, &key, &manifest_for("2099.01.01", &files), &files);
    let data = load_or_bundled(&dir, &key.public(), &app_version());
    assert_eq!(data.downloaded.as_deref(), Some("2099.01.01"));
    assert_eq!(data.rules.len(), 6);
    assert_eq!(data.phrase_list, BUNDLED_PHRASE_LIST);
    fs::remove_dir_all(dir).unwrap();
}

/// Every way a downloaded set can be bad falls back to the bundled data.
#[test]
fn a_bad_downloaded_set_falls_back_to_the_bundled_data() {
    let key = TestKey::new(3);
    let files = rule_files();
    let manifest = manifest_for("2099.01.01", &files);
    let corruptions: [(&str, fn(&Path)); 5] = [
        ("file changed", |set| {
            fs::write(set.join("drug_fix_rules.jsonl"), "{}").unwrap()
        }),
        ("file missing", |set| {
            fs::remove_file(set.join("manual_fix_rules.jsonl")).unwrap()
        }),
        ("manifest changed", |set| {
            let text = fs::read_to_string(set.join("manifest.json")).unwrap();
            fs::write(set.join("manifest.json"), text.replace("2099", "2098")).unwrap()
        }),
        ("signature missing", |set| {
            fs::remove_file(set.join("manifest.json.minisig")).unwrap()
        }),
        ("active.json broken", |set| {
            fs::write(set.parent().unwrap().join("active.json"), "{").unwrap()
        }),
    ];
    for (what, corrupt) in corruptions {
        let dir = temp_dir();
        write_active_set(&dir, &key, &manifest, &files);
        corrupt(&dir.join("2099.01.01"));
        let data = load_or_bundled(&dir, &key.public(), &app_version());
        assert_eq!(data.downloaded, None, "{what}");
        assert!(data.rules.len() > 20_000, "{what}");
        fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn unparsable_rules_or_another_key_fall_back_too() {
    let key = TestKey::new(4);
    let mut files = rule_files();
    files[0].1 = "not json\nnot json either".to_string();
    let dir = temp_dir();
    write_active_set(&dir, &key, &manifest_for("2099.01.01", &files), &files);
    assert!(load_downloaded(&dir, &key.public(), &app_version()).is_err());
    fs::remove_dir_all(dir).unwrap();

    let files = rule_files();
    let dir = temp_dir();
    write_active_set(&dir, &key, &manifest_for("2099.01.01", &files), &files);
    assert!(load_downloaded(&dir, &TestKey::new(4).public(), &app_version()).is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn bundled_data_wins_when_as_recent_or_nothing_is_installed() {
    let key = TestKey::new(5);
    let dir = temp_dir();
    assert!(matches!(
        load_downloaded(&dir, &key.public(), &app_version()),
        Ok(None)
    ));
    let files = rule_files();
    write_active_set(&dir, &key, &manifest_for(BUNDLED_VERSION, &files), &files);
    assert!(matches!(
        load_downloaded(&dir, &key.public(), &app_version()),
        Ok(None)
    ));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn older_sets_are_removed_but_the_kept_one() {
    let dir = temp_dir();
    for version in ["2099.01.01", "2099.02.01"] {
        fs::create_dir_all(dir.join(version)).unwrap();
    }
    let staging = disk::new_staging(&dir).unwrap();
    disk::commit_staging(&dir, &staging, "2099.03.01").unwrap();
    disk::write_active(&dir, "2099.03.01").unwrap();
    disk::remove_others(&dir, "2099.03.01");
    let mut left: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    left.sort();
    assert_eq!(left, ["2099.03.01", "active.json"]);
    disk::remove_all(&dir).unwrap();
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn edits_of_rules_that_no_longer_exist_are_counted() {
    let rules = Bundled::from_jsonl(&[BUNDLED_RULES[4]]);
    let line: serde_json::Value =
        serde_json::from_str(BUNDLED_RULES[4].lines().next().unwrap()).unwrap();
    let edit = |original: Option<&str>| RuleEdit {
        original: original.map(str::to_string),
        core: "abc".to_string(),
        replacement: "ABC".to_string(),
        enabled: true,
    };
    let edits = [
        edit(line["pattern"].as_str()),
        edit(Some("a pattern from an older rule set")),
        edit(None),
    ];
    let orphaned = rules.orphaned(&edits);
    assert_eq!(orphaned, [&edits[1]]);
}
