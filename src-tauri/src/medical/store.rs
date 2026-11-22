//! The user's changes to the medical data, one JSON file per list in the app
//! data folder. Separate from `formatting_rules.json`, so importing or
//! resetting the user's formatting rules never touches them, and never part
//! of the exported app settings.

use serde::de::DeserializeOwned;
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

/// A store file and the key the changes are saved under.
pub struct StoreFile {
    name: &'static str,
    key: &'static str,
    /// Shown in error messages.
    what: &'static str,
}

/// Edits of the drug and medical term rules ([`super::RuleEdit`]).
pub const RULE_EDITS: StoreFile = StoreFile {
    name: "medical_rules.json",
    key: "edits",
    what: "medical rule edits",
};
/// Changes to the phrase boosting list ([`super::PhraseOverlay`]).
pub const PHRASE_OVERLAY: StoreFile = StoreFile {
    name: "medical_phrases.json",
    key: "overlay",
    what: "phrase list changes",
};
/// Changes to the confusion groups ([`super::ConfusionOverlay`]).
pub const CONFUSION_OVERLAY: StoreFile = StoreFile {
    name: "medical_confusion_groups.json",
    key: "overlay",
    what: "confusion group changes",
};

/// Changes to the grammar step's rules ([`crate::grammar::rules::GrammarOverlay`]).
pub const GRAMMAR_OVERLAY: StoreFile = StoreFile {
    name: "grammar_rules.json",
    key: "overlay",
    what: "grammar rule changes",
};

/// The saved changes, the default (no change) when there are none.
pub fn load<T: DeserializeOwned + Default>(app: &AppHandle, file: &StoreFile) -> Result<T, String> {
    let store = app.store(file.name).map_err(|e| e.to_string())?;
    match store.get(file.key) {
        Some(value) => {
            serde_json::from_value(value).map_err(|e| format!("Failed to parse {}: {e}", file.what))
        }
        None => Ok(T::default()),
    }
}

pub fn save<T: Serialize + ?Sized>(
    app: &AppHandle,
    file: &StoreFile,
    value: &T,
) -> Result<(), String> {
    let store = app.store(file.name).map_err(|e| e.to_string())?;
    let value = serde_json::to_value(value)
        .map_err(|e| format!("Failed to serialize {}: {e}", file.what))?;
    store.set(file.key, value);
    Ok(())
}
