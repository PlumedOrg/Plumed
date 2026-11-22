//! Tests with the real Grammalecte, from `resources/grammalecte/` (run
//! `scripts/fetch-grammalecte.sh` first; CI does). Written with Claude Code.

use super::engine::{Grammalecte, JS_STACK_BYTES};
use super::mask;
use super::procedure::{self, Checker, Config};
use super::rules::{RuleSet, DATA};
use super::worker;
use std::collections::HashSet;
use std::path::PathBuf;

fn resources() -> Option<PathBuf> {
    Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/grammalecte"))
}

/// Through the app's worker thread, which loads Grammalecte once for all tests.
fn correct_with(text: &str, rules: &RuleSet) -> String {
    worker::correct(resources, text, rules.clone())
        .unwrap_or_else(|e| panic!("{e} (run scripts/fetch-grammalecte.sh)"))
}

fn defaults() -> RuleSet {
    RuleSet::from_overlay(&DATA.rules, &Default::default())
}

#[test]
fn an_age_does_not_hide_the_participle_agreement() {
    assert_eq!(
        correct_with("La patiente de 63 ans est fatigué.", &defaults()),
        "La patiente de 63 ans est fatiguée."
    );
}

#[test]
fn doses_and_acronyms_are_left_alone() {
    let text = "Le patient prend 500 mg de PARACETAMOL depuis 3 jours.";
    assert_eq!(correct_with(text, &defaults()), text);
}

#[test]
fn a_title_and_name_is_guarded() {
    // mask.py's title_name_guard covers "Madame Durand"; Grammalecte reports nothing here.
    let text = "Madame Durand est opéré.";
    assert_eq!(correct_with(text, &defaults()), text);
}

#[test]
fn rules_off_by_default_are_not_applied() {
    let text = "Les examens biologique sont normaux.";
    assert_eq!(correct_with(text, &defaults()), text);
    let with_family = RuleSet::new(["g3__gn_les_2m".to_string()]);
    assert_eq!(
        correct_with(text, &with_family),
        "Les examens biologiques sont normaux."
    );
}

#[test]
fn offsets_survive_characters_outside_the_bmp() {
    assert_eq!(
        correct_with("Note 😀. La patiente est fatigué.", &defaults()),
        "Note 😀. La patiente est fatiguée."
    );
}

#[test]
fn a_sentence_takes_a_few_milliseconds() {
    let text = "La patiente de 63 ans est fatigué depuis trois jours, elle est mis sous \
                venlafaxine et les examens biologique sont normaux.";
    correct_with(text, &defaults());
    let started = std::time::Instant::now();
    for _ in 0..20 {
        correct_with(text, &defaults());
    }
    let per_sentence = started.elapsed() / 20;
    eprintln!("grammar step: {per_sentence:?} per sentence");
    // Generous for debug builds and slow CI machines; release is about 10 ms.
    assert!(per_sentence < std::time::Duration::from_millis(500));
}

/// Parity with the reference (eval/mask.py + the allowlist + Grammalecte's Python package,
/// see the session handoff): LOCAL ONLY, the reference file holds private dictation text
/// and is never committed. `PLUMED_GRAMMAR_PARITY` points to a JSONL file of
/// {"text", "masked", "expected", "expected_catalog"} lines.
#[test]
#[ignore]
fn parity_with_the_python_reference() {
    let path = std::env::var("PLUMED_GRAMMAR_PARITY").expect("set PLUMED_GRAMMAR_PARITY");
    let content = std::fs::read_to_string(path).unwrap();
    // Its own engine, on a thread with the stack the worker thread gets.
    std::thread::Builder::new()
        .stack_size(2 * JS_STACK_BYTES)
        .spawn(move || check_parity(&content))
        .unwrap()
        .join()
        .unwrap();
}

fn check_parity(content: &str) {
    #[derive(serde::Deserialize)]
    struct Line {
        text: String,
        masked: String,
        expected: String,
        expected_catalog: String,
    }
    let mut engine = Grammalecte::load(&resources().unwrap()).unwrap();
    let masks = mask::compile(&DATA.masks).unwrap();
    let skip: HashSet<String> = DATA.skip_types.iter().cloned().collect();
    let defaults = defaults();
    let catalog = RuleSet::new(DATA.rules.iter().map(|r| r.id.clone()));
    let (mut total, mut changed, mut changed_catalog) = (0, 0, 0);
    let mut mismatches = Vec::new();
    let mut spell_time = std::time::Duration::ZERO;
    let mut mask_time = std::time::Duration::ZERO;
    let mut check_time = std::time::Duration::ZERO;
    for line in content.lines() {
        let line: Line = serde_json::from_str(line).unwrap();
        total += 1;
        changed += usize::from(line.expected != line.text);
        changed_catalog += usize::from(line.expected_catalog != line.text);
        let started = std::time::Instant::now();
        let unknown = engine.unknown_words(&line.text).unwrap();
        spell_time += started.elapsed();
        let started = std::time::Instant::now();
        let masked = mask::mask(&line.text, &masks, &unknown).unwrap();
        mask_time += started.elapsed();
        let started = std::time::Instant::now();
        engine.grammar_errors(&masked.text).unwrap();
        check_time += started.elapsed();
        let mut run = |rules: &RuleSet| {
            let config = Config {
                rules,
                masks: &masks,
                skip_types: &skip,
            };
            procedure::correct(&line.text, &mut engine, &config).unwrap()
        };
        let got = run(&defaults);
        let got_catalog = run(&catalog);
        for (what, got, want) in [
            ("masked", &masked.text, &line.masked),
            ("default rules", &got, &line.expected),
            ("whole catalog", &got_catalog, &line.expected_catalog),
        ] {
            if got != want {
                mismatches.push(format!("{what}:\n  got  {got}\n  want {want}"));
            }
        }
    }
    eprintln!(
        "parity: {total} lines ({changed} changed by the default rules, {changed_catalog} by \
         the whole catalog), {} mismatches",
        mismatches.len()
    );
    eprintln!(
        "per line: spell checker {:?}, masks {:?}, grammar check {:?}",
        spell_time / total.max(1) as u32,
        mask_time / total.max(1) as u32,
        check_time / total.max(1) as u32
    );
    for m in mismatches.iter().take(20) {
        eprintln!("{m}");
    }
    assert!(mismatches.is_empty());
}
