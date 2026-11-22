//! Which Grammalecte rules the grammar step applies. `grammar-data/grammar_rules.json`
//! holds the catalog the Corrections tab shows (the 8 rules applied by default, then every
//! other grammar family, each with what it did on the UltiMed dev set with the masks on),
//! the masks and the error types never applied (typography). The user's changes are an
//! overlay on the defaults: default rules turned off by id, other rules turned on.
//! Written with Claude Code.

use crate::medical::list_overlay::ListOverlay;
use crate::utils::user_text::has_unsafe_chars;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const RULES_JSON: &str = include_str!("../../grammar-data/grammar_rules.json");

/// How a rule id of the catalog matches Grammalecte's rule ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleLevel {
    /// Every rule id whose part before the second `__` is this id.
    Family,
    /// This exact rule id.
    Rule,
}

/// One entry of the catalog, with its measured effect: `n` single-suggestion fixes, of
/// which `better` brought the text closer to the reference and `worse` further from it;
/// `precision` = better / (better + worse), `None` when it never fired with one suggestion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleInfo {
    pub id: String,
    pub level: RuleLevel,
    pub n: u32,
    pub better: u32,
    pub worse: u32,
    pub precision: Option<f64>,
    /// Applied unless the user turns it off.
    pub default: bool,
}

/// A mask of the spec: a regex in Python `re` syntax, or the spell checker's unknown words.
#[derive(Debug, Clone, Deserialize)]
pub struct MaskSpec {
    pub name: String,
    pub action: MaskAction,
    pub case_sensitive: bool,
    #[serde(default)]
    pub regex: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MaskAction {
    /// Deleted from what Grammalecte sees.
    Replace,
    /// Left as is, but a fix touching it is dropped.
    Guard,
}

#[derive(Debug, Deserialize)]
pub struct GrammarData {
    pub grammalecte_version: String,
    pub skip_types: Vec<String>,
    pub masks: Vec<MaskSpec>,
    pub rules: Vec<RuleInfo>,
}

pub static DATA: Lazy<GrammarData> = Lazy::new(|| {
    // A unit test parses the same bundled file, so this cannot fail in a build that passed.
    serde_json::from_str(RULES_JSON).unwrap_or_else(|e| {
        log::error!("Invalid grammar_rules.json: {e}");
        GrammarData {
            grammalecte_version: String::new(),
            skip_types: Vec::new(),
            masks: Vec::new(),
            rules: Vec::new(),
        }
    })
});

/// The user's changes: ids of default rules turned off (`disabled`) and of other rules
/// turned on (`added`).
pub type GrammarOverlay = ListOverlay<String>;

/// Grammalecte's family of a rule id: the part before the second `__`, e.g.
/// "g3__gn_ces_aux_pluriel_1m__b1_a3_1" -> "g3__gn_ces_aux_pluriel_1m"; an id without `__`
/// loses its trailing digits ("typo12" -> "typo"). Same as the evaluation's `family()`.
pub fn family(rule_id: &str) -> String {
    let parts: Vec<&str> = rule_id.split("__").collect();
    match parts.len() {
        1 => rule_id
            .trim_end_matches(|c: char| c.is_ascii_digit())
            .to_string(),
        _ => parts[..2].join("__"),
    }
}

/// The rules the step applies.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RuleSet {
    enabled: HashSet<String>,
}

impl RuleSet {
    pub fn new<I: IntoIterator<Item = String>>(ids: I) -> Self {
        Self {
            enabled: ids.into_iter().collect(),
        }
    }

    /// The defaults with the user's changes: defaults not turned off, then turned on ids.
    pub fn from_overlay(rules: &[RuleInfo], overlay: &GrammarOverlay) -> Self {
        let disabled: HashSet<&str> = overlay.disabled.iter().map(String::as_str).collect();
        let defaults = rules
            .iter()
            .filter(|r| r.default && !disabled.contains(r.id.as_str()))
            .map(|r| r.id.clone());
        Self::new(defaults.chain(overlay.added.iter().cloned()))
    }

    /// Whether this catalog id itself is on.
    pub fn contains(&self, id: &str) -> bool {
        self.enabled.contains(id)
    }

    /// The ids on, sorted.
    pub fn ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.enabled.iter().cloned().collect();
        ids.sort();
        ids
    }

    pub fn is_empty(&self) -> bool {
        self.enabled.is_empty()
    }

    /// Whether an error of `rule_id` may be applied: its family or the id itself is on.
    pub fn allows(&self, rule_id: &str) -> bool {
        self.enabled.contains(&family(rule_id)) || self.enabled.contains(rule_id)
    }
}

/// Rule ids are short ASCII-ish names; anything else in an imported file is refused.
const MAX_RULE_ID_LEN: usize = 200;

/// Checks the user's changes before saving them (also an imported file's).
pub fn validate_overlay(overlay: &GrammarOverlay, rules: &[RuleInfo]) -> Result<(), String> {
    // Twice the catalog leaves room for ids of a newer catalog, not for junk.
    let max = 2 * rules.len().max(1);
    if overlay.disabled.len() + overlay.added.len() > max {
        return Err(format!("too many grammar rule changes (at most {max})"));
    }
    match overlay
        .disabled
        .iter()
        .chain(&overlay.added)
        .find(|id| id.is_empty() || id.len() > MAX_RULE_ID_LEN || has_unsafe_chars(id))
    {
        Some(id) => Err(format!(
            "\"{}\" is not a grammar rule id",
            id.escape_debug()
        )),
        None => Ok(()),
    }
}

/// The overlay that gives the rules turned on in `enabled` (the tab's toggles): default
/// rules missing from it are disabled, other ids are added. Order follows the catalog.
pub fn overlay_for(rules: &[RuleInfo], enabled: &HashSet<String>) -> GrammarOverlay {
    let known: HashSet<&str> = rules.iter().map(|r| r.id.as_str()).collect();
    let disabled = rules
        .iter()
        .filter(|r| r.default && !enabled.contains(&r.id))
        .map(|r| r.id.clone())
        .collect();
    let mut added: Vec<String> = rules
        .iter()
        .filter(|r| !r.default && enabled.contains(&r.id))
        .map(|r| r.id.clone())
        .collect();
    // Ids the catalog does not know (an import from a newer catalog) stay on.
    let mut unknown: Vec<String> = enabled
        .iter()
        .filter(|id| !known.contains(id.as_str()))
        .cloned()
        .collect();
    unknown.sort();
    added.extend(unknown);
    GrammarOverlay { disabled, added }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_data_parses_with_the_eight_default_rules() {
        let data: GrammarData = serde_json::from_str(RULES_JSON).unwrap();
        assert_eq!(data.grammalecte_version, "2.3.0");
        assert_eq!(data.rules.iter().filter(|r| r.default).count(), 8);
        assert_eq!(data.masks.len(), 18);
        assert!(data.skip_types.contains(&"typo".to_string()));
        let ids: HashSet<&str> = data.rules.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids.len(), data.rules.len(), "catalog ids must be unique");
    }

    #[test]
    fn family_is_the_part_before_the_second_double_underscore() {
        assert_eq!(
            family("g3__gn_ces_aux_pluriel_1m__b1_a3_1"),
            "g3__gn_ces_aux_pluriel_1m"
        );
        assert_eq!(
            family("gv1__ppas_sujet_être_accord_singulier__b2_a1_1"),
            "gv1__ppas_sujet_être_accord_singulier"
        );
        assert_eq!(family("typo12"), "typo");
    }

    #[test]
    fn families_match_every_rule_and_rules_match_exactly() {
        let set = RuleSet::new([
            "gv1__ppas_sujet_être_accord_singulier".to_string(),
            "g3__gn_au_1m__b1_a4_1".to_string(),
        ]);
        assert!(set.allows("gv1__ppas_sujet_être_accord_singulier__b2_a1_1"));
        assert!(set.allows("g3__gn_au_1m__b1_a4_1"));
        assert!(!set.allows("g3__gn_au_1m__b1_a2_1"));
        assert!(!set.allows("g3__gn_les_1m__b3_a1_1"));
    }

    #[test]
    fn the_overlay_turns_defaults_off_and_others_on() {
        let rules = &DATA.rules;
        let default_id = rules.iter().find(|r| r.default).unwrap().id.clone();
        let other_id = rules.iter().find(|r| !r.default).unwrap().id.clone();
        let overlay = GrammarOverlay {
            disabled: vec![default_id.clone()],
            added: vec![other_id.clone()],
        };
        let set = RuleSet::from_overlay(rules, &overlay);
        assert!(!set.allows(&default_id));
        assert!(set.allows(&other_id));
        assert_eq!(
            RuleSet::from_overlay(rules, &GrammarOverlay::default())
                .enabled
                .len(),
            8
        );
        // The tab's toggles give back the same overlay.
        assert_eq!(overlay_for(rules, &set.enabled), overlay);
    }

    #[test]
    fn junk_ids_are_refused() {
        let ok = GrammarOverlay {
            disabled: vec!["g2__locutions_invariables".to_string()],
            added: Vec::new(),
        };
        assert!(validate_overlay(&ok, &DATA.rules).is_ok());
        let bad = GrammarOverlay {
            disabled: Vec::new(),
            added: vec!["g2\u{202e}x".to_string()],
        };
        assert!(validate_overlay(&bad, &DATA.rules).is_err());
        let too_many = GrammarOverlay {
            disabled: Vec::new(),
            added: (0..1000).map(|i| format!("r{i}")).collect(),
        };
        assert!(validate_overlay(&too_many, &DATA.rules).is_err());
    }
}
