//! What a medical data update changes, line by line, for the optional
//! review in the update dialog. Lines are compared as sets per file (order
//! and duplicates ignored): a rule rewritten counts as one line removed and
//! one added. Written with Claude Code.

use serde::Serialize;
use std::collections::{BTreeSet, HashMap};

/// Lines shown per side and per file, the counts stay exact.
pub const MAX_SAMPLES: usize = 50;

/// Changes of one file of the set.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FileDiff {
    pub name: String,
    pub added: usize,
    pub removed: usize,
    /// At most [`MAX_SAMPLES`] of the added lines, sorted.
    pub added_samples: Vec<String>,
    pub removed_samples: Vec<String>,
}

fn lines(text: &str) -> BTreeSet<&str> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect()
}

/// The changed files among `names`, from `old` to `new`. A file missing
/// from `new` is unchanged (an optional file left out keeps the current one).
/// Example: old `{"a.txt": "x\ny"}`, new `{"a.txt": "y\nz"}` gives one
/// `FileDiff { name: "a.txt", added: 1, removed: 1, ["z"], ["x"] }`.
pub fn diff_files(
    names: &[&str],
    old: &HashMap<String, String>,
    new: &HashMap<String, String>,
) -> Vec<FileDiff> {
    names
        .iter()
        .filter_map(|name| {
            let new_lines = lines(new.get(*name)?);
            let old_lines = lines(old.get(*name).map_or("", String::as_str));
            let added: Vec<&str> = new_lines.difference(&old_lines).copied().collect();
            let removed: Vec<&str> = old_lines.difference(&new_lines).copied().collect();
            let samples = |l: &[&str]| l.iter().take(MAX_SAMPLES).map(|s| s.to_string()).collect();
            (!added.is_empty() || !removed.is_empty()).then(|| FileDiff {
                name: name.to_string(),
                added: added.len(),
                removed: removed.len(),
                added_samples: samples(&added),
                removed_samples: samples(&removed),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn counts_added_and_removed_lines() {
        let old = map(&[("a", "x\ny\n\n"), ("b", "same")]);
        let new = map(&[("a", "y\nz\n w "), ("b", "same")]);
        let diff = diff_files(&["a", "b"], &old, &new);
        assert_eq!(
            diff,
            vec![FileDiff {
                name: "a".into(),
                added: 2,
                removed: 1,
                added_samples: vec!["w".into(), "z".into()],
                removed_samples: vec!["x".into()],
            }]
        );
    }

    #[test]
    fn missing_new_file_is_unchanged_and_new_file_is_all_added() {
        let old = map(&[("a", "x")]);
        let new = map(&[("b", "1\n2")]);
        let diff = diff_files(&["a", "b"], &old, &new);
        assert_eq!(diff.len(), 1);
        assert_eq!(
            (diff[0].name.as_str(), diff[0].added, diff[0].removed),
            ("b", 2, 0)
        );
    }

    #[test]
    fn samples_are_capped_but_counts_are_not() {
        let text: String = (0..MAX_SAMPLES + 10).map(|i| format!("{i}\n")).collect();
        let diff = diff_files(&["a"], &HashMap::new(), &map(&[("a", &text)]));
        assert_eq!(diff[0].added, MAX_SAMPLES + 10);
        assert_eq!(diff[0].added_samples.len(), MAX_SAMPLES);
    }
}
