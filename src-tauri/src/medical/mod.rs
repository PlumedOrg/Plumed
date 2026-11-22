//! French medical post-processing (Plumed fork only, not for upstream).
//!
//! The drug and medical-term fix rules (bundled, or a newer downloaded set,
//! see `data_set`), plus the user's edits to them, kept apart from the user's
//! own formatting rules: those run after, and an import or a reset of either
//! side leaves the other alone. The phrase list and the confusion groups get
//! the same treatment (`list_overlay`), applied when `engine_lists` arms them.

pub mod data_set;
pub mod data_update;
mod drug_rules;
pub mod engine_lists;
mod interhop;
pub mod list_overlay;
mod store;

use drug_rules::DrugRules;
pub use drug_rules::{validate_edit, RuleEdit, RuleView};
pub use engine_lists::MedicalList;
pub use interhop::default_rules as default_formatting_rules;
use list_overlay::{ListOverlay, ListPage, OverlayEntry, Phrase};
use log::warn;
use serde::Deserialize;
use std::sync::{Arc, RwLock};
use tauri::AppHandle;

use crate::engine::confusion::ConfusionGroup;

/// The rules [`apply_medical_fixes`] runs: active data set plus saved edits.
static ACTIVE: RwLock<Option<Arc<DrugRules>>> = RwLock::new(None);

fn activate(edits: &[RuleEdit]) {
    let rules = Arc::new(DrugRules::new(data_set::current().rules.clone(), edits));
    match ACTIVE.write() {
        Ok(mut active) => *active = Some(rules),
        Err(e) => warn!("Medical rules lock poisoned: {e}"),
    }
}

fn active() -> Arc<DrugRules> {
    if let Some(rules) = ACTIVE.read().ok().and_then(|a| a.clone()) {
        return rules;
    }
    activate(&[]);
    ACTIVE
        .read()
        .ok()
        .and_then(|a| a.clone())
        .unwrap_or_else(|| Arc::new(DrugRules::new(data_set::current().rules.clone(), &[])))
}

fn load_edits(app: &AppHandle) -> Vec<RuleEdit> {
    store::load(app, &store::RULE_EDITS).unwrap_or_else(|e| {
        warn!("Could not load medical rule edits, using the rules without them: {e}");
        Vec::new()
    })
}

/// Finds the data set in use, then parses its rules and loads the saved
/// edits off the main thread (parsing ~25,000 rules takes a fraction of a
/// second). Call once at startup.
pub fn init(app: &AppHandle) {
    data_set::init(app);
    let app = app.clone();
    std::thread::spawn(move || activate(&load_edits(&app)));
}

/// After the data set changed: activates the saved edits on the new rules
/// and re-arms the engine lists. Returns how many edits now refer to rules
/// that no longer exist (they stay saved).
fn reactivate(app: &AppHandle) -> usize {
    let edits = load_edits(app);
    activate(&edits);
    engine_lists::rearm(app);
    data_set::current().rules.orphaned(&edits).len()
}

/// Fixes drug names and medical terms that speech recognition mishears.
pub fn apply_medical_fixes(text: String) -> String {
    active().apply(&text)
}

pub fn get_edits(app: &AppHandle) -> Result<Vec<RuleEdit>, String> {
    store::load(app, &store::RULE_EDITS)
}

/// Validates, saves and activates the edits. Nothing is saved if one is invalid.
pub fn set_edits(app: &AppHandle, edits: Vec<RuleEdit>) -> Result<(), String> {
    for edit in &edits {
        validate_edit(edit).map_err(|e| format!("{}: {e}", edit.replacement))?;
    }
    store::save(app, &store::RULE_EDITS, &edits)?;
    activate(&edits);
    Ok(())
}

/// Original patterns of the saved edits whose rule the active data set no
/// longer has.
pub fn orphaned_edits(app: &AppHandle) -> Result<Vec<String>, String> {
    let edits = get_edits(app)?;
    Ok(data_set::current()
        .rules
        .orphaned(&edits)
        .into_iter()
        .filter_map(|e| e.original.clone())
        .collect())
}

/// Active rules matching `query`, see [`drug_rules::Bundled::search`].
pub fn search(query: &str, limit: usize) -> Vec<RuleView> {
    data_set::current().rules.search(query, limit)
}

/// The user's changes to the list of entry type `T`.
pub fn get_overlay<T: OverlayEntry>(app: &AppHandle) -> Result<ListOverlay<T>, String> {
    store::load(app, T::LIST.store_file())
}

/// Validates, saves and arms the changes. Nothing is saved if one is invalid.
pub fn set_overlay<T: OverlayEntry>(
    app: &AppHandle,
    overlay: ListOverlay<T>,
) -> Result<(), String> {
    overlay.validate()?;
    store::save(app, T::LIST.store_file(), &overlay)?;
    engine_lists::rearm_list(app, T::LIST);
    Ok(())
}

/// The entries of the list matching `query`, with the user's changes.
pub fn search_list<T: OverlayEntry>(
    app: &AppHandle,
    query: &str,
    offset: usize,
    limit: usize,
) -> Result<ListPage<T>, String> {
    let views = get_overlay::<T>(app)?.views(T::LIST.bundled(&data_set::current()));
    Ok(list_overlay::search(views, query, offset, limit))
}

fn overlaid<T: OverlayEntry>(app: &AppHandle, data: &data_set::ActiveData) -> String {
    let overlay = get_overlay::<T>(app).unwrap_or_else(|e| {
        warn!("Could not load the changes to a medical list, using it without them: {e}");
        ListOverlay::default()
    });
    overlay.apply(T::LIST.bundled(data))
}

/// The list in `data` with the user's changes on top, as the engine gets it.
pub fn overlaid_list(app: &AppHandle, list: MedicalList, data: &data_set::ActiveData) -> String {
    match list {
        MedicalList::Phrases => overlaid::<Phrase>(app, data),
        MedicalList::ConfusionGroups => overlaid::<ConfusionGroup>(app, data),
    }
}

/// A list the Corrections page exports and imports, in its native format.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListFile {
    Rules,
    Phrases,
    ConfusionGroups,
}

/// The list's content in use (data set plus the user's changes).
pub fn export_list(app: &AppHandle, list: ListFile) -> Result<String, String> {
    let data = data_set::current();
    match list {
        ListFile::Rules => Ok(data.rules.export_jsonl(&get_edits(app)?)),
        ListFile::Phrases => Ok(overlaid_list(app, MedicalList::Phrases, &data)),
        ListFile::ConfusionGroups => Ok(overlaid_list(app, MedicalList::ConfusionGroups, &data)),
    }
}

/// At most this many import errors are listed, then a count of the others.
const MAX_LISTED_ERRORS: usize = 5;

fn import_errors(errors: Vec<String>) -> String {
    let more = errors.len().saturating_sub(MAX_LISTED_ERRORS);
    let mut listed: Vec<String> = errors.into_iter().take(MAX_LISTED_ERRORS).collect();
    if more > 0 {
        listed.push(format!("... and {more} more"));
    }
    listed.join("\n")
}

fn import_overlay<T: OverlayEntry>(app: &AppHandle, content: &str) -> Result<(), String> {
    let bundled = data_set::current();
    let overlay =
        ListOverlay::<T>::diff(T::LIST.bundled(&bundled), content).map_err(import_errors)?;
    set_overlay(app, overlay)
}

/// Stores `content` (the list's native format) as the user's changes: the
/// difference with the data in use. Nothing is saved when a line is invalid.
pub fn import_list(app: &AppHandle, list: ListFile, content: &str) -> Result<(), String> {
    match list {
        ListFile::Rules => {
            let edits = data_set::current()
                .rules
                .import_jsonl(content)
                .map_err(import_errors)?;
            set_edits(app, edits)
        }
        ListFile::Phrases => import_overlay::<Phrase>(app, content),
        ListFile::ConfusionGroups => import_overlay::<ConfusionGroup>(app, content),
    }
}
