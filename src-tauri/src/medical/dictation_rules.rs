//! UltiMed's French medical dictation rules (units, vital signs, dictated
//! punctuation and layout), seeded as the user's formatting rules on a fresh
//! install. Unlike the drug fixes they are ordinary user rules from then on:
//! the user edits, disables or deletes them in the formatting rules UI.
//!
//! The bundled file is `rules/dictation_rules.csv` of the rules repo, copied
//! as is by `medical-data/sync-regex-fixes.sh` (see `medical-data/SOURCE.md`),
//! and read here. Written with Claude Code.
//!
//! CSV format: columns `id,remplacement,regex,example_pre,example_post`,
//! blank lines and `#` comment lines ignored, `\n` in a replacement or an
//! example is a newline. Two kinds of rows are skipped because Plumed's
//! formatter (the `regex` crate) cannot run them: lookarounds, and a
//! replacement ending in `\u` (uppercase the next letter).

use crate::formatting_rules::types::{FormattingRule, MatchMode};
use log::warn;

const BUNDLED: &str = include_str!("../../medical-data/regex-fixes/dictation_rules.csv");

/// Hashed into every rule id, with the rule's examples. The value comes from
/// the murmure-regex converter these rules used to go through: keeping it
/// keeps the ids, and so the user's edits and the telemetry baseline, stable.
const ID_SOURCE_NAME: &str = "regex.csv";

/// Ends a replacement that uppercases the letter after the match.
const UPPERCASE_MARKER: &str = "\\u";

#[derive(Debug, PartialEq)]
struct DictationRule {
    id: String,
    trigger: String,
    replacement: String,
    example_pre: String,
    example_post: String,
}

/// Splits CSV `text` into records of fields (RFC 4180: a field in double
/// quotes may hold commas, newlines and `""` for a quote). Example:
/// `a,"b,""c""\nd"` gives `[["a", "b,\"c\"\nd"]]`.
fn parse_records(text: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (in_quotes, c) {
            (true, '"') if chars.peek() == Some(&'"') => {
                chars.next();
                field.push('"');
            }
            (true, '"') => in_quotes = false,
            (true, _) => field.push(c),
            (false, '"') => in_quotes = true,
            (false, ',') => record.push(std::mem::take(&mut field)),
            (false, '\r') => {}
            (false, '\n') => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            (false, _) => field.push(c),
        }
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    records
}

/// Same test as the converter: `(?=`, `(?!`, `(?<=` or `(?<!`.
fn has_lookaround(trigger: &str) -> bool {
    ["(?=", "(?!", "(?<=", "(?<!"]
        .iter()
        .any(|look| trigger.contains(look))
}

/// The rules of the CSV `text`, skipped rows left out.
fn parse(text: &str) -> Vec<DictationRule> {
    // Blank and comment lines go before the CSV parsing, as in the rules
    // repo's own test, so they are dropped even inside a quoted field.
    let kept: String = text
        .split_inclusive('\n')
        .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
        .collect();
    let mut records = parse_records(&kept).into_iter();
    let Some(header) = records.next() else {
        return Vec::new();
    };
    let column = |name: &str| header.iter().position(|h| h == name);
    let (Some(replacement_at), Some(trigger_at), Some(pre_at), Some(post_at)) = (
        column("remplacement"),
        column("regex"),
        column("example_pre"),
        column("example_post"),
    ) else {
        warn!("Bundled dictation rules: unexpected CSV header {header:?}");
        return Vec::new();
    };
    let newlines = |s: &str| s.replace("\\n", "\n");
    records
        .filter_map(|record| {
            let field = |at: usize| record.get(at).map(String::as_str).unwrap_or_default();
            let (trigger, replacement) = (field(trigger_at), field(replacement_at));
            // A replacement of spaces only is a real rule (an indent).
            if trigger.trim().is_empty() || replacement.is_empty() {
                return None;
            }
            if has_lookaround(trigger) || replacement.ends_with(UPPERCASE_MARKER) {
                return None;
            }
            let (example_pre, example_post) = (newlines(field(pre_at)), newlines(field(post_at)));
            let name = format!("{ID_SOURCE_NAME}\0{example_pre}\0{example_post}");
            Some(DictationRule {
                id: uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, name.as_bytes()).to_string(),
                trigger: trigger.to_string(),
                replacement: newlines(replacement),
                example_pre,
                example_post,
            })
        })
        .collect()
}

fn bundled() -> Vec<DictationRule> {
    parse(BUNDLED)
}

/// The formatting rules of a fresh install.
pub fn default_rules() -> Vec<FormattingRule> {
    bundled()
        .into_iter()
        .map(|rule| FormattingRule {
            id: rule.id,
            trigger: rule.trigger,
            replacement: rule.replacement,
            enabled: true,
            match_mode: MatchMode::Regex,
            name: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formatting_rules::types::BuiltInOptions;
    use crate::formatting_rules::{apply_formatting, FormattingSettings};

    /// Only the given user rules, every built-in option off.
    fn rules_only(rules: Vec<FormattingRule>) -> FormattingSettings {
        FormattingSettings {
            built_in: BuiltInOptions {
                short_text_correction: 0,
                trailing_space: false,
                medical_fixes: false,
                space_before_punctuation: false,
                convert_text_numbers: false,
                ..BuiltInOptions::default()
            },
            rules,
        }
    }

    #[test]
    fn fresh_install_gets_the_dictation_rules() {
        let rules = FormattingSettings::default().rules;
        assert_eq!(rules.len(), bundled().len());
        assert!(rules.len() > 40);
        assert!(rules
            .iter()
            .all(|r| r.enabled && r.match_mode == MatchMode::Regex));
    }

    /// The ids must not change: user edits and the telemetry baseline refer
    /// to them. These two were computed by the murmure-regex converter.
    #[test]
    fn rule_ids_stay_those_of_the_converter() {
        let rules = bundled();
        let id_of = |pre: &str| {
            rules
                .iter()
                .find(|r| r.example_pre == pre)
                .map(|r| r.id.as_str())
        };
        assert_eq!(
            id_of("15 mg par kilo par jour"),
            Some("82534e5d-07f6-5171-a4ed-994ed28527f7")
        );
        // `\n` in the CSV example: the id hashes the real newline.
        let newline = rules
            .iter()
            .find(|r| r.example_pre == " à la ligne ")
            .unwrap();
        assert_eq!(newline.id, "045c3d1c-f058-52f1-8e67-9eb07423b36a");
        assert_eq!(newline.replacement, "\n");
    }

    #[test]
    fn unsupported_and_empty_rows_are_skipped() {
        let csv = "id,remplacement,regex,example_pre,example_post\n\
                   # === section ===\n\
                   \n\
                   a,\"    $1\\u\",\"(?i)alinea\",x,y\n\
                   b,X,\"(?<=a)b\",ab,aX\n\
                   c,,word,w,w\n\
                   d,\" \",\"\\s+\",\"a  b\",\"a b\"\n\
                   e,\"x, \"\"y\"\"\\nz\",q,q,\"x, \"\"y\"\"\\nz\"\n";
        let rules = parse(csv);
        let triggers: Vec<&str> = rules.iter().map(|r| r.trigger.as_str()).collect();
        assert_eq!(triggers, ["\\s+", "q"]);
        assert_eq!(rules[1].replacement, "x, \"y\"\nz");
    }

    /// Every example whose output differs from the expected text, when run
    /// through the settings that `settings_for` builds from the example's rule.
    fn mismatches(settings_for: impl Fn(FormattingRule) -> FormattingSettings) -> String {
        default_rules()
            .into_iter()
            .zip(bundled())
            .filter_map(|(rule, source)| {
                let got = apply_formatting(source.example_pre.clone(), &settings_for(rule));
                (got != source.example_post).then(|| {
                    format!(
                        "{}: {:?} -> {got:?}, expected {:?}",
                        source.id, source.example_pre, source.example_post
                    )
                })
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Each rule alone turns its example into the expected text, with
    /// Plumed's regex engine (the rules repo checks them with Python's `re`).
    #[test]
    fn each_rule_fixes_its_example() {
        let failures = mismatches(|rule| rules_only(vec![rule]));
        assert!(failures.is_empty(), "{failures}");
    }

    /// All rules together still give each example's expected text: no rule
    /// undoes another one's work.
    #[test]
    fn rules_do_not_interfere() {
        let all = rules_only(default_rules());
        let failures = mismatches(|_| all.clone());
        assert!(failures.is_empty(), "{failures}");
    }
}
