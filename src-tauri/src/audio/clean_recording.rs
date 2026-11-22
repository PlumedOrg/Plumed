static FILLER_WORDS: &[&str] = &[
    "euh", "hmm", "hm", "mmm", "uh", "um", "uhm", "umm", "uhh", "uhhh", "ah", "mm", "mh", "eh",
    "ehh", "ha", "mm-hmm",
];

fn is_filler(word: &str) -> bool {
    let normalized = word.trim_matches(|c: char| !c.is_alphanumeric());
    if normalized.is_empty() {
        return false;
    }
    let lower = normalized.to_lowercase();
    FILLER_WORDS.contains(&lower.as_str())
}

pub(super) fn clean_transcription(text: &str, remove_hesitations: bool) -> String {
    let words: Vec<&str> = text
        .split_whitespace()
        .filter(|w| !remove_hesitations || !is_filler(w))
        .collect();
    if words.is_empty() {
        return String::new();
    }

    let mut result: Vec<&str> = Vec::with_capacity(words.len());
    let mut i = 0;

    while i < words.len() {
        let current_lower = words[i].to_lowercase();
        let mut count = 1;

        while i + count < words.len() && words[i + count].to_lowercase() == current_lower {
            count += 1;
        }

        if count >= 3 {
            result.push(words[i]);
            result.push(words[i + 1]);
        } else {
            for j in 0..count {
                result.push(words[i + j]);
            }
        }

        i += count;
    }

    result.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedup_four_to_two() {
        assert_eq!(clean_transcription("je je je je vais", true), "je je vais");
    }

    #[test]
    fn dedup_two_kept_unchanged() {
        assert_eq!(clean_transcription("oui oui", true), "oui oui");
    }

    #[test]
    fn dedup_five_to_two() {
        assert_eq!(
            clean_transcription("the the the the the cat", true),
            "the the cat"
        );
    }

    #[test]
    fn dedup_three_to_two_case_insensitive() {
        assert_eq!(
            clean_transcription("Hello HELLO hello world", true),
            "Hello HELLO world"
        );
    }

    #[test]
    fn dedup_no_repetition() {
        assert_eq!(
            clean_transcription("normal sentence", true),
            "normal sentence"
        );
    }

    #[test]
    fn dedup_empty_string() {
        assert_eq!(clean_transcription("", true), "");
    }

    #[test]
    fn dedup_single_word() {
        assert_eq!(clean_transcription("word", true), "word");
    }

    #[test]
    fn dedup_multiple_groups() {
        assert_eq!(
            clean_transcription("the the the cat the the the dog", true),
            "the the cat the the dog"
        );
    }

    #[test]
    fn dedup_exactly_three_to_two() {
        assert_eq!(
            clean_transcription("hello hello hello world", true),
            "hello hello world"
        );
    }

    #[test]
    fn dedup_one_occurrence_unchanged() {
        assert_eq!(clean_transcription("hello world", true), "hello world");
    }

    #[test]
    fn filler_isolated_removed() {
        assert_eq!(clean_transcription("je euh vais", true), "je vais");
    }

    #[test]
    fn filler_repeated_fully_removed() {
        assert_eq!(clean_transcription("euh euh euh bonjour", true), "bonjour");
    }

    #[test]
    fn filler_substring_in_real_word_kept() {
        assert_eq!(
            clean_transcription("aujourd'hui ah hammer", true),
            "aujourd'hui hammer"
        );
    }

    #[test]
    fn filler_mm_hmm_removed() {
        assert_eq!(
            clean_transcription("oui mm-hmm bonjour", true),
            "oui bonjour"
        );
    }

    #[test]
    fn hesitations_kept_when_disabled() {
        assert_eq!(clean_transcription("je euh vais", false), "je euh vais");
    }

    #[test]
    fn hesitations_only_text_kept_when_disabled() {
        assert_eq!(clean_transcription("um", false), "um");
    }

    #[test]
    fn repeats_deduped_while_hesitations_kept() {
        assert_eq!(
            clean_transcription("euh euh euh bonjour bonjour bonjour", false),
            "euh euh bonjour bonjour"
        );
    }
}
