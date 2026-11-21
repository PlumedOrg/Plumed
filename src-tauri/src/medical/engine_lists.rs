//! French medical data armed on the engine once, when the model loads, from
//! the active data set (bundled or downloaded, see `data_set`): the phrase
//! boosting list and the confusion groups of the decoder's second pass.
//! Building them takes a moment, unlike the per-transcription dictionary
//! sync. Each one has a built-in option that turns it off. Written with
//! Claude Code.

use super::data_set::{self, ActiveData};
use crate::audio::types::AudioState;
use crate::engine::ParakeetEngine;
use crate::formatting_rules::BuiltInOptions;
use tauri::{AppHandle, Manager};

/// One list of the data set, the option that enables it and how to arm it.
struct EngineList {
    data: fn(&ActiveData) -> &str,
    enabled: fn(&BuiltInOptions) -> bool,
    arm: fn(&mut ParakeetEngine, &str),
}

const LISTS: [EngineList; 2] = [
    EngineList {
        data: |data| &data.phrase_list,
        enabled: |options| options.phrase_boosting,
        arm: ParakeetEngine::set_boost_list,
    },
    EngineList {
        data: |data| &data.confusion_groups,
        enabled: |options| options.confusion_rescoring,
        arm: ParakeetEngine::set_confusion_groups,
    },
];

impl EngineList {
    /// The content to arm: the list, or nothing to turn it off.
    fn content<'a>(&self, data: &'a ActiveData, options: &BuiltInOptions) -> &'a str {
        match (self.enabled)(options) {
            true => (self.data)(data),
            false => "",
        }
    }
}

/// Arms every enabled list on a freshly loaded `engine` (defaults when the
/// settings are unreadable).
pub fn arm(app: &AppHandle, engine: &mut ParakeetEngine) {
    let options = crate::formatting_rules::load(app)
        .map(|settings| settings.built_in)
        .unwrap_or_default();
    let data = data_set::current();
    for list in &LISTS {
        (list.arm)(engine, list.content(&data, &options));
    }
}

/// Re-arms every list on the already loaded engine, if any, after the data
/// set changed (a not yet loaded engine gets them in [`arm`]).
pub fn rearm(app: &AppHandle) {
    let state = app.state::<AudioState>();
    let mut engine = state.engine.lock();
    if let Some(engine) = engine.as_mut() {
        arm(app, engine);
    }
}

/// Re-arms, on the already loaded engine if any, the lists whose option
/// changed (a not yet loaded engine reads the options in [`arm`]).
pub fn apply_settings(app: &AppHandle, before: Option<&BuiltInOptions>, after: &BuiltInOptions) {
    let state = app.state::<AudioState>();
    let mut engine = state.engine.lock();
    let Some(engine) = engine.as_mut() else {
        return;
    };
    let data = data_set::current();
    for list in &LISTS {
        if before.map(list.enabled) != Some((list.enabled)(after)) {
            (list.arm)(engine, list.content(&data, after));
        }
    }
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
            assert_eq!(
                list.content(&data, &BuiltInOptions::default()),
                (list.data)(&data)
            );
            assert_eq!(list.content(&data, &off), "");
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
        assert_eq!(list.commitment, Some(0.0));
        assert!(
            list.phrases.len() > 70_000,
            "{} phrases",
            list.phrases.len()
        );
        let first = &list.phrases[0];
        assert_eq!((first.weight, first.minp), (1.5, 0.01));
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
