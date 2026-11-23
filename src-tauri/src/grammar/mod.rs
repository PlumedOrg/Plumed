//! Plumed fork, step 7 "Grammaire" (Grammar), the last correction step of a dictation: Grammalecte
//! 2.3.0 (French grammar checker, GPL v3+, https://grammalecte.net) checks the final text
//! and only the fixes of the chosen rules with exactly one suggestion are applied, with
//! masks around misleading contexts. By default the 8 rules selected on the UltiMed
//! evaluation; the user can turn rules on and off in the Corrections tab. Everything runs
//! locally, in an embedded JavaScript engine ([`engine`]). Written with Claude Code.

mod engine;
mod mask;
mod procedure;
pub mod rules;
mod worker;

use crate::history::{PipelineStep, StepTrace};
use crate::medical::store::{self, GRAMMAR_OVERLAY};
use rules::{GrammarOverlay, RuleInfo, RuleSet, DATA};
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use tauri::AppHandle;

/// Where `scripts/fetch-grammalecte.sh` puts Grammalecte, under the app resources.
fn resources_dir(app: &AppHandle) -> Option<PathBuf> {
    crate::utils::resources::resolve_resource_path(app, "grammalecte")
}

/// The user's changes to the default rules.
pub fn get_overlay(app: &AppHandle) -> Result<GrammarOverlay, String> {
    store::load(app, &GRAMMAR_OVERLAY)
}

/// Saves the user's changes after checking them.
pub fn set_overlay(app: &AppHandle, overlay: GrammarOverlay) -> Result<(), String> {
    rules::validate_overlay(&overlay, &DATA.rules)?;
    store::save(app, &GRAMMAR_OVERLAY, &overlay)
}

/// The rules in use: the defaults when the changes cannot be read.
fn rule_set(app: &AppHandle) -> RuleSet {
    let overlay = get_overlay(app).unwrap_or_else(|e| {
        log::warn!("Could not load the grammar rule changes, using the defaults: {e}");
        GrammarOverlay::default()
    });
    RuleSet::from_overlay(&DATA.rules, &overlay)
}

/// Starts loading Grammalecte in the background when the step is on, so the first
/// dictation does not wait for it.
pub fn warm_up(app: &AppHandle) {
    let enabled = crate::formatting_rules::load(app)
        .map(|settings| settings.built_in.grammar)
        .unwrap_or(false);
    if enabled {
        worker::warm_up(|| resources_dir(app));
    }
}

/// Step 7: `text` with the grammar fixes applied when `enabled`, recorded in `trace`.
/// On any error (Grammalecte missing, too slow to answer) the text is kept as it is.
pub fn apply_traced(app: &AppHandle, text: String, enabled: bool, trace: &mut StepTrace) -> String {
    let rules = match enabled {
        true => rule_set(app),
        false => RuleSet::default(),
    };
    let result = match rules.is_empty() || text.trim().is_empty() {
        true => text,
        false => {
            let started = std::time::Instant::now();
            match worker::correct(|| resources_dir(app), &text, rules) {
                Ok(corrected) => {
                    log::debug!("Grammar step took {:?}", started.elapsed());
                    corrected
                }
                Err(e) => {
                    log::warn!("Grammar step skipped: {e}");
                    text
                }
            }
        }
    };
    trace.record(PipelineStep::Grammar, &result);
    result
}

/// A catalog entry and whether it is on, for the Corrections tab.
#[derive(Debug, Clone, Serialize)]
pub struct RuleView {
    #[serde(flatten)]
    pub info: RuleInfo,
    pub enabled: bool,
}

/// What the Corrections tab shows: Grammalecte's version and every rule with its state.
#[derive(Debug, Serialize)]
pub struct GrammarRules {
    pub grammalecte_version: String,
    pub rules: Vec<RuleView>,
}

pub fn get_rules(app: &AppHandle) -> Result<GrammarRules, String> {
    let set = RuleSet::from_overlay(&DATA.rules, &get_overlay(app)?);
    Ok(GrammarRules {
        grammalecte_version: DATA.grammalecte_version.clone(),
        rules: DATA
            .rules
            .iter()
            .map(|info| RuleView {
                enabled: set.contains(&info.id),
                info: info.clone(),
            })
            .collect(),
    })
}

/// Saves the rules turned on in the tab as the user's changes.
pub fn set_enabled(app: &AppHandle, enabled: Vec<String>) -> Result<(), String> {
    let enabled: HashSet<String> = enabled.into_iter().collect();
    set_overlay(app, rules::overlay_for(&DATA.rules, &enabled))
}

/// The rules in use, one id per line (the tab's export file).
pub fn export_list(app: &AppHandle) -> Result<String, String> {
    let overlay = get_overlay(app)?;
    Ok(RuleSet::from_overlay(&DATA.rules, &overlay)
        .ids()
        .join("\n"))
}

/// Reads an export file (one rule id per line, `#` comments and blank lines ignored) and
/// saves it as the user's changes.
pub fn import_list(app: &AppHandle, content: &str) -> Result<(), String> {
    let enabled: Vec<String> = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect();
    set_enabled(app, enabled)
}

#[cfg(test)]
mod engine_tests;
