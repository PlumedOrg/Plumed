//! Confusion-group second pass (Plumed fork, written with Claude Code).
//!
//! Some decoded words are the mishearing of several different words, e.g.
//! "singulaire" for both "Singulair" and "cingulaire". A regex can only map
//! such a form to one replacement, so the UltiMed rules leave them out and
//! list them in `confusion_groups.jsonl` instead:
//!
//! ```text
//! {"heard": "singulaire", "candidates": [{"text": "Singulair", "count": 11},
//!  {"text": "cingulaire", "count": 11}], "heard_correct_count": 3, ...}
//! ```
//!
//! After decoding, every decoded word (or run of up to 3 words) matching a
//! `heard` form is re-scored on the encoder frames it spans, together with the
//! previous word: an elided sound ("voire rituximab" shares one [ʁ]) belongs
//! to both words, so each candidate is judged with its left context. A
//! candidate's score is the best TDT alignment (Viterbi) of the previous
//! word's tokens plus the candidate's tokens over those frames, from the
//! model's unboosted log-probabilities. All candidates explain the same
//! frames, so their scores compare directly. The decoded word is replaced
//! only when a candidate scores higher (by more than `margin` nats): when the audio cannot
//! tell them apart (singular and plural often sound the same), the decoded
//! text stays and the regex pass still runs after it.
//!
//! The encoder is not re-run: a match costs one batched decoder call per
//! frame of its window (about 20 to 40), plus replaying the decoder state
//! over the tokens before the first match.

use std::collections::HashMap;

use ndarray::ArrayViewD;
use serde::Deserialize;
use tokenizers::Tokenizer;

use super::beam::log_sum_exp;
use super::engine::DecodedSequence;
use super::types::{DecoderState, ParakeetEngine, ParakeetError, ParakeetModel};

/// Nats a candidate must win by over the decoded word to replace it. On the
/// v2 model, 233 held-out confusion cases: margin 0 fixed 6 and broke none,
/// margin 1 fixed 1, margin 2 none; on the 400 general benchmark clips margin 0
/// changed no output.
pub const DEFAULT_MARGIN: f64 = 0.0;
/// Windows longer than this (frames of 80 ms) are skipped: a heard form is at
/// most 3 words, so a longer window means a timestamp oddity.
const MAX_WINDOW_FRAMES: usize = 150;

#[derive(Deserialize)]
struct RawCandidate {
    text: String,
    count: u32,
}

#[derive(Deserialize)]
struct RawGroup {
    heard: String,
    candidates: Vec<RawCandidate>,
    heard_correct_count: u32,
}

impl RawGroup {
    /// The realistic working set (about 140 of the 905 v2 groups): a second
    /// candidate seen at least twice, and a heard form wrong at least as often
    /// as right. The others are single-target (the regexes handle them) or
    /// a heard form that is almost always right.
    fn is_ambiguous(&self) -> bool {
        let total: u32 = self.candidates.iter().map(|c| c.count).sum();
        self.candidates.get(1).is_some_and(|c| c.count >= 2) && self.heard_correct_count <= total
    }
}

/// Confusion groups indexed by normalized heard form.
pub struct ConfusionGroups {
    by_heard: HashMap<String, Vec<String>>,
    max_words: usize,
    pub margin: f64,
}

impl ConfusionGroups {
    /// Parses `confusion_groups.jsonl` content, possibly several files
    /// concatenated. Unreadable lines are skipped with a warning.
    pub fn parse(raw: &str) -> Self {
        let mut by_heard: HashMap<String, Vec<String>> = HashMap::new();
        for (index, line) in raw.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let group: RawGroup = match serde_json::from_str(line) {
                Ok(group) => group,
                Err(e) => {
                    log::warn!("confusion groups: line {} skipped: {}", index + 1, e);
                    continue;
                }
            };
            if !group.is_ambiguous() {
                continue;
            }
            let key = group
                .heard
                .split_whitespace()
                .map(normalize)
                .collect::<Vec<_>>()
                .join(" ");
            let candidates = by_heard.entry(key).or_default();
            for candidate in group.candidates {
                if !candidates.contains(&candidate.text) {
                    candidates.push(candidate.text);
                }
            }
        }
        let max_words = by_heard
            .keys()
            .map(|k| k.split(' ').count())
            .max()
            .unwrap_or(0);
        Self {
            by_heard,
            max_words,
            margin: DEFAULT_MARGIN,
        }
    }

    pub fn len(&self) -> usize {
        self.by_heard.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_heard.is_empty()
    }
}

/// Lowercase, letters, digits and hyphens only (the heard forms' alphabet).
fn normalize(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_apostrophe(c: char) -> bool {
    c == '\'' || c == '’'
}

/// A decoded word as a token range. `core` skips an elided prefix ("l'" in
/// "l'épistaxie", matched as "épistaxie"); `end` excludes trailing
/// punctuation tokens, which stay outside any replacement.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Word {
    start: usize,
    core: usize,
    end: usize,
}

/// Splits token strings (word starts carry a leading space) into words.
fn split_words(tokens: &[&str]) -> Vec<Word> {
    let mut starts: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter(|(i, t)| *i == 0 || t.starts_with(' '))
        .map(|(i, _)| i)
        .collect();
    starts.push(tokens.len());
    starts
        .windows(2)
        .filter_map(|pair| {
            let (start, next) = (pair[0], pair[1]);
            let has_content = |t: &&str| t.chars().any(char::is_alphanumeric);
            let end = start + tokens[start..next].iter().rposition(has_content)? + 1;
            let core = tokens[start..end]
                .iter()
                .rposition(|t| t.ends_with(is_apostrophe))
                .map_or(start, |i| start + i + 1);
            match core < end {
                true => Some(Word { start, core, end }),
                false => None,
            }
        })
        .collect()
}

/// One replacement decided by the pass: tokens `[start, end)` become `ids`.
struct Replacement {
    start: usize,
    end: usize,
    ids: Vec<i32>,
}

/// Decoder state replayed along the decoded tokens, so a window starts from
/// the state the decoder had there. `target` and `state` are the joint input
/// once `next` tokens are emitted (`target` is the last one, blank at first).
struct Replay {
    next: usize,
    target: i32,
    state: DecoderState,
}

/// Unboosted log-probabilities one alignment step needs at (frame, prefix).
#[derive(Clone)]
struct StepScores {
    token: f64,
    blank: f64,
    durations: Vec<f64>,
}

/// Best alignment score of `steps[t][u]` (u tokens emitted at frame t) over
/// `frames` frames: blanks advance at least one frame, tokens may stay.
fn viterbi(steps: &[Vec<StepScores>], frames: usize, tokens: usize) -> f64 {
    let mut alpha = vec![vec![f64::NEG_INFINITY; tokens + 1]; frames + 1];
    alpha[0][0] = 0.0;
    for t in 0..frames {
        for u in 0..=tokens {
            let here = alpha[t][u];
            if here == f64::NEG_INFINITY {
                continue;
            }
            let step = &steps[t][u];
            for (d, &duration) in step.durations.iter().enumerate() {
                let to = (t + d.max(1)).min(frames);
                alpha[to][u] = alpha[to][u].max(here + step.blank + duration);
                if u < tokens {
                    let to = (t + d).min(frames);
                    alpha[to][u + 1] = alpha[to][u + 1].max(here + step.token + duration);
                }
            }
        }
    }
    alpha[frames][tokens]
}

/// Gives `candidate` the decoded word's leading capital ("Communiquant" at a
/// sentence start stays capitalized).
fn match_case(decoded: &str, candidate: &str) -> String {
    let decoded_upper = decoded
        .trim_start()
        .chars()
        .next()
        .is_some_and(char::is_uppercase);
    let mut chars = candidate.chars();
    match (decoded_upper, chars.next()) {
        (true, Some(first)) if first.is_lowercase() => first.to_uppercase().chain(chars).collect(),
        _ => candidate.to_string(),
    }
}

impl ParakeetModel {
    pub fn set_confusion_groups(&mut self, raw: &str) {
        let groups = ConfusionGroups::parse(raw);
        log::info!("confusion groups: {} heard forms armed", groups.len());
        self.confusion = match groups.is_empty() {
            true => None,
            false => Some(groups),
        };
    }

    /// Runs the second pass on a decoded sequence (see the module doc).
    pub(super) fn rescore_confusions(
        &mut self,
        encodings: &ArrayViewD<f32>,
        encodings_len: usize,
        sequence: DecodedSequence,
    ) -> Result<DecodedSequence, ParakeetError> {
        let (Some(groups), Some(tokenizer)) = (self.confusion.take(), self.tokenizer.take()) else {
            return Ok(sequence);
        };
        let replacements =
            self.find_replacements(&groups, &tokenizer, encodings, encodings_len, &sequence);
        self.confusion = Some(groups);
        self.tokenizer = Some(tokenizer);
        Ok(apply_replacements(sequence, replacements?))
    }

    fn find_replacements(
        &mut self,
        groups: &ConfusionGroups,
        tokenizer: &Tokenizer,
        encodings: &ArrayViewD<f32>,
        encodings_len: usize,
        sequence: &DecodedSequence,
    ) -> Result<Vec<Replacement>, ParakeetError> {
        let (ids, timestamps, _) = sequence;
        let owned: Vec<String> = ids
            .iter()
            .map(|&id| self.token_text(id).to_string())
            .collect();
        let texts: Vec<&str> = owned.iter().map(String::as_str).collect();
        let words = split_words(&texts);
        let mut replay: Option<Replay> = None;
        let mut replacements = Vec::new();
        let mut w = 0;
        while w < words.len() {
            let Some((len, candidates)) = Self::match_at(groups, &texts, &words[w..]) else {
                w += 1;
                continue;
            };
            let first = words[w];
            let last = words[w + len - 1];
            let context = match w {
                0 => first.start,
                _ => words[w - 1].start,
            };
            let window_start = timestamps[context];
            let window_end = timestamps.get(last.end).copied().unwrap_or(encodings_len);
            let frames = window_end.saturating_sub(window_start);
            if frames == 0 || frames > MAX_WINDOW_FRAMES {
                w += len;
                continue;
            }
            let decoded: String = texts[first.core..last.end].concat();
            let prefix: String = texts[first.start..first.core].concat();
            let mut options: Vec<Vec<i32>> = vec![ids[first.start..last.end].to_vec()];
            for candidate in candidates {
                let text = format!("{}{}", prefix.trim_start(), match_case(&decoded, candidate));
                if let Some(candidate_ids) = super::helpers::tokenize_word_to_ids(tokenizer, &text)
                {
                    if !options.contains(&candidate_ids) {
                        options.push(candidate_ids);
                    }
                }
            }
            if options.len() > 1 {
                let start_row = self.replay_to(&mut replay, ids, encodings, context)?;
                let left = &ids[context..first.start];
                let sequences: Vec<Vec<i32>> = options
                    .iter()
                    .map(|option| [left, option.as_slice()].concat())
                    .collect();
                let scores =
                    self.align_scores(encodings, window_start, frames, start_row, &sequences)?;
                let best = (1..scores.len())
                    .max_by(|&a, &b| scores[a].total_cmp(&scores[b]))
                    .filter(|&b| scores[b] - scores[0] > groups.margin);
                log::debug!(
                    "confusion pass: {} options over {} frames, margin {:.2}, replaced: {}",
                    options.len(),
                    frames,
                    scores[1..].iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)) - scores[0],
                    best.is_some()
                );
                if let Some(best) = best {
                    replacements.push(Replacement {
                        start: first.start,
                        end: last.end,
                        ids: options.swap_remove(best),
                    });
                }
            }
            w += len;
        }
        Ok(replacements)
    }

    fn token_text(&self, id: i32) -> &str {
        self.vocab.get(id as usize).map_or("", String::as_str)
    }

    /// Longest run of words starting at `words[0]` whose text is a heard form.
    fn match_at<'g>(
        groups: &'g ConfusionGroups,
        texts: &[&str],
        words: &[Word],
    ) -> Option<(usize, &'g Vec<String>)> {
        (1..=groups.max_words.min(words.len()))
            .rev()
            .find_map(|len| {
                let key = words[..len]
                    .iter()
                    .enumerate()
                    .map(|(i, word)| {
                        let from = match i {
                            0 => word.core,
                            _ => word.start,
                        };
                        normalize(&texts[from..word.end].concat())
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                groups
                    .by_heard
                    .get(&key)
                    .map(|candidates| (len, candidates))
            })
    }

    /// Advances the replayed decoder state to `emitted` tokens and returns
    /// the joint input there. Matches come in order, so it only moves forward.
    fn replay_to(
        &mut self,
        replay: &mut Option<Replay>,
        ids: &[i32],
        encodings: &ArrayViewD<f32>,
        emitted: usize,
    ) -> Result<(i32, DecoderState), ParakeetError> {
        let mut current = match replay.take() {
            Some(current) => current,
            None => Replay {
                next: 0,
                target: self.blank_idx,
                state: self.create_decoder_state()?,
            },
        };
        let frame = encodings.slice(ndarray::s![0, ..]);
        while current.next < emitted {
            let (_, state) = self
                .decode_steps(&frame, &[(current.target, &current.state)])?
                .pop()
                .ok_or_else(|| ParakeetError::OutputNotFound("decoder state".to_string()))?;
            current = Replay {
                next: current.next + 1,
                target: ids[current.next],
                state,
            };
        }
        let row = (current.target, current.state.clone());
        *replay = Some(current);
        Ok(row)
    }

    /// Viterbi score of each token sequence over `frames` frames from
    /// `window_start`, every sequence starting from `start_row`.
    fn align_scores(
        &mut self,
        encodings: &ArrayViewD<f32>,
        window_start: usize,
        frames: usize,
        start_row: (i32, DecoderState),
        sequences: &[Vec<i32>],
    ) -> Result<Vec<f64>, ParakeetError> {
        // Joint inputs of every prefix: rows[s][u] once u tokens are emitted.
        let mut rows: Vec<Vec<(i32, DecoderState)>> =
            sequences.iter().map(|_| vec![start_row.clone()]).collect();
        let longest = sequences.iter().map(Vec::len).max().unwrap_or(0);
        let frame = encodings.slice(ndarray::s![window_start, ..]);
        for k in 0..longest {
            let growing: Vec<usize> = (0..sequences.len())
                .filter(|&s| sequences[s].len() > k)
                .collect();
            let batch: Vec<(i32, &DecoderState)> = growing
                .iter()
                .map(|&s| (rows[s][k].0, &rows[s][k].1))
                .collect();
            let outputs = self.decode_steps(&frame, &batch)?;
            for (&s, (_, state)) in growing.iter().zip(outputs) {
                rows[s].push((sequences[s][k], state));
            }
        }

        // One batched joint call per frame over every (sequence, prefix).
        let flat: Vec<(usize, usize)> = rows
            .iter()
            .enumerate()
            .flat_map(|(s, prefixes)| (0..prefixes.len()).map(move |u| (s, u)))
            .collect();
        let mut steps: Vec<Vec<Vec<StepScores>>> = sequences.iter().map(|_| Vec::new()).collect();
        for t in 0..frames {
            let frame = encodings.slice(ndarray::s![window_start + t, ..]);
            let batch: Vec<(i32, &DecoderState)> = flat
                .iter()
                .map(|&(s, u)| (rows[s][u].0, &rows[s][u].1))
                .collect();
            let outputs = self.decode_steps(&frame, &batch)?;
            for steps in steps.iter_mut() {
                steps.push(Vec::new());
            }
            for (&(s, u), (row, _)) in flat.iter().zip(outputs) {
                let (logits, durations) = Self::split_joint_row(&row, self.vocab_size);
                let z = log_sum_exp(logits);
                let token = sequences[s]
                    .get(u)
                    .map_or(f64::NEG_INFINITY, |&id| logits[id as usize] as f64 - z);
                let durations = match durations.is_empty() {
                    true => vec![0.0],
                    false => {
                        let dz = log_sum_exp(durations);
                        durations.iter().map(|&d| d as f64 - dz).collect()
                    }
                };
                steps[s][t].push(StepScores {
                    token,
                    blank: logits[self.blank_idx as usize] as f64 - z,
                    durations,
                });
            }
        }
        Ok(steps
            .iter()
            .zip(sequences)
            .map(|(steps, sequence)| viterbi(steps, frames, sequence.len()))
            .collect())
    }
}

/// Splices the replacements in, back to front so earlier indices stay valid.
/// New tokens get evenly spread timestamps over the replaced span and the
/// mean confidence of the tokens they replace.
fn apply_replacements(
    sequence: DecodedSequence,
    replacements: Vec<Replacement>,
) -> DecodedSequence {
    let (mut ids, mut timestamps, mut probs) = sequence;
    for r in replacements.into_iter().rev() {
        let first = timestamps[r.start];
        let last = timestamps[r.end - 1];
        let n = r.ids.len();
        let new_timestamps: Vec<usize> = (0..n)
            .map(|i| first + (last - first) * i / n.max(1))
            .collect();
        let old = probs.get(r.start..r.end).unwrap_or_default();
        let mean = old.iter().sum::<f32>() / old.len().max(1) as f32;
        ids.splice(r.start..r.end, r.ids);
        timestamps.splice(r.start..r.end, new_timestamps);
        if probs.len() >= r.end {
            probs.splice(r.start..r.end, std::iter::repeat_n(mean, n));
        }
    }
    (ids, timestamps, probs)
}

impl ParakeetEngine {
    /// Arms the confusion groups (`confusion_groups.jsonl` content); an empty
    /// string turns the second pass off.
    pub fn set_confusion_groups(&mut self, raw: &str) {
        if let Some(model) = self.model.as_mut() {
            model.set_confusion_groups(raw);
        }
    }

    /// Benchmarks only: overrides `DEFAULT_MARGIN` of the armed groups.
    #[cfg(test)]
    pub fn set_confusion_margin(&mut self, margin: f64) {
        if let Some(groups) = self.model.as_mut().and_then(|m| m.confusion.as_mut()) {
            groups.margin = margin;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_keeps_only_ambiguous_groups() {
        let raw = concat!(
            r#"{"heard": "singulaire", "candidates": [{"text": "Singulair", "count": 11}, {"text": "cingulaire", "count": 11}], "heard_correct_count": 3}"#,
            "\n",
            r#"{"heard": "abdominale", "candidates": [{"text": "abdominales", "count": 630}, {"text": "abdominal", "count": 9}], "heard_correct_count": 8194}"#,
            "\n",
            r#"{"heard": "torvastatine", "candidates": [{"text": "atorvastatine", "count": 99}, {"text": "x", "count": 1}], "heard_correct_count": 0}"#,
            "\nnot json\n",
        );
        let groups = ConfusionGroups::parse(raw);
        assert_eq!(groups.len(), 1);
        assert_eq!(
            groups.by_heard["singulaire"],
            vec!["Singulair".to_string(), "cingulaire".to_string()]
        );
    }

    #[test]
    fn split_words_skips_elision_and_trailing_punctuation() {
        let tokens = [" sous", " l", "'", "épi", "staxie", ",", " et"];
        assert_eq!(
            split_words(&tokens),
            vec![
                Word {
                    start: 0,
                    core: 0,
                    end: 1
                },
                Word {
                    start: 1,
                    core: 3,
                    end: 5
                },
                Word {
                    start: 6,
                    core: 6,
                    end: 7
                },
            ]
        );
    }

    #[test]
    fn match_at_prefers_the_longest_heard_form() {
        let raw = concat!(
            r#"{"heard": "rituxima bancourt", "candidates": [{"text": "rituximab en cours", "count": 5}, {"text": "rituximab encore", "count": 2}], "heard_correct_count": 0}"#,
            "\n",
            r#"{"heard": "rituxima", "candidates": [{"text": "rituximab", "count": 5}, {"text": "rituxan", "count": 2}], "heard_correct_count": 0}"#,
        );
        let groups = ConfusionGroups::parse(raw);
        let tokens = [" Rituxi", "ma", " ban", "court", "."];
        let words = split_words(&tokens);
        let (len, candidates) = ParakeetModel::match_at(&groups, &tokens, &words).unwrap();
        assert_eq!(len, 2);
        assert_eq!(candidates[0], "rituximab en cours");
    }

    #[test]
    fn viterbi_prefers_the_sequence_the_frames_support() {
        // 2 frames, durations {0, 1}. Sequence "a": emit a then blank.
        let step = |token: f64, blank: f64| StepScores {
            token,
            blank,
            durations: vec![(0.5f64).ln(), (0.5f64).ln()],
        };
        let likely = vec![vec![step(-0.1, -3.0), step(f64::NEG_INFINITY, -0.1)]; 2];
        let unlikely = vec![vec![step(-4.0, -3.0), step(f64::NEG_INFINITY, -0.1)]; 2];
        assert!(viterbi(&likely, 2, 1) > viterbi(&unlikely, 2, 1));
        assert_eq!(viterbi(&likely, 2, 2), f64::NEG_INFINITY);
    }

    #[test]
    fn match_case_keeps_a_leading_capital() {
        assert_eq!(match_case(" Communiquant", "communicant"), "Communicant");
        assert_eq!(match_case(" singulaire", "Singulair"), "Singulair");
        assert_eq!(match_case(" douane", "cingulaire"), "cingulaire");
    }

    #[test]
    fn apply_replacements_splices_tokens_back_to_front() {
        let sequence = (
            vec![1, 2, 3, 4],
            vec![0, 4, 8, 12],
            vec![0.9, 0.5, 0.7, 0.9],
        );
        let replacements = vec![Replacement {
            start: 1,
            end: 3,
            ids: vec![7, 8, 9],
        }];
        let (ids, timestamps, probs) = apply_replacements(sequence, replacements);
        assert_eq!(ids, vec![1, 7, 8, 9, 4]);
        assert_eq!(timestamps, vec![0, 4, 5, 6, 12]);
        assert_eq!(probs.len(), 5);
        assert!((probs[1] - 0.6).abs() < 1e-6);
    }

    #[test]
    fn candidates_tokenize_as_a_word_start() {
        let Some(tokenizer) = super::super::helpers::bundled_tokenizer() else {
            return;
        };
        let ids = super::super::helpers::tokenize_word_to_ids(&tokenizer, "rituximab").unwrap();
        let first = tokenizer.id_to_token(ids[0] as u32).unwrap();
        assert!(first.starts_with('▁'), "{first}");
    }
}
