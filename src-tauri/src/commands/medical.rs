use crate::engine::confusion::ConfusionGroup;
use crate::formatting_rules::types::FormattingRule;
use crate::medical::list_overlay::{ListOverlay, ListPage, Phrase};
use crate::medical::{self, ListFile, RuleEdit, RuleView};
use tauri::{command, AppHandle};

/// Most search results the settings UI shows at once.
const SEARCH_LIMIT: usize = 50;

#[command]
pub fn get_medical_rule_edits(app: AppHandle) -> Result<Vec<RuleEdit>, String> {
    medical::get_edits(&app)
}

/// Saves the whole list of edits; an empty list resets to the bundled rules.
#[command]
pub fn set_medical_rule_edits(app: AppHandle, edits: Vec<RuleEdit>) -> Result<(), String> {
    medical::set_edits(&app, edits)
}

/// Original patterns of the saved edits whose rule the medical data in use
/// no longer has (after a data update). They stay saved.
#[command]
pub fn get_orphaned_medical_rule_edits(app: AppHandle) -> Result<Vec<String>, String> {
    medical::orphaned_edits(&app)
}

#[command]
pub fn search_medical_rules(query: String) -> Vec<RuleView> {
    medical::search(&query, SEARCH_LIMIT)
}

/// What the medical rules (with saved edits) make of `text`, to try them out.
#[command]
pub fn preview_medical_fixes(text: String) -> String {
    medical::apply_medical_fixes(text)
}

/// The InterHop rules a fresh install starts with, to reset the user's
/// formatting rules to them.
#[command]
pub fn get_default_formatting_rules() -> Vec<FormattingRule> {
    medical::default_formatting_rules()
}

#[command]
pub fn get_phrase_overlay(app: AppHandle) -> Result<ListOverlay<Phrase>, String> {
    medical::get_overlay(&app)
}

/// Saves the user's changes to the phrase list and re-arms it (async: the
/// list takes a moment to build, off the main thread).
#[command]
pub async fn set_phrase_overlay(
    app: AppHandle,
    overlay: ListOverlay<Phrase>,
) -> Result<(), String> {
    medical::set_overlay(&app, overlay)
}

#[command]
pub fn search_phrases(
    app: AppHandle,
    query: String,
    offset: usize,
    limit: usize,
) -> Result<ListPage<Phrase>, String> {
    medical::search_list(&app, &query, offset, limit)
}

#[command]
pub fn get_confusion_overlay(app: AppHandle) -> Result<ListOverlay<ConfusionGroup>, String> {
    medical::get_overlay(&app)
}

#[command]
pub async fn set_confusion_overlay(
    app: AppHandle,
    overlay: ListOverlay<ConfusionGroup>,
) -> Result<(), String> {
    medical::set_overlay(&app, overlay)
}

#[command]
pub fn search_confusion_groups(
    app: AppHandle,
    query: String,
    offset: usize,
    limit: usize,
) -> Result<ListPage<ConfusionGroup>, String> {
    medical::search_list(&app, &query, offset, limit)
}

/// Writes the list in use (with the user's changes) to `file_path`.
#[command]
pub async fn export_medical_list(
    app: AppHandle,
    list: ListFile,
    file_path: String,
) -> Result<(), String> {
    let content = medical::export_list(&app, list)?;
    std::fs::write(&file_path, content + "\n").map_err(|e| e.to_string())
}

/// Reads a list file and saves it as the user's changes, or nothing at all
/// when a line is invalid.
#[command]
pub async fn import_medical_list(
    app: AppHandle,
    list: ListFile,
    file_path: String,
) -> Result<(), String> {
    let content = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
    medical::import_list(&app, list, &content)
}
