//! Text templates (Plumed fork only, written with Claude Code): a spoken
//! trigger phrase replaced by a longer, possibly multi-line text, for example
//! "insère l'examen clinique normal" and a whole normal examination.
//!
//! A template is an entry of an overlay list like the phrase list (`list_overlay`):
//! bundled examples that can be turned off, and templates of the user, saved in
//! `text_templates.json`. The bundled examples start turned off
//! ([`OverlayEntry::OFF_BY_DEFAULT`]). Expansion reuses the formatting rules
//! engine in smart mode (case-insensitive, surrounding punctuation handled), and
//! runs last, as step 8 after the grammar, so the text is inserted as written.

use super::drug_rules::fold;
use super::engine_lists::MedicalList;
use super::list_overlay::{ListOverlay, OverlayEntry};
use crate::formatting_rules::{apply_custom_rule, MatchMode};
use crate::utils::user_text::{has_unsafe_chars, has_unsafe_chars_besides_lines};
use log::warn;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tauri::AppHandle;

/// The templates that ship with the app, one JSON object per line.
pub const BUNDLED: &str = include_str!("../../medical-data/text-templates/templates.jsonl");

/// A spoken trigger and the text it expands to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextTemplate {
    pub trigger: String,
    /// May span several lines.
    pub text: String,
}

impl OverlayEntry for TextTemplate {
    const LIST: MedicalList = MedicalList::Templates;
    const OFF_BY_DEFAULT: bool = true;

    /// Accent, case and edge space insensitive: "Insère" and "insere " are the same trigger.
    fn key(&self) -> String {
        fold(self.trigger.trim())
    }

    fn parse_line(line: &str) -> Result<Option<Self>, String> {
        match line.trim() {
            "" => Ok(None),
            json => serde_json::from_str(json)
                .map(Some)
                .map_err(|e| format!("not a template ({e})")),
        }
    }

    fn to_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    fn validate(&self) -> Result<(), String> {
        if has_unsafe_chars(&self.trigger) {
            return Err(format!(
                "\"{}\": the trigger cannot contain control characters",
                self.trigger.escape_debug()
            ));
        }
        if has_unsafe_chars_besides_lines(&self.text) {
            return Err(format!(
                "\"{}\": the text cannot contain control characters",
                self.trigger
            ));
        }
        match (self.key().is_empty(), self.text.trim().is_empty()) {
            (true, _) => Err("The trigger is empty".to_string()),
            (_, true) => Err(format!("\"{}\": the text is empty", self.trigger)),
            _ => Ok(()),
        }
    }

    fn search_text(&self) -> String {
        format!("{} {}", self.trigger, self.text)
    }

    fn from_key(key: &str) -> Self {
        TextTemplate {
            trigger: key.to_string(),
            text: String::new(),
        }
    }
}

/// The trigger as the matcher gets it: the model may or may not end a dictated
/// phrase with a period, and smart mode already accepts one after the match.
fn spoken_form(trigger: &str) -> &str {
    trigger
        .trim()
        .trim_end_matches(['.', ',', ';', ':', '!', '?'])
}

/// Replaces each trigger found in `text` by its template, in order.
fn expand(templates: &[TextTemplate], text: String) -> String {
    templates.iter().fold(text, |text, template| {
        match spoken_form(&template.trigger).is_empty() {
            true => text,
            false => apply_custom_rule(
                &text,
                spoken_form(&template.trigger),
                &template.text,
                &MatchMode::Smart,
            ),
        }
    })
}

/// The enabled templates [`apply`] runs.
static ACTIVE: RwLock<Option<Arc<Vec<TextTemplate>>>> = RwLock::new(None);

/// The enabled templates of `overlay` on the bundled ones.
fn enabled(overlay: &ListOverlay<TextTemplate>) -> Vec<TextTemplate> {
    overlay
        .apply(BUNDLED)
        .lines()
        .filter_map(|line| TextTemplate::parse_line(line).ok().flatten())
        .collect()
}

/// The overlay a fresh install has: every bundled template turned off.
pub fn initial_overlay() -> ListOverlay<TextTemplate> {
    ListOverlay::initial(BUNDLED)
}

fn current_overlay(app: &AppHandle) -> ListOverlay<TextTemplate> {
    super::get_overlay(app).unwrap_or_else(|e| {
        warn!("Could not load the text templates, using none: {e}");
        ListOverlay::default()
    })
}

/// Loads the saved templates and makes them the active ones.
pub fn activate(app: &AppHandle) {
    let templates = Arc::new(enabled(&current_overlay(app)));
    match ACTIVE.write() {
        Ok(mut active) => *active = Some(templates),
        Err(e) => warn!("Text templates lock poisoned: {e}"),
    }
}

/// How many templates are enabled (the usage statistics bucket this).
pub fn enabled_count(app: &AppHandle) -> usize {
    enabled(&current_overlay(app)).len()
}

/// Expands the spoken templates found in `text` (nothing before [`activate`] ran).
pub fn apply(text: String) -> String {
    let active = ACTIVE.read().ok().and_then(|a| a.clone());
    match active {
        Some(templates) if !templates.is_empty() => expand(&templates, text),
        _ => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template(trigger: &str, text: &str) -> TextTemplate {
        TextTemplate {
            trigger: trigger.to_string(),
            text: text.to_string(),
        }
    }

    fn exam() -> Vec<TextTemplate> {
        vec![template("Insère l'examen normal.", "Examen :\n- RAS.")]
    }

    #[test]
    fn expands_a_trigger_and_keeps_the_line_breaks() {
        let out = expand(&exam(), "Insère l'examen normal.".to_string());
        assert_eq!(out, "Examen :\n- RAS.");
    }

    #[test]
    fn matching_ignores_case_and_a_missing_or_extra_final_period() {
        for heard in [
            "insère l'examen normal",
            "Insère l'examen normal.",
            "INSÈRE L'EXAMEN NORMAL,",
        ] {
            assert_eq!(
                expand(&exam(), heard.to_string()),
                "Examen :\n- RAS.",
                "{heard}"
            );
        }
    }

    #[test]
    fn keeps_the_text_around_the_trigger() {
        let out = expand(
            &exam(),
            "Bonjour. Insère l'examen normal. Merci.".to_string(),
        );
        assert_eq!(out, "Bonjour. Examen :\n- RAS. Merci.");
    }

    #[test]
    fn a_dollar_sign_in_the_text_is_kept_literally() {
        let out = expand(&[template("prix", "5 $1 $$")], "prix".to_string());
        assert_eq!(out, "5 $1 $$");
    }

    #[test]
    fn nothing_changes_without_a_trigger() {
        assert_eq!(
            expand(&exam(), "Rien à signaler".to_string()),
            "Rien à signaler"
        );
        assert_eq!(expand(&[], "a".to_string()), "a");
    }

    #[test]
    fn a_trigger_made_of_punctuation_only_is_ignored() {
        assert_eq!(expand(&[template(" . ", "x")], "a. b".to_string()), "a. b");
    }

    #[test]
    fn the_bundled_example_is_valid_and_complete() {
        let bundled: Vec<TextTemplate> = BUNDLED
            .lines()
            .filter_map(|l| TextTemplate::parse_line(l).unwrap())
            .collect();
        assert_eq!(bundled.len(), 1);
        let example = &bundled[0];
        assert_eq!(example.validate(), Ok(()));
        assert_eq!(example.trigger, "Insère l'examen clinique normal.");
        assert!(example
            .text
            .starts_with("Examen clinique :\nÉtat général :"));
        for heading in [
            "Cardiovasculaire :",
            "Pulmonaire :",
            "Abdominal :",
            "Neurologique :",
        ] {
            assert!(
                example.text.contains(&format!("\n{heading}\n- ")),
                "{heading}"
            );
        }
        assert!(example
            .text
            .contains("\n- Bruits du cœur réguliers, sans souffle.\n"));
        assert!(!example.text.contains('—'));
    }

    #[test]
    fn the_bundled_example_starts_turned_off() {
        let initial = initial_overlay();
        assert_eq!(initial.disabled, vec!["insere l'examen clinique normal."]);
        assert!(enabled(&initial).is_empty());
        let on: ListOverlay<TextTemplate> = ListOverlay::default();
        assert_eq!(enabled(&on).len(), 1);
    }

    #[test]
    fn expanding_the_bundled_example_after_turning_it_on() {
        let on: ListOverlay<TextTemplate> = ListOverlay::default();
        let out = expand(
            &enabled(&on),
            "Insère l'examen clinique normal.".to_string(),
        );
        assert!(out.starts_with("Examen clinique :\n"));
        assert!(out.contains("Bruits du cœur réguliers"));
    }

    #[test]
    fn validation_refuses_empty_and_unsafe_entries() {
        assert!(template("", "x").validate().is_err());
        assert!(template("a", " \n ").validate().is_err());
        assert!(template("a\u{202e}b", "x").validate().is_err());
        assert!(template("a\nb", "x").validate().is_err());
        assert!(template("a", "x\u{202e}").validate().is_err());
        assert_eq!(template("a", "l1\nl2\tl3").validate(), Ok(()));
    }

    #[test]
    fn keys_ignore_accents_case_and_edge_spaces() {
        assert_eq!(
            template(" Insère ", "x").key(),
            template("insere", "y").key()
        );
    }

    #[test]
    fn an_export_round_trips_through_the_native_format() {
        let overlay = ListOverlay {
            disabled: vec![],
            added: vec![template("mon texte", "ligne 1\nligne 2 \"citée\"")],
        };
        let exported = overlay.apply(BUNDLED);
        let back = ListOverlay::<TextTemplate>::diff(BUNDLED, &exported).unwrap();
        assert_eq!(back, overlay);
    }

    #[test]
    fn importing_a_file_without_the_bundled_example_turns_it_off() {
        let file = template("mon texte", "x").to_line();
        let back = ListOverlay::<TextTemplate>::diff(BUNDLED, &file).unwrap();
        assert_eq!(back.disabled, initial_overlay().disabled);
        assert_eq!(back.added.len(), 1);
    }

    #[test]
    fn a_template_over_the_entry_limit_is_refused() {
        let overlay = ListOverlay {
            disabled: vec![],
            added: (0..=crate::utils::user_text::MAX_USER_ENTRIES)
                .map(|i| template(&format!("t{i}"), "x"))
                .collect(),
        };
        assert!(overlay.validate().is_err());
    }
}
