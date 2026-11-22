//! French medical data armed on the engine once, when the model loads, from
//! the active data set (bundled or downloaded, see `data_set`) with the
//! user's changes on top (see `list_overlay`): the phrase boosting list and
//! the confusion groups of the decoder's second pass. Building them takes a
//! moment, unlike the per-transcription dictionary sync. Each one has a
//! built-in option that turns it off. Written with Claude Code.

use super::data_set::{self, ActiveData};
use super::store::{self, StoreFile};
use crate::audio::types::AudioState;
use crate::engine::ParakeetEngine;
use crate::formatting_rules::BuiltInOptions;
use serde::Deserialize;
use tauri::{AppHandle, Manager};

/// A list of the data set the engine uses.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MedicalList {
    Phrases,
    ConfusionGroups,
}

impl MedicalList {
    /// The list as the data set has it, before the user's changes.
    pub fn bundled(self, data: &ActiveData) -> &str {
        match self {
            MedicalList::Phrases => &data.phrase_list,
            MedicalList::ConfusionGroups => &data.confusion_groups,
        }
    }

    /// Where the user's changes to it are saved.
    pub(super) fn store_file(self) -> &'static StoreFile {
        match self {
            MedicalList::Phrases => &store::PHRASE_OVERLAY,
            MedicalList::ConfusionGroups => &store::CONFUSION_OVERLAY,
        }
    }
}

/// One list, the option that enables it and how to arm it.
struct EngineList {
    list: MedicalList,
    enabled: fn(&BuiltInOptions) -> bool,
    arm: fn(&mut ParakeetEngine, &str),
}

const LISTS: [EngineList; 2] = [
    EngineList {
        list: MedicalList::Phrases,
        enabled: |options| options.phrase_boosting,
        arm: ParakeetEngine::set_boost_list,
    },
    EngineList {
        list: MedicalList::ConfusionGroups,
        enabled: |options| options.confusion_rescoring,
        arm: ParakeetEngine::set_confusion_groups,
    },
];

impl EngineList {
    /// What to arm: the list, or nothing to turn it off.
    fn content<'a>(&self, list: &'a str, options: &BuiltInOptions) -> &'a str {
        match (self.enabled)(options) {
            true => list,
            false => "",
        }
    }

    fn arm_on(
        &self,
        app: &AppHandle,
        engine: &mut ParakeetEngine,
        data: &ActiveData,
        options: &BuiltInOptions,
    ) {
        let list = match (self.enabled)(options) {
            true => super::overlaid_list(app, self.list, data),
            false => String::new(),
        };
        (self.arm)(engine, self.content(&list, options));
    }
}

fn options(app: &AppHandle) -> BuiltInOptions {
    crate::formatting_rules::load(app)
        .map(|settings| settings.built_in)
        .unwrap_or_default()
}

/// Arms every enabled list on a freshly loaded `engine` (defaults when the
/// settings are unreadable).
pub fn arm(app: &AppHandle, engine: &mut ParakeetEngine) {
    let options = options(app);
    let data = data_set::current();
    for list in &LISTS {
        list.arm_on(app, engine, &data, &options);
    }
}

/// Runs `f` on the already loaded engine, if any (a not yet loaded engine
/// gets everything in [`arm`]).
fn with_loaded_engine(app: &AppHandle, f: impl FnOnce(&mut ParakeetEngine)) {
    let state = app.state::<AudioState>();
    let mut engine = state.engine.lock();
    if let Some(engine) = engine.as_mut() {
        f(engine);
    }
}

/// Re-arms every list after the data set changed.
pub fn rearm(app: &AppHandle) {
    with_loaded_engine(app, |engine| arm(app, engine));
}

/// Re-arms one list after the user changed it.
pub fn rearm_list(app: &AppHandle, list: MedicalList) {
    let options = options(app);
    let data = data_set::current();
    with_loaded_engine(app, |engine| {
        for engine_list in LISTS.iter().filter(|l| l.list == list) {
            engine_list.arm_on(app, engine, &data, &options);
        }
    });
}

/// Re-arms the lists whose option changed.
pub fn apply_settings(app: &AppHandle, before: Option<&BuiltInOptions>, after: &BuiltInOptions) {
    let data = data_set::current();
    with_loaded_engine(app, |engine| {
        for list in &LISTS {
            if before.map(list.enabled) != Some((list.enabled)(after)) {
                list.arm_on(app, engine, &data, after);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::confusion::ConfusionGroups;
    use crate::engine::phrase_boost::list::parse_boost_list;

    #[test]
    fn turning_an_option_off_arms_nothing() {
        let off = BuiltInOptions {
            phrase_boosting: false,
            confusion_rescoring: false,
            ..BuiltInOptions::default()
        };
        let data = data_set::current();
        for list in &LISTS {
            let bundled = list.list.bundled(&data);
            assert_eq!(list.content(bundled, &BuiltInOptions::default()), bundled);
            assert_eq!(list.content(bundled, &off), "");
        }
    }

    #[test]
    fn bundled_list_parses_with_the_benchmarked_tuning() {
        let list = parse_boost_list(data_set::BUNDLED_PHRASE_LIST);
        assert!(
            list.warnings.is_empty(),
            "{:?}",
            &list.warnings[..3.min(list.warnings.len())]
        );
        assert_eq!(list.commitment, Some(0.5));
        assert!(
            list.phrases.len() > 70_000,
            "{} phrases",
            list.phrases.len()
        );
        let first = &list.phrases[0];
        assert_eq!((first.weight, first.minp), (0.5, 0.01));
    }

    #[test]
    fn bundled_list_has_no_field_swallowed_into_a_phrase() {
        // A line with one field too many ("inoep:-1:::fph") parses as the
        // phrase "inoep:-1" with the default weight: its penalty is lost.
        let list = parse_boost_list(data_set::BUNDLED_PHRASE_LIST);
        let broken: Vec<&str> = list
            .phrases
            .iter()
            .map(|p| p.phrase.as_str())
            .filter(|p| p.contains(':'))
            .collect();
        assert!(broken.is_empty(), "{broken:?}");
        let penalized = |name: &str| {
            list.phrases
                .iter()
                .any(|p| p.phrase == name && p.weight < 0.0)
        };
        assert!(penalized("inoep") && penalized("myrtazapine"));
    }

    #[test]
    fn bundled_confusion_groups_keep_the_ambiguous_ones() {
        let groups = ConfusionGroups::parse(data_set::BUNDLED_CONFUSION_GROUPS);
        assert!(
            (100..300).contains(&groups.len()),
            "{} groups",
            groups.len()
        );
    }
}
