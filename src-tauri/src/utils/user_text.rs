//! Checks on what the user, or a file they import, adds to a list whose text
//! ends up in the transcription (formatting rules, dictionary, medical
//! lists). Written with Claude Code.

/// Most entries of their own a list accepts (added formatting rules,
/// dictionary words, added medical phrases, groups or rules): far above any
/// real list, low enough that a huge or hostile import file is refused
/// instead of slowing every dictation down.
pub const MAX_USER_ENTRIES: usize = 50_000;

/// Refuses more than [`MAX_USER_ENTRIES`] entries of `what` ("dictionary
/// words", ...).
pub fn check_entry_count(count: usize, what: &str) -> Result<(), String> {
    match count > MAX_USER_ENTRIES {
        true => Err(format!(
            "Too many {what}: {count}, at most {MAX_USER_ENTRIES} are accepted"
        )),
        false => Ok(()),
    }
}

/// Control characters, bidi overrides and invisible format characters. The
/// text can be typed as keys or land in the clipboard, so a tampered rule or
/// list must not be able to inject them: a control character acts as a key,
/// a bidi override flips what the record shows, and an invisible character
/// (soft hyphen, zero-width space, Unicode tags) hides inside a word, so a
/// drug name no longer matches a search of the patient record, or carries
/// text nobody sees. The narrow no-break space (U+202F) of French typography
/// stays allowed.
fn is_unsafe_char(c: char) -> bool {
    matches!(c,
        '\u{0}'..='\u{1f}'
        | '\u{7f}'..='\u{9f}'
        | '\u{ad}'
        | '\u{61c}'
        | '\u{180e}'
        | '\u{200b}'..='\u{200f}'
        | '\u{2028}'..='\u{202e}'
        | '\u{2060}'..='\u{206f}'
        | '\u{feff}'
        | '\u{fff9}'..='\u{fffb}'
        | '\u{e0000}'..='\u{e007f}')
}

/// [`is_unsafe_char`], line breaks (`\n`) and tabs aside.
pub fn is_unsafe_char_besides_lines(c: char) -> bool {
    !matches!(c, '\n' | '\t') && is_unsafe_char(c)
}

/// Whether `text` has a control or bidi-override character.
pub fn has_unsafe_chars(text: &str) -> bool {
    text.chars().any(is_unsafe_char)
}

/// Same as [`has_unsafe_chars`], but line breaks (`\n`) and tabs are allowed:
/// a formatting rule may insert several lines ("à la ligne", templates).
pub fn has_unsafe_chars_besides_lines(text: &str) -> bool {
    text.chars().any(is_unsafe_char_besides_lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_and_bidi_characters_are_unsafe() {
        assert!(!has_unsafe_chars("Kardégic 75 mg, à jeun"));
        assert!(!has_unsafe_chars("Traitement\u{202f}: 1\u{a0}cp"));
        for text in [
            "a\u{8}",
            "\u{1b}",
            "\r",
            "\n",
            "\t",
            "\u{7f}",
            "\u{85}",
            "\u{202e}x",
            "\u{2066}",
            "Kard\u{ad}égic",
            "Kard\u{200b}égic",
            "\u{200f}",
            "\u{2028}",
            "\u{2060}",
            "\u{feff}",
            "\u{e0041}",
        ] {
            assert!(has_unsafe_chars(text), "{text:?}");
        }
    }

    #[test]
    fn rule_replacements_may_hold_lines_and_tabs_only() {
        assert!(!has_unsafe_chars_besides_lines("Bilan :\n-HD\tok\n\n"));
        assert!(has_unsafe_chars_besides_lines("a\r\nb"));
        assert!(has_unsafe_chars_besides_lines("ok\u{8}\u{8}"));
        assert!(has_unsafe_chars_besides_lines("\u{202e}"));
    }

    #[test]
    fn entry_count_is_capped() {
        assert!(check_entry_count(MAX_USER_ENTRIES, "words").is_ok());
        let error = check_entry_count(MAX_USER_ENTRIES + 1, "words").unwrap_err();
        assert!(error.contains("50000"), "{error}");
    }
}
