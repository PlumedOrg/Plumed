//! The grammar step's procedure, line by line: the spell checker's unknown words of the
//! original line, the masks, Grammalecte on the masked line, its errors mapped back, and
//! only the allowed rules' fixes with exactly one suggestion applied to the original line,
//! right to left. Written with Claude Code.

use super::mask::{self, Mask};
use super::rules::RuleSet;
use std::collections::HashSet;

/// Lines longer than this are left as they are (Grammalecte itself skips sentences of
/// 3,000 characters or more, and the masks backtrack).
pub const MAX_LINE_CHARS: usize = 5_000;

/// A grammar error, offsets in code points of the checked text.
#[derive(Debug, Clone, PartialEq)]
pub struct GcError {
    pub start: usize,
    pub end: usize,
    pub rule_id: String,
    /// Grammalecte's option group (`sType`), e.g. "gn", "ppas", "typo".
    pub kind: String,
    pub suggestions: Vec<String>,
}

/// What the procedure needs from Grammalecte (a fake in tests).
pub trait Checker {
    /// Spans of the words the spell checker does not know, in code points.
    fn unknown_words(&mut self, text: &str) -> Result<Vec<(usize, usize)>, String>;
    /// Grammar errors (spelling excluded) of a paragraph.
    fn grammar_errors(&mut self, text: &str) -> Result<Vec<GcError>, String>;
}

/// What the procedure applies: the rules, the masks and the error types never applied.
pub struct Config<'a> {
    pub rules: &'a RuleSet,
    pub masks: &'a [Mask],
    pub skip_types: &'a HashSet<String>,
}

/// One line (no newline in it) corrected.
fn correct_line(line: &str, checker: &mut dyn Checker, config: &Config) -> Result<String, String> {
    let unknown = checker.unknown_words(line)?;
    let masked = mask::mask(line, config.masks, &unknown)?;
    let mut fixes: Vec<(usize, usize, String)> = checker
        .grammar_errors(&masked.text)?
        .into_iter()
        .filter(|e| !config.skip_types.contains(&e.kind))
        .filter(|e| e.suggestions.len() == 1 && config.rules.allows(&e.rule_id))
        .filter_map(|mut e| {
            let (start, end) = masked.map_back(e.start, e.end)?;
            e.suggestions.pop().map(|s| (start, end, s))
        })
        .collect();
    if fixes.is_empty() {
        return Ok(line.to_string());
    }
    // Right to left by start; a stable sort keeps Grammalecte's order for equal starts.
    fixes.sort_by_key(|&(start, _, _)| std::cmp::Reverse(start));
    let mut chars: Vec<char> = line.chars().collect();
    for (start, end, suggestion) in fixes {
        // Same slicing as Python's out[:s] + sugg + out[e:], also when fixes overlap.
        let start = start.min(chars.len());
        let end = end.clamp(start, chars.len());
        chars.splice(start..end, suggestion.chars());
    }
    Ok(chars.into_iter().collect())
}

/// `text` with the allowed fixes applied, each line on its own (a line is a paragraph for
/// Grammalecte). Blank and overlong lines are kept as they are.
pub fn correct(text: &str, checker: &mut dyn Checker, config: &Config) -> Result<String, String> {
    if config.rules.is_empty() {
        return Ok(text.to_string());
    }
    let lines: Result<Vec<String>, String> = text
        .split('\n')
        .map(
            |line| match line.trim().is_empty() || line.chars().count() > MAX_LINE_CHARS {
                true => Ok(line.to_string()),
                false => correct_line(line, checker, config),
            },
        )
        .collect();
    Ok(lines?.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::super::rules::DATA;
    use super::*;

    /// Answers with fixed errors for a given masked text.
    struct Fake {
        unknown: Vec<(usize, usize)>,
        expected_masked: &'static str,
        errors: Vec<GcError>,
    }

    impl Checker for Fake {
        fn unknown_words(&mut self, _: &str) -> Result<Vec<(usize, usize)>, String> {
            Ok(self.unknown.clone())
        }
        fn grammar_errors(&mut self, text: &str) -> Result<Vec<GcError>, String> {
            assert_eq!(text, self.expected_masked);
            Ok(self.errors.clone())
        }
    }

    fn error(start: usize, end: usize, rule_id: &str, kind: &str, sugg: &[&str]) -> GcError {
        GcError {
            start,
            end,
            rule_id: rule_id.to_string(),
            kind: kind.to_string(),
            suggestions: sugg.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn run(text: &str, fake: &mut Fake, rules: &RuleSet) -> String {
        let masks = mask::compile(&DATA.masks).unwrap();
        let skip: HashSet<String> = DATA.skip_types.iter().cloned().collect();
        let config = Config {
            rules,
            masks: &masks,
            skip_types: &skip,
        };
        correct(text, fake, &config).unwrap()
    }

    fn defaults() -> RuleSet {
        RuleSet::from_overlay(&DATA.rules, &Default::default())
    }

    #[test]
    fn a_fix_on_the_masked_text_lands_on_the_original() {
        let mut fake = Fake {
            unknown: Vec::new(),
            expected_masked: "La patiente est fatigué.",
            errors: vec![error(
                16,
                23,
                "gv1__ppas_sujet_être_accord_singulier__b2_a1_1",
                "ppas",
                &["fatiguée"],
            )],
        };
        assert_eq!(
            run("La patiente de 63 ans est fatigué.", &mut fake, &defaults()),
            "La patiente de 63 ans est fatiguée."
        );
    }

    #[test]
    fn only_allowed_single_suggestion_grammar_fixes_apply() {
        let text = "Les examens biologique sont rassurant.";
        let mut fake = Fake {
            unknown: Vec::new(),
            expected_masked: text,
            errors: vec![
                // Not allowlisted.
                error(12, 22, "g3__gn_les_2m__b1_a4_1", "gn", &["biologiques"]),
                // Allowlisted family but two suggestions.
                error(
                    28,
                    37,
                    "gv1__ppas_sujet_être_accord_pluriel__b1_a3_1",
                    "ppas",
                    &["rassurants", "rassurantes"],
                ),
                // Typography never applies.
                error(0, 3, "g2__locutions_invariables__x", "typo", &["LES"]),
            ],
        };
        assert_eq!(run(text, &mut fake, &defaults()), text);
    }

    #[test]
    fn fixes_apply_right_to_left_on_each_line() {
        let mut fake = Fake {
            unknown: Vec::new(),
            expected_masked: "ces examen",
            errors: vec![error(
                4,
                10,
                "g3__gn_ces_aux_pluriel_1m__b1_a3_1",
                "gn",
                &["examens"],
            )],
        };
        assert_eq!(
            run("ces examen\n\nces examen", &mut fake, &defaults()),
            "ces examens\n\nces examens"
        );
    }

    #[test]
    fn no_rule_means_no_check() {
        struct Never;
        impl Checker for Never {
            fn unknown_words(&mut self, _: &str) -> Result<Vec<(usize, usize)>, String> {
                Err("called".into())
            }
            fn grammar_errors(&mut self, _: &str) -> Result<Vec<GcError>, String> {
                Err("called".into())
            }
        }
        let masks = mask::compile(&DATA.masks).unwrap();
        let skip = HashSet::new();
        let rules = RuleSet::default();
        let config = Config {
            rules: &rules,
            masks: &masks,
            skip_types: &skip,
        };
        assert_eq!(
            correct("ces examen", &mut Never, &config).unwrap(),
            "ces examen"
        );
    }
}
