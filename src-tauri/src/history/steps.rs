//! Plumed fork (written with Claude Code): the text after each processing
//! step of a dictation, kept with its history entry so the home page can show
//! what each step changed. Follows the history rules: in memory, on disk only
//! when history persistence is on, five entries at most.

use serde::{Deserialize, Serialize};

/// The text steps after recognition, in the order the code runs them. Mirrors
/// the frontend `TextStepId` (`src/features/corrections/corrections.helpers.ts`),
/// where `ShortText` and `BuiltInFormatting` are options shown under the
/// clean-up step of the Explanation flow, not steps of their own.
/// Recognition covers the medical vocabulary and the sound-alike words, which
/// act while decoding and cannot be told apart in the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStep {
    Recognition,
    Dictionary,
    CleanUp,
    ModelCorrections,
    ShortText,
    FormattingRules,
    BuiltInFormatting,
    Grammar,
    Templates,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StepText {
    pub step_id: PipelineStep,
    pub text_after: String,
}

/// Collects the text after each step. `off()` records nothing (previews, CLI),
/// so callers that do not keep the steps pay no copy.
#[derive(Debug, Default)]
pub struct StepTrace(Option<Vec<StepText>>);

impl StepTrace {
    pub fn on() -> Self {
        Self(Some(Vec::new()))
    }

    pub fn off() -> Self {
        Self(None)
    }

    pub fn record(&mut self, step_id: PipelineStep, text: &str) {
        if let Some(steps) = self.0.as_mut() {
            steps.push(StepText {
                step_id,
                text_after: text.to_string(),
            });
        }
    }

    pub fn into_steps(self) -> Vec<StepText> {
        self.0.unwrap_or_default()
    }
}

/// Joins the steps of consecutive parts of one dictation (the PhoneMic
/// recording is transcribed in chunks), text by text with a space, like the
/// final texts are. Parts recording different steps keep the first's steps.
pub fn join_parts(parts: Vec<Vec<StepText>>) -> Vec<StepText> {
    let mut parts = parts.into_iter();
    let Some(mut joined) = parts.next() else {
        return Vec::new();
    };
    for part in parts {
        let same_steps = part.len() == joined.len()
            && part
                .iter()
                .zip(&joined)
                .all(|(a, b)| a.step_id == b.step_id);
        if !same_steps {
            continue;
        }
        for (step, next) in joined.iter_mut().zip(part) {
            step.text_after.push(' ');
            step.text_after.push_str(&next.text_after);
        }
    }
    joined
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(step_id: PipelineStep, text: &str) -> StepText {
        StepText {
            step_id,
            text_after: text.to_string(),
        }
    }

    #[test]
    fn off_trace_records_nothing() {
        let mut trace = StepTrace::off();
        trace.record(PipelineStep::Recognition, "bonjour");
        assert!(trace.into_steps().is_empty());
    }

    #[test]
    fn on_trace_keeps_order() {
        let mut trace = StepTrace::on();
        trace.record(PipelineStep::Recognition, "le le patient");
        trace.record(PipelineStep::CleanUp, "le patient");
        assert_eq!(
            trace.into_steps(),
            vec![
                step(PipelineStep::Recognition, "le le patient"),
                step(PipelineStep::CleanUp, "le patient"),
            ]
        );
    }

    #[test]
    fn join_parts_joins_each_step_with_a_space() {
        let joined = join_parts(vec![
            vec![
                step(PipelineStep::Recognition, "veine lafaxine"),
                step(PipelineStep::ModelCorrections, "venlafaxine"),
            ],
            vec![
                step(PipelineStep::Recognition, "deux fois"),
                step(PipelineStep::ModelCorrections, "deux fois"),
            ],
        ]);
        assert_eq!(
            joined,
            vec![
                step(PipelineStep::Recognition, "veine lafaxine deux fois"),
                step(PipelineStep::ModelCorrections, "venlafaxine deux fois"),
            ]
        );
    }

    #[test]
    fn join_parts_of_nothing_is_empty() {
        assert!(join_parts(Vec::new()).is_empty());
    }

    #[test]
    fn steps_serialize_in_snake_case() {
        let json = serde_json::to_string(&step(PipelineStep::BuiltInFormatting, "x")).unwrap();
        assert_eq!(
            json,
            r#"{"step_id":"built_in_formatting","text_after":"x"}"#
        );
    }

    #[test]
    fn templates_step_has_the_frontend_id() {
        let json = serde_json::to_string(&step(PipelineStep::Templates, "x")).unwrap();
        assert_eq!(json, r#"{"step_id":"templates","text_after":"x"}"#);
    }
}
