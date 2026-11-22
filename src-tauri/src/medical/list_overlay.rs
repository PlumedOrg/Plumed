//! The user's changes to a list of the medical data (the phrase boosting list,
//! the confusion groups), stored apart from it like the rule edits: bundled
//! entries turned off by key, and entries of their own. Applied on top of the
//! active data set, bundled or downloaded, so a data update keeps them.
//! Written with Claude Code.

use super::engine_lists::MedicalList;
use crate::engine::confusion::ConfusionGroup;
use crate::engine::phrase_boost::list::line_phrase;
use crate::utils::user_text::{check_entry_count, has_unsafe_chars};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// An entry of a list, read from and written to one line of its native file.
pub trait OverlayEntry: Clone + PartialEq + Serialize + DeserializeOwned {
    /// The list this is an entry of.
    const LIST: MedicalList;
    /// Identifies the entry: disabling a key turns off every bundled entry with it.
    fn key(&self) -> String;
    /// `Ok(None)` for a line that is not an entry (blank, directive, group
    /// the engine leaves out): such bundled lines are always kept.
    fn parse_line(line: &str) -> Result<Option<Self>, String>;
    fn to_line(&self) -> String;
    /// Checks an entry of the user before it is saved.
    fn validate(&self) -> Result<(), String>;
    /// The text a search looks in.
    fn search_text(&self) -> String;
    /// Stands for a disabled key the data no longer has, to show it.
    fn from_key(key: &str) -> Self;
}

/// A phrase of the phrase boosting list, without its fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Phrase(pub String);

impl OverlayEntry for Phrase {
    const LIST: MedicalList = MedicalList::Phrases;

    fn key(&self) -> String {
        self.0.clone()
    }

    fn parse_line(line: &str) -> Result<Option<Self>, String> {
        Ok(line_phrase(line).map(Phrase))
    }

    /// Added lines come after the list's defaults line, so they get the same tuning.
    fn to_line(&self) -> String {
        self.0.clone()
    }

    fn validate(&self) -> Result<(), String> {
        match Self::parse_line(&self.0) {
            Ok(Some(read)) if read == *self => Ok(()),
            _ => Err(format!(
                "\"{}\": a phrase cannot be empty, start with # or *, or end with : and a number",
                self.0
            )),
        }
    }

    fn search_text(&self) -> String {
        self.0.clone()
    }

    fn from_key(key: &str) -> Self {
        Phrase(key.to_string())
    }
}

/// Most candidates of a group of the user: each is aligned on the audio of
/// the heard word, so a huge group would slow every dictation down.
const MAX_CANDIDATES: usize = 20;

impl OverlayEntry for ConfusionGroup {
    const LIST: MedicalList = MedicalList::ConfusionGroups;

    fn key(&self) -> String {
        ConfusionGroup::key(self)
    }

    fn parse_line(line: &str) -> Result<Option<Self>, String> {
        ConfusionGroup::from_line(line)
    }

    fn to_line(&self) -> String {
        ConfusionGroup::to_line(self)
    }

    fn validate(&self) -> Result<(), String> {
        let words = std::iter::once(&self.heard).chain(&self.candidates);
        if words.clone().any(|w| has_unsafe_chars(w)) {
            return Err(format!(
                "\"{}\": a group cannot contain control characters",
                self.heard.escape_debug()
            ));
        }
        if self.candidates.len() > MAX_CANDIDATES {
            return Err(format!(
                "\"{}\": a group has at most {MAX_CANDIDATES} candidates",
                self.heard
            ));
        }
        let distinct: HashSet<&str> = self
            .candidates
            .iter()
            .map(|c| c.trim())
            .filter(|c| !c.is_empty())
            .collect();
        match (self.key().is_empty(), distinct.len()) {
            (true, _) => Err("The heard form is empty".to_string()),
            (false, n) if n < 2 => Err(format!(
                "\"{}\": a group needs at least 2 different candidates",
                self.heard
            )),
            _ => Ok(()),
        }
    }

    fn search_text(&self) -> String {
        format!("{} {}", self.heard, self.candidates.join(" "))
    }

    fn from_key(key: &str) -> Self {
        ConfusionGroup {
            heard: key.to_string(),
            candidates: Vec::new(),
        }
    }
}

/// The user's changes to one list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, bound(deserialize = "T: Deserialize<'de>"))]
pub struct ListOverlay<T> {
    /// Keys of the bundled entries turned off. Kept even when the data in
    /// use no longer has them.
    pub disabled: Vec<String>,
    /// The user's own entries, used after the bundled ones.
    pub added: Vec<T>,
}

impl<T> Default for ListOverlay<T> {
    fn default() -> Self {
        Self {
            disabled: Vec::new(),
            added: Vec::new(),
        }
    }
}

/// Where an entry shown in the settings comes from.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    Bundled,
    Added,
    /// A disabled key the data in use no longer has: only its key is known.
    Orphaned,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EntryView<T> {
    /// What disables it (see [`OverlayEntry::key`]).
    pub key: String,
    pub entry: T,
    pub origin: Origin,
    pub enabled: bool,
}

/// One page of a search, and how many entries match in all.
#[derive(Debug, Serialize)]
pub struct ListPage<T> {
    pub total: usize,
    pub entries: Vec<EntryView<T>>,
}

fn bundled_entries<T: OverlayEntry>(bundled: &str) -> impl Iterator<Item = T> + '_ {
    bundled
        .lines()
        .filter_map(|line| T::parse_line(line).ok().flatten())
}

impl<T: OverlayEntry> ListOverlay<T> {
    fn disabled_set(&self) -> HashSet<&str> {
        self.disabled.iter().map(String::as_str).collect()
    }

    /// The list the engine uses and an export writes: the bundled lines,
    /// minus the disabled entries, then the added entries that are not
    /// already in it (an entry added again by a newer data set is not
    /// doubled, but stays saved).
    pub fn apply(&self, bundled: &str) -> String {
        let disabled = self.disabled_set();
        let mut present: HashSet<String> = HashSet::new();
        let mut lines: Vec<String> = Vec::new();
        for line in bundled.lines() {
            // Lines that are not an entry (directives, left out groups) stay.
            if let Ok(Some(entry)) = T::parse_line(line) {
                let key = entry.key();
                if disabled.contains(key.as_str()) {
                    continue;
                }
                present.insert(key);
            }
            lines.push(line.to_string());
        }
        for entry in &self.added {
            if present.insert(entry.key()) {
                lines.push(entry.to_line());
            }
        }
        lines.join("\n")
    }

    /// Every entry with its state: the user's own first, then the disabled
    /// keys the data no longer has, then the bundled entries.
    pub fn views(&self, bundled: &str) -> Vec<EntryView<T>> {
        let disabled = self.disabled_set();
        let bundled: Vec<T> = bundled_entries(bundled).collect();
        let bundled_keys: HashSet<String> = bundled.iter().map(T::key).collect();
        let added = self.added.iter().map(|entry| EntryView {
            key: entry.key(),
            entry: entry.clone(),
            origin: Origin::Added,
            enabled: true,
        });
        let orphaned = self
            .disabled
            .iter()
            .filter(|key| !bundled_keys.contains(key.as_str()))
            .map(|key| EntryView {
                key: key.clone(),
                entry: T::from_key(key),
                origin: Origin::Orphaned,
                enabled: false,
            });
        let bundled = bundled.into_iter().map(|entry| {
            let key = entry.key();
            EntryView {
                enabled: !disabled.contains(key.as_str()),
                key,
                entry,
                origin: Origin::Bundled,
            }
        });
        added.chain(orphaned).chain(bundled).collect()
    }

    /// The changes that turn `bundled` into the `imported` file: bundled
    /// entries missing from it (or different in it) are disabled, the others
    /// of the file are added. Unreadable lines and invalid entries are all
    /// reported, by line number.
    pub fn diff(bundled: &str, imported: &str) -> Result<Self, Vec<String>> {
        let bundled: Vec<T> = bundled_entries(bundled).collect();
        let mut by_key: HashMap<String, Vec<&T>> = HashMap::new();
        for entry in &bundled {
            by_key.entry(entry.key()).or_default().push(entry);
        }
        let mut errors = Vec::new();
        let mut kept: HashSet<String> = HashSet::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut added = Vec::new();
        for (index, line) in imported.lines().enumerate() {
            let entry = match T::parse_line(line) {
                Ok(Some(entry)) => entry,
                Ok(None) => continue,
                Err(e) => {
                    errors.push(format!("line {}: {e}", index + 1));
                    continue;
                }
            };
            let key = entry.key();
            if !seen.insert(key.clone()) {
                continue;
            }
            let unchanged = by_key
                .get(&key)
                .is_some_and(|same_key| same_key.iter().any(|b| **b == entry));
            // Only new entries are checked: a bundled line stays as it is.
            match (unchanged, entry.validate()) {
                (true, _) => {
                    kept.insert(key);
                }
                (false, Ok(())) => added.push(entry),
                (false, Err(e)) => errors.push(format!("line {}: {e}", index + 1)),
            }
        }
        if errors.is_empty() && seen.is_empty() {
            errors.push("the file has no entry".to_string());
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        // `kept` also gets the disabled keys, so each is listed once.
        let mut disabled = Vec::new();
        for key in bundled.iter().map(T::key) {
            if kept.insert(key.clone()) {
                disabled.push(key);
            }
        }
        Ok(Self { disabled, added })
    }

    /// Checks the user's entries before saving.
    pub fn validate(&self) -> Result<(), String> {
        check_entry_count(self.added.len(), "added entries")?;
        self.added.iter().try_for_each(T::validate)
    }
}

/// The entries matching `query` (accents and case ignored), `limit` of them
/// from `offset`.
pub fn search<T: OverlayEntry>(
    views: Vec<EntryView<T>>,
    query: &str,
    offset: usize,
    limit: usize,
) -> ListPage<T> {
    let needle = super::drug_rules::fold(query.trim());
    let matching: Vec<EntryView<T>> = views
        .into_iter()
        .filter(|view| {
            needle.is_empty()
                || super::drug_rules::fold(&view.entry.search_text()).contains(&needle)
        })
        .collect();
    ListPage {
        total: matching.len(),
        entries: matching.into_iter().skip(offset).take(limit).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHRASES: &str =
        "#!commitment 0.5\n*:0.5:0.01:fhpn\nmicroangiopathie thrombotique\nrituximab\ncetuximab";

    fn phrases(list: &[&str]) -> Vec<Phrase> {
        list.iter().map(|p| Phrase(p.to_string())).collect()
    }

    fn group(heard: &str, candidates: &[&str]) -> ConfusionGroup {
        ConfusionGroup {
            heard: heard.to_string(),
            candidates: candidates.iter().map(|c| c.to_string()).collect(),
        }
    }

    #[test]
    fn disabling_drops_the_entry_and_keeps_the_directives() {
        let overlay: ListOverlay<Phrase> = ListOverlay {
            disabled: vec!["rituximab".to_string()],
            added: Vec::new(),
        };
        assert_eq!(
            overlay.apply(PHRASES),
            "#!commitment 0.5\n*:0.5:0.01:fhpn\nmicroangiopathie thrombotique\ncetuximab"
        );
    }

    #[test]
    fn added_entries_come_last_without_duplicates() {
        let overlay = ListOverlay {
            disabled: Vec::new(),
            added: phrases(&["venlafaxine", "rituximab", "venlafaxine"]),
        };
        assert!(overlay.apply(PHRASES).ends_with("cetuximab\nvenlafaxine"));
    }

    #[test]
    fn changes_the_data_no_longer_has_are_kept_and_shown() {
        let overlay: ListOverlay<Phrase> = ListOverlay {
            disabled: vec!["ancienne expression".to_string()],
            added: phrases(&["cetuximab"]),
        };
        // A newer data set without "ancienne expression", and already with
        // the added "cetuximab": the list is unchanged, the overlay intact.
        assert_eq!(overlay.apply(PHRASES), PHRASES);
        let views = overlay.views(PHRASES);
        assert_eq!(views[0].origin, Origin::Added);
        assert_eq!(
            views[1],
            EntryView {
                key: "ancienne expression".to_string(),
                entry: Phrase("ancienne expression".to_string()),
                origin: Origin::Orphaned,
                enabled: false
            }
        );
        assert_eq!(views.len(), 2 + 3);
    }

    #[test]
    fn importing_an_export_changes_nothing() {
        let overlay = ListOverlay {
            disabled: vec!["rituximab".to_string()],
            added: phrases(&["venlafaxine"]),
        };
        let exported = overlay.apply(PHRASES);
        assert_eq!(ListOverlay::<Phrase>::diff(PHRASES, &exported), Ok(overlay));
    }

    #[test]
    fn importing_an_empty_or_invalid_file_is_refused() {
        assert!(ListOverlay::<Phrase>::diff(PHRASES, "\n#!commitment 0\n").is_err());
        let errors = ListOverlay::<ConfusionGroup>::diff(
            "",
            "not json\n{\"heard\": \"a\", \"candidates\": [{\"text\": \"b\"}]}",
        )
        .unwrap_err();
        assert_eq!(errors.len(), 2);
        assert!(errors[0].starts_with("line 1"));
        assert!(errors[1].starts_with("line 2"));
    }

    #[test]
    fn a_changed_group_disables_the_bundled_one_and_adds_the_new_one() {
        let bundled = format!(
            "{}\n{}",
            r#"{"heard": "singulaire", "candidates": [{"text": "Singulair", "count": 11}, {"text": "cingulaire", "count": 11}], "heard_correct_count": 3}"#,
            r#"{"heard": "abdominale", "candidates": [{"text": "abdominales", "count": 630}, {"text": "abdominal", "count": 9}], "heard_correct_count": 8194}"#,
        );
        let changed = group("singulaire", &["Singulair", "cingulaire", "singulier"]);
        let overlay = ListOverlay::<ConfusionGroup>::diff(&bundled, &changed.to_line()).unwrap();
        assert_eq!(overlay.disabled, vec!["singulaire".to_string()]);
        assert_eq!(overlay.added, vec![changed.clone()]);
        // The group left out by the engine stays in the list, untouched.
        let applied = overlay.apply(&bundled);
        assert!(applied.contains("abdominale"));
        assert!(applied.ends_with(&changed.to_line()));
        assert_eq!(overlay.views(&bundled).len(), 2);
    }

    #[test]
    fn the_bundled_lists_round_trip_unchanged() {
        use crate::medical::data_set::{BUNDLED_CONFUSION_GROUPS, BUNDLED_PHRASE_LIST};
        let phrases: ListOverlay<Phrase> = ListOverlay::default();
        assert_eq!(
            phrases.apply(BUNDLED_PHRASE_LIST),
            BUNDLED_PHRASE_LIST.trim_end()
        );
        assert_eq!(
            ListOverlay::<Phrase>::diff(BUNDLED_PHRASE_LIST, BUNDLED_PHRASE_LIST),
            Ok(phrases)
        );
        let groups: ListOverlay<ConfusionGroup> = ListOverlay::default();
        assert_eq!(
            ListOverlay::<ConfusionGroup>::diff(BUNDLED_CONFUSION_GROUPS, BUNDLED_CONFUSION_GROUPS),
            Ok(groups)
        );
    }

    #[test]
    fn user_groups_need_two_candidates() {
        assert!(group("ituximab", &["rituximab", "cetuximab"])
            .validate()
            .is_ok());
        assert!(group("ituximab", &["rituximab", "rituximab "])
            .validate()
            .is_err());
        assert!(group(" ", &["a", "b"]).validate().is_err());
        assert!(Phrase("ratio 3:1".to_string()).validate().is_err());
        assert!(Phrase("veine lafaxine".to_string()).validate().is_ok());
    }

    #[test]
    fn user_groups_have_at_most_20_candidates_and_no_control_character() {
        let names: Vec<String> = (0..21).map(|i| format!("candidat{i}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        assert!(group("heard", &names[..20]).validate().is_ok());
        assert!(group("heard", &names).validate().is_err());
        assert!(group("ituximab", &["rituximab\u{8}\u{8}", "cetuximab"])
            .validate()
            .is_err());
        assert!(group("itux\u{1b}", &["rituximab", "cetuximab"])
            .validate()
            .is_err());
    }

    #[test]
    fn an_overlay_with_too_many_added_entries_is_refused() {
        let max = crate::utils::user_text::MAX_USER_ENTRIES;
        let mut overlay = ListOverlay {
            disabled: Vec::new(),
            added: vec![Phrase("rituximab".to_string()); max],
        };
        assert!(overlay.validate().is_ok());
        overlay.added.push(Phrase("cetuximab".to_string()));
        assert!(overlay.validate().is_err());
    }

    #[test]
    fn search_ignores_accents_and_case_and_pages() {
        let overlay: ListOverlay<Phrase> = ListOverlay::default();
        let page = search(
            overlay.views("Thrombotique\nthrombose\nrituximab"),
            "THROMBÔ",
            1,
            10,
        );
        assert_eq!(page.total, 2);
        assert_eq!(page.entries[0].entry, Phrase("thrombose".to_string()));
    }
}
