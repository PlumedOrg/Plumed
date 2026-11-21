//! Bundled French medical data armed on the engine once, when the model loads
//! (see `medical-data/SOURCE.md`): the phrase boosting list and the confusion
//! groups of the decoder's second pass. Building them takes a moment, unlike
//! the per-transcription dictionary sync. Each one has a built-in option
//! that turns it off. Written with Claude Code.

use crate::audio::types::AudioState;
use crate::engine::ParakeetEngine;
use crate::formatting_rules::BuiltInOptions;
use tauri::{AppHandle, Manager};

const PHRASE_LIST: &str = include_str!("../../medical-data/phrase-boosting/french_medical.txt");
const CONFUSION_GROUPS: &str =
    include_str!("../../medical-data/regex-fixes/confusion_groups.jsonl");

/// One bundled data set, the option that enables it and how to arm it.
struct EngineList {
    bundled: &'static str,
    enabled: fn(&BuiltInOptions) -> bool,
    arm: fn(&mut ParakeetEngine, &str),
}

const LISTS: [EngineList; 2] = [
    EngineList {
        bundled: PHRASE_LIST,
        enabled: |options| options.phrase_boosting,
        arm: ParakeetEngine::set_boost_list,
    },
    EngineList {
        bundled: CONFUSION_GROUPS,
        enabled: |options| options.confusion_rescoring,
        arm: ParakeetEngine::set_confusion_groups,
    },
];

impl EngineList {
    /// The content to arm: the bundled data, or nothing to turn it off.
    fn content(&self, options: &BuiltInOptions) -> &'static str {
        match (self.enabled)(options) {
            true => self.bundled,
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
    for list in &LISTS {
        (list.arm)(engine, list.content(&options));
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
    for list in &LISTS {
        if before.map(list.enabled) != Some((list.enabled)(after)) {
            (list.arm)(engine, list.content(after));
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
        for list in &LISTS {
            assert_eq!(list.content(&BuiltInOptions::default()), list.bundled);
            assert_eq!(list.content(&off), "");
        }
    }

    #[test]
    fn bundled_list_parses_with_the_benchmarked_tuning() {
        let list = parse_boost_list(PHRASE_LIST);
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
        let groups = ConfusionGroups::parse(CONFUSION_GROUPS);
        assert!(
            (100..300).contains(&groups.len()),
            "{} groups",
            groups.len()
        );
    }
}
