//! Plumed fork: the grammar step's rules (Corrections tab 7). Export and import of the
//! list go through the medical list commands (`ListFile::Grammar`). Written with Claude Code.

use crate::grammar::{self, rules::GrammarOverlay, GrammarRules};
use tauri::{command, AppHandle};

#[command]
pub fn get_grammar_rules(app: AppHandle) -> Result<GrammarRules, String> {
    grammar::get_rules(&app)
}

/// Saves the rules turned on (catalog ids) as the user's changes to the defaults.
#[command]
pub fn set_grammar_rules(app: AppHandle, enabled: Vec<String>) -> Result<(), String> {
    grammar::set_enabled(&app, enabled)
}

/// Back to the default rules.
#[command]
pub fn reset_grammar_rules(app: AppHandle) -> Result<(), String> {
    grammar::set_overlay(&app, GrammarOverlay::default())
}
