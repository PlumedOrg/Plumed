use crate::medical::{self, RuleEdit, RuleView};
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
