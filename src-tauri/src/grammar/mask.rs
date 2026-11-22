//! The masks around Grammalecte: contexts that mislead its agreement rules (ages, doses,
//! acronyms, "Madame Durand"...) are deleted from or guarded in the text it sees, and its
//! errors are mapped back to the original text. A port of the evaluation's `mask.py`
//! (`spans`, `mask_generic`, `map_back`), which the parity test compares byte for byte.
//! All offsets are in code points, like Python's. Written with Claude Code.
//!
//! The mask regexes are kept in Python `re` syntax in `grammar_rules.json` and translated
//! here, so that they match exactly as in Python: Python's `\w` is any letter or number
//! (`[\p{L}\p{N}_]`, where the `regex` crate's `\w` also has marks and lacks numbers like
//! "²"), `\b` is a boundary of that `\w`, `\s` also has the separators U+001C to U+001F,
//! and `$` also matches before a final newline.

use super::rules::{MaskAction, MaskSpec};
use fancy_regex::Regex;

const PY_WORD: &str = r"\p{L}\p{N}_";
const PY_SPACE: &str = r"\s\x1c-\x1f";

/// `pattern` (Python `re` syntax, no flags) in `fancy_regex` syntax, same matches.
/// Example: `\bx(?!\w)` becomes
/// `(?:(?<=[\p{L}\p{N}_])(?![\p{L}\p{N}_])|(?<![\p{L}\p{N}_])(?=[\p{L}\p{N}_]))x(?![\p{L}\p{N}_])`.
pub fn python_to_fancy(pattern: &str) -> String {
    let word = format!("[{PY_WORD}]");
    let boundary = format!("(?:(?<={word})(?!{word})|(?<!{word})(?={word}))");
    let mut out = String::with_capacity(pattern.len() * 2);
    let mut in_class = false;
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, in_class) {
            ('\\', _) => match (chars.next(), in_class) {
                (Some('w'), false) => out.push_str(&word),
                (Some('w'), true) => out.push_str(PY_WORD),
                (Some('s'), false) => {
                    out.push('[');
                    out.push_str(PY_SPACE);
                    out.push(']');
                }
                (Some('s'), true) => out.push_str(PY_SPACE),
                (Some('b'), false) => out.push_str(&boundary),
                (Some(other), _) => {
                    out.push('\\');
                    out.push(other);
                }
                (None, _) => out.push('\\'),
            },
            ('[', false) => {
                in_class = true;
                out.push('[');
                // A `]` or `^]` right after the opening bracket is a literal, as in Python.
                if chars.peek() == Some(&'^') {
                    out.push('^');
                    chars.next();
                }
                if chars.peek() == Some(&']') {
                    out.push_str(r"\]");
                    chars.next();
                }
            }
            (']', true) => {
                in_class = false;
                out.push(']');
            }
            ('[', true) => out.push_str(r"\["),
            ('$', false) => out.push_str(r"(?=\n?\z)"),
            (other, _) => out.push(other),
        }
    }
    out
}

/// A mask ready to run: its regex, or `None` for the spell checker's unknown words.
pub struct Mask {
    regex: Option<Regex>,
    action: MaskAction,
}

/// Compiles the masks of the spec, in order.
pub fn compile(specs: &[MaskSpec]) -> Result<Vec<Mask>, String> {
    specs
        .iter()
        .map(|spec| {
            let regex = match &spec.regex {
                None => None,
                Some(pattern) => {
                    let flags = if spec.case_sensitive { "" } else { "(?i)" };
                    let translated = format!("{flags}{}", python_to_fancy(pattern));
                    Some(Regex::new(&translated).map_err(|e| format!("mask {}: {e}", spec.name))?)
                }
            };
            Ok(Mask {
                regex,
                action: spec.action,
            })
        })
        .collect()
}

/// A masked span of the original text, in code points.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Span {
    start: usize,
    end: usize,
    action: MaskAction,
}

/// Code point index of each byte offset that starts a character, and of the end.
/// Example: "été" gives [0, 0, 1, 2, 2, 3] (only offsets 0, 2, 3 and 5 are used).
fn char_index(text: &str) -> Vec<usize> {
    let mut map = vec![0; text.len() + 1];
    for (index, (byte, _)) in text.char_indices().enumerate() {
        map[byte] = index;
    }
    map[text.len()] = text.chars().count();
    map
}

/// The regex masks' spans: every match of every mask (empty ones skipped), sorted by
/// start, then longest first, then mask order; a span is kept when it starts at or after
/// the end of the last kept one (`mask.py` `spans`).
fn regex_spans(text: &str, masks: &[Mask]) -> Result<Vec<Span>, String> {
    let to_char = char_index(text);
    let mut found: Vec<(usize, usize, usize, MaskAction)> = Vec::new();
    for (order, mask) in masks.iter().enumerate() {
        let Some(regex) = &mask.regex else { continue };
        for m in regex.find_iter(text) {
            let m = m.map_err(|e| format!("mask regex: {e}"))?;
            if m.end() > m.start() {
                found.push((to_char[m.start()], to_char[m.end()], order, mask.action));
            }
        }
    }
    found.sort_by_key(|&(start, end, order, _)| (start, std::cmp::Reverse(end - start), order));
    let mut kept: Vec<Span> = Vec::new();
    let mut last_end = 0;
    for (start, end, _, action) in found {
        if start >= last_end {
            kept.push(Span { start, end, action });
            last_end = end;
        }
    }
    Ok(kept)
}

/// The text Grammalecte sees and how to map its offsets back.
#[derive(Debug, Clone, PartialEq)]
pub struct Masked {
    pub text: String,
    /// Kept stretches of the original: (masked start, masked end, original start).
    segments: Vec<(usize, usize, usize)>,
    /// Replaced or guarded spans, in masked coordinates (a deletion is zero-length).
    holes: Vec<(usize, usize)>,
}

/// Masks `text`; `unknown` are the spell checker's unknown words (code points), guarded
/// where no regex mask already covers them (`mask.py` `mask_generic` with `unk_extra`).
pub fn mask(text: &str, masks: &[Mask], unknown: &[(usize, usize)]) -> Result<Masked, String> {
    let mut spans = regex_spans(text, masks)?;
    if masks.iter().any(|m| m.regex.is_none()) {
        let extra: Vec<Span> = unknown
            .iter()
            .filter(|(s, e)| !spans.iter().any(|k| *s < k.end && *e > k.start))
            .map(|&(start, end)| Span {
                start,
                end,
                action: MaskAction::Guard,
            })
            .collect();
        spans.extend(extra);
        spans.sort_by_key(|s| (s.start, s.end));
    }
    let chars: Vec<char> = text.chars().collect();
    let mut masked = String::with_capacity(text.len());
    let (mut segments, mut holes) = (Vec::new(), Vec::new());
    let (mut pos, mut mpos) = (0usize, 0usize);
    for span in spans {
        masked.extend(&chars[pos..span.start]);
        segments.push((mpos, mpos + span.start - pos, pos));
        mpos += span.start - pos;
        let replacement: &[char] = match span.action {
            MaskAction::Replace => &[],
            MaskAction::Guard => &chars[span.start..span.end],
        };
        masked.extend(replacement);
        holes.push((mpos, mpos + replacement.len()));
        mpos += replacement.len();
        pos = span.end;
    }
    masked.extend(&chars[pos..]);
    segments.push((mpos, mpos + chars.len() - pos, pos));
    Ok(Masked {
        text: masked,
        segments,
        holes,
    })
}

impl Masked {
    /// The original span of the masked span `[start, end]`, `None` when it touches a hole,
    /// strictly contains a deletion, or straddles two kept stretches (`mask.py` `map_back`).
    pub fn map_back(&self, start: usize, end: usize) -> Option<(usize, usize)> {
        let touches_hole = self
            .holes
            .iter()
            .any(|&(hs, he)| (start < he && end > hs) || (hs == he && start < hs && hs < end));
        if touches_hole {
            return None;
        }
        self.segments
            .iter()
            .find(|&&(ms, me, _)| ms <= start && end <= me)
            .map(|&(ms, _, original)| (start + original - ms, end + original - ms))
    }
}

#[cfg(test)]
mod tests {
    use super::super::rules::DATA;
    use super::*;

    fn masks() -> Vec<Mask> {
        compile(&DATA.masks).unwrap()
    }

    #[test]
    fn python_classes_are_translated() {
        assert_eq!(python_to_fancy(r"a\.b"), r"a\.b");
        assert_eq!(python_to_fancy(r"[\w,.]"), r"[\p{L}\p{N}_,.]");
        assert_eq!(python_to_fancy(r"\s+"), r"[\s\x1c-\x1f]+");
        assert_eq!(python_to_fancy(r"x$"), r"x(?=\n?\z)");
        assert_eq!(python_to_fancy(r"[$]"), r"[$]");
    }

    #[test]
    fn python_word_class_has_numbers_but_not_marks() {
        let re = Regex::new(&python_to_fancy(r"cm(?!\w)")).unwrap();
        // "²" is a number (No) for Python's \w, so "cm²" does not end after "cm".
        assert!(!re.is_match("3 cm² de").unwrap());
        assert!(re.is_match("3 cm de").unwrap());
        let boundary = Regex::new(&python_to_fancy(r"\bmg\b")).unwrap();
        assert!(boundary.is_match("500 mg.").unwrap());
        assert!(!boundary.is_match("500mgé").unwrap());
    }

    #[test]
    fn every_mask_compiles() {
        assert_eq!(masks().len(), 18);
    }

    #[test]
    fn an_age_is_deleted_and_offsets_map_back() {
        let text = "La patiente de 63 ans est fatigué.";
        let masked = mask(text, &masks(), &[]).unwrap();
        assert_eq!(masked.text, "La patiente est fatigué.");
        // "fatigué" at 16..23 in the masked text is at 26..33 in the original.
        assert_eq!(masked.map_back(16, 23), Some((26, 33)));
        // A span over the deletion point is dropped.
        assert_eq!(masked.map_back(3, 15), None);
    }

    #[test]
    fn guards_keep_the_text_but_drop_fixes_touching_them() {
        let text = "Le patient prend 500 mg de PARACETAMOL.";
        let masked = mask(text, &masks(), &[]).unwrap();
        assert_eq!(masked.text, text);
        let acronym = text.find("PARACETAMOL").unwrap();
        assert_eq!(masked.map_back(acronym, acronym + 11), None);
        assert_eq!(masked.map_back(3, 10), Some((3, 10)));
    }

    #[test]
    fn unknown_words_are_guarded_unless_a_regex_mask_covers_them() {
        let text = "Le patient prend du Zorglub.";
        let start = text.find("Zorglub").unwrap();
        let masked = mask(text, &masks(), &[(start, start + 7)]).unwrap();
        assert_eq!(masked.text, text);
        assert_eq!(masked.map_back(start, start + 7), None);
        assert_eq!(masked.map_back(0, 2), Some((0, 2)));
    }
}
