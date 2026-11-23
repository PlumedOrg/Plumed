//! The user's changes to the three medical lists of the Corrections page, as
//! the settings export (`.murmure` file) carries them: one category per list,
//! `medical_phrases` (Medical vocabulary tab), `medical_confusion_groups`
//! (Sound-alike words tab), `medical_rules` (Residual errors tab) and
//! `grammar_rules` (Grammar tab, the rules turned off and on) and
//! `text_templates` (Text templates tab). Only the
//! changes travel, never the bundled lists (the phrase list alone has ~77,000
//! entries), so a file stays small and applies on top of whatever medical
//! data the importing computer uses, like the per-tab files do.
//!
//! The frontend import and the CLI import (`plumed import`) both go through
//! [`import`], which saves with the same functions as the tabs
//! ([`super::set_overlay`], [`super::set_edits`]) after the same validation.
//! These lists hold text rules only: nothing here can turn on a network
//! feature or change a relay URL. Written with Claude Code.

use super::list_overlay::{ListOverlay, OverlayEntry, Phrase};
use super::text_templates::TextTemplate;
use super::RuleEdit;
use crate::engine::confusion::ConfusionGroup;
use crate::grammar::rules::GrammarOverlay;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

/// The changes of each list, `None` for a list the file does not carry
/// (an export where it was not selected, or a file from before these
/// categories existed).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MedicalChanges {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medical_phrases: Option<ListOverlay<Phrase>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medical_confusion_groups: Option<ListOverlay<ConfusionGroup>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medical_rules: Option<Vec<RuleEdit>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grammar_rules: Option<GrammarOverlay>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_templates: Option<ListOverlay<TextTemplate>>,
}

impl MedicalChanges {
    /// Keys of the categories present, in the order of the Corrections tabs,
    /// e.g. `["medical_phrases", "medical_rules"]`.
    pub fn categories(&self) -> Vec<&'static str> {
        [
            ("medical_phrases", self.medical_phrases.is_some()),
            (
                "medical_confusion_groups",
                self.medical_confusion_groups.is_some(),
            ),
            ("medical_rules", self.medical_rules.is_some()),
            ("grammar_rules", self.grammar_rules.is_some()),
            ("text_templates", self.text_templates.is_some()),
        ]
        .into_iter()
        .filter_map(|(key, present)| present.then_some(key))
        .collect()
    }
}

/// `current` with each entry of `imported` added: an imported entry replaces
/// the current one with the same key, the others are appended in their order.
/// Example with keys as the entries: current [a, b], imported [b', c] gives
/// [a, b', c].
fn merged<T, K: PartialEq>(current: Vec<T>, imported: Vec<T>, key: impl Fn(&T) -> K) -> Vec<T> {
    let mut merged = current;
    for entry in imported {
        match merged.iter().position(|kept| key(kept) == key(&entry)) {
            Some(index) => merged[index] = entry,
            None => merged.push(entry),
        }
    }
    merged
}

fn merged_overlay<T: OverlayEntry>(
    current: ListOverlay<T>,
    imported: ListOverlay<T>,
) -> ListOverlay<T> {
    ListOverlay {
        disabled: merged(current.disabled, imported.disabled, String::clone),
        added: merged(current.added, imported.added, T::key),
    }
}

/// What a rule edit stands for: the bundled rule it changes, or, for a rule
/// of the user, its pattern. Two edits with the same key cannot both apply.
fn edit_key(edit: &RuleEdit) -> (Option<String>, Option<String>) {
    match &edit.original {
        Some(original) => (Some(original.clone()), None),
        None => (None, Some(edit.core.clone())),
    }
}

/// The changes to save: the imported ones replace the current ones, or are
/// merged into them. Each list is then validated exactly like its tab
/// validates before saving, all of them before anything is saved, so an
/// invalid list leaves every list as it was.
fn resolve(
    current: MedicalChanges,
    imported: MedicalChanges,
    merge: bool,
) -> Result<MedicalChanges, String> {
    let resolved = match merge {
        false => imported,
        true => MedicalChanges {
            medical_phrases: imported.medical_phrases.map(|overlay| {
                merged_overlay(current.medical_phrases.unwrap_or_default(), overlay)
            }),
            medical_confusion_groups: imported.medical_confusion_groups.map(|overlay| {
                merged_overlay(
                    current.medical_confusion_groups.unwrap_or_default(),
                    overlay,
                )
            }),
            medical_rules: imported
                .medical_rules
                .map(|edits| merged(current.medical_rules.unwrap_or_default(), edits, edit_key)),
            grammar_rules: imported.grammar_rules.map(|overlay| {
                let current = current.grammar_rules.unwrap_or_default();
                GrammarOverlay {
                    disabled: merged(current.disabled, overlay.disabled, String::clone),
                    added: merged(current.added, overlay.added, String::clone),
                }
            }),
            text_templates: imported
                .text_templates
                .map(|overlay| merged_overlay(current.text_templates.unwrap_or_default(), overlay)),
        },
    };
    if let Some(overlay) = &resolved.medical_phrases {
        overlay.validate()?;
    }
    if let Some(overlay) = &resolved.medical_confusion_groups {
        overlay.validate()?;
    }
    if let Some(edits) = &resolved.medical_rules {
        super::validate_edits(edits)?;
    }
    if let Some(overlay) = &resolved.grammar_rules {
        crate::grammar::rules::validate_overlay(overlay, &crate::grammar::rules::DATA.rules)?;
    }
    if let Some(overlay) = &resolved.text_templates {
        overlay.validate()?;
    }
    Ok(resolved)
}

/// The saved changes of the lists, for an export.
pub fn current(app: &AppHandle) -> Result<MedicalChanges, String> {
    Ok(MedicalChanges {
        medical_phrases: Some(super::get_overlay(app)?),
        medical_confusion_groups: Some(super::get_overlay(app)?),
        medical_rules: Some(super::get_edits(app)?),
        grammar_rules: Some(crate::grammar::get_overlay(app)?),
        text_templates: Some(super::get_overlay(app)?),
    })
}

/// Saves the lists `imported` carries (replacing or merged into the saved
/// changes) and re-arms them, like a change made in their tab.
pub fn import(app: &AppHandle, imported: MedicalChanges, merge: bool) -> Result<(), String> {
    // Replacing needs nothing saved, so a broken store file does not block it.
    let saved = match merge {
        true => current(app)?,
        false => MedicalChanges::default(),
    };
    let resolved = resolve(saved, imported, merge)?;
    if let Some(overlay) = resolved.medical_phrases {
        super::set_overlay(app, overlay)?;
    }
    if let Some(overlay) = resolved.medical_confusion_groups {
        super::set_overlay(app, overlay)?;
    }
    if let Some(edits) = resolved.medical_rules {
        super::set_edits(app, edits)?;
    }
    if let Some(overlay) = resolved.grammar_rules {
        crate::grammar::set_overlay(app, overlay)?;
    }
    if let Some(overlay) = resolved.text_templates {
        super::set_overlay(app, overlay)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phrases(disabled: &[&str], added: &[&str]) -> ListOverlay<Phrase> {
        ListOverlay {
            disabled: disabled.iter().map(|k| k.to_string()).collect(),
            added: added.iter().map(|p| Phrase(p.to_string())).collect(),
        }
    }

    fn templates(disabled: &[&str], added: &[(&str, &str)]) -> ListOverlay<TextTemplate> {
        ListOverlay {
            disabled: disabled.iter().map(|k| k.to_string()).collect(),
            added: added
                .iter()
                .map(|(trigger, text)| TextTemplate {
                    trigger: trigger.to_string(),
                    text: text.to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn an_invalid_template_refuses_the_whole_import() {
        let imported = MedicalChanges {
            medical_phrases: Some(phrases(&[], &["cetuximab"])),
            text_templates: Some(templates(&[], &[("", "x")])),
            ..Default::default()
        };
        assert!(resolve(MedicalChanges::default(), imported, false).is_err());
    }

    #[test]
    fn text_templates_are_a_category() {
        let changes = MedicalChanges {
            text_templates: Some(templates(&[], &[])),
            ..Default::default()
        };
        assert_eq!(changes.categories(), vec!["text_templates"]);
    }

    fn edit(original: Option<&str>, core: &str, replacement: &str) -> RuleEdit {
        RuleEdit {
            original: original.map(str::to_string),
            core: core.to_string(),
            replacement: replacement.to_string(),
            enabled: true,
        }
    }

    fn group(heard: &str, candidates: &[&str]) -> ConfusionGroup {
        ConfusionGroup {
            heard: heard.to_string(),
            candidates: candidates.iter().map(|c| c.to_string()).collect(),
        }
    }

    #[test]
    fn replacing_keeps_only_the_imported_lists() {
        let current = MedicalChanges {
            medical_phrases: Some(phrases(&["rituximab"], &["venlafaxine"])),
            ..Default::default()
        };
        let imported = MedicalChanges {
            medical_phrases: Some(phrases(&[], &["cetuximab"])),
            ..Default::default()
        };
        let resolved = resolve(current, imported.clone(), false).unwrap();
        assert_eq!(resolved, imported);
        // Lists the file does not carry stay untouched (None: not saved).
        assert!(resolved.medical_rules.is_none());
    }

    #[test]
    fn merging_adds_and_replaces_by_key() {
        let current = MedicalChanges {
            medical_phrases: Some(phrases(&["rituximab"], &["venlafaxine"])),
            medical_confusion_groups: Some(ListOverlay {
                disabled: Vec::new(),
                added: vec![group("ituximab", &["rituximab", "cetuximab"])],
            }),
            medical_rules: Some(vec![
                edit(
                    Some("\\bveine lafaxine\\b"),
                    "veine lafaxine",
                    "venlafaxine",
                ),
                edit(None, "dolipran", "Doliprane"),
            ]),
            grammar_rules: None,
            text_templates: Some(templates(
                &["insere l'examen clinique normal."],
                &[("a", "x")],
            )),
        };
        let changed_group = group("ituximab", &["rituximab", "cetuximab", "infliximab"]);
        let imported = MedicalChanges {
            medical_phrases: Some(phrases(&["rituximab", "cetuximab"], &["sertraline"])),
            medical_confusion_groups: Some(ListOverlay {
                disabled: Vec::new(),
                added: vec![changed_group.clone()],
            }),
            medical_rules: Some(vec![
                edit(
                    Some("\\bveine lafaxine\\b"),
                    "veine lafaxine",
                    "Venlafaxine",
                ),
                edit(None, "levotirox", "Levothyrox"),
            ]),
            grammar_rules: None,
            text_templates: Some(templates(&[], &[("a", "y"), ("b", "z")])),
        };
        let resolved = resolve(current, imported, true).unwrap();
        assert_eq!(
            resolved.text_templates,
            Some(templates(
                &["insere l'examen clinique normal."],
                &[("a", "y"), ("b", "z")]
            ))
        );
        assert_eq!(
            resolved.medical_phrases,
            Some(phrases(
                &["rituximab", "cetuximab"],
                &["venlafaxine", "sertraline"]
            ))
        );
        assert_eq!(
            resolved.medical_confusion_groups.unwrap().added,
            vec![changed_group]
        );
        assert_eq!(
            resolved.medical_rules,
            Some(vec![
                edit(
                    Some("\\bveine lafaxine\\b"),
                    "veine lafaxine",
                    "Venlafaxine"
                ),
                edit(None, "dolipran", "Doliprane"),
                edit(None, "levotirox", "Levothyrox"),
            ])
        );
    }

    #[test]
    fn an_invalid_list_is_refused_whole() {
        // A valid phrase list next to an invalid rule: resolve fails, so
        // `import` saves neither (it saves only after resolve succeeded).
        let imported = MedicalChanges {
            medical_phrases: Some(phrases(&[], &["sertraline"])),
            medical_rules: Some(vec![edit(None, "(?<=a)b", "c")]),
            ..Default::default()
        };
        assert!(resolve(MedicalChanges::default(), imported, false).is_err());
        let one_candidate = MedicalChanges {
            medical_confusion_groups: Some(ListOverlay {
                disabled: Vec::new(),
                added: vec![group("ituximab", &["rituximab"])],
            }),
            ..Default::default()
        };
        assert!(resolve(MedicalChanges::default(), one_candidate, true).is_err());
        let bad_phrase = MedicalChanges {
            medical_phrases: Some(phrases(&[], &["ratio 3:1"])),
            ..Default::default()
        };
        assert!(resolve(MedicalChanges::default(), bad_phrase, false).is_err());
    }

    #[test]
    fn grammar_rule_changes_merge_by_id_and_are_checked() {
        let ids = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let current = MedicalChanges {
            grammar_rules: Some(GrammarOverlay {
                disabled: ids(&["g2__locutions_invariables"]),
                added: ids(&["g3__gn_les_2m"]),
            }),
            ..Default::default()
        };
        let imported = MedicalChanges {
            grammar_rules: Some(GrammarOverlay {
                disabled: ids(&["g2__locutions_invariables", "g3__gn_couleurs_composées"]),
                added: Vec::new(),
            }),
            ..Default::default()
        };
        let resolved = resolve(current, imported.clone(), true).unwrap();
        assert_eq!(
            resolved.grammar_rules,
            Some(GrammarOverlay {
                disabled: ids(&["g2__locutions_invariables", "g3__gn_couleurs_composées"]),
                added: ids(&["g3__gn_les_2m"]),
            })
        );
        assert_eq!(
            resolve(MedicalChanges::default(), imported.clone(), false).unwrap(),
            imported
        );
        let junk = MedicalChanges {
            grammar_rules: Some(GrammarOverlay {
                disabled: Vec::new(),
                added: ids(&["bad\u{7}id"]),
            }),
            ..Default::default()
        };
        assert!(resolve(MedicalChanges::default(), junk, false).is_err());
    }

    #[test]
    fn categories_follow_the_tab_order() {
        let changes = MedicalChanges {
            medical_rules: Some(Vec::new()),
            medical_phrases: Some(ListOverlay::default()),
            ..Default::default()
        };
        assert_eq!(
            changes.categories(),
            vec!["medical_phrases", "medical_rules"]
        );
        assert!(MedicalChanges::default().categories().is_empty());
    }
}
