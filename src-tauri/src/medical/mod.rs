//! French medical post-processing (Plumed fork only, not for upstream).
//!
//! The drug and medical-term fix rules (bundled, or a newer downloaded set,
//! see `data_set`), plus the user's edits to them, kept apart from the user's
//! own formatting rules: those run after, and an import or a reset of either
//! side leaves the other alone.

pub mod data_set;
mod drug_rules;
pub mod engine_lists;
mod interhop;
mod store;

use drug_rules::DrugRules;
pub use drug_rules::{validate_edit, RuleEdit, RuleView};
pub use interhop::default_rules as default_formatting_rules;
use log::warn;
use std::sync::{Arc, RwLock};
use tauri::AppHandle;

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
    store::load(app).unwrap_or_else(|e| {
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

/// Fixes drug names and medical terms that speech recognition mishears.
pub fn apply_medical_fixes(text: String) -> String {
    active().apply(&text)
}

pub fn get_edits(app: &AppHandle) -> Result<Vec<RuleEdit>, String> {
    store::load(app)
}

/// Validates, saves and activates the edits. Nothing is saved if one is invalid.
pub fn set_edits(app: &AppHandle, edits: Vec<RuleEdit>) -> Result<(), String> {
    for edit in &edits {
        validate_edit(edit).map_err(|e| format!("{}: {e}", edit.replacement))?;
    }
    store::save(app, &edits)?;
    activate(&edits);
    Ok(())
}

/// Original patterns of the saved edits whose rule the active data set no
/// longer has.
pub fn orphaned_edits(app: &AppHandle) -> Result<Vec<String>, String> {
    let edits = store::load(app)?;
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
