//! MAES beam search for the TDT decoder, ported from ParakeetWeb's
//! `_decodeBeam` (written with Claude Code). Only final transcriptions use it:
//! the streaming preview and the wake word stay greedy, which is several times
//! cheaper.
//!
//! Every hypothesis carries its own decoder state, last token, dictionary
//! boost state and phrase trie active set, so a phrase engaged on one
//! hypothesis never leaks into another. Scores combine like ParakeetWeb's:
//! a step adds the TRUE token log-probability, plus the boost bonus of that
//! token (boosted logit minus raw logit, never subtracted again), plus the
//! duration log-probability. The confidences pushed with each token come
//! from the raw logits, like the greedy decoders.
//!
//! Knobs are ParakeetWeb's benchmarked ones (beam 5 + boost: 7.80 WER vs 8.26
//! greedy + boost on 400 French medical clips).

use std::collections::HashMap;
use std::rc::Rc;

use ndarray::ArrayViewD;

use super::boost_tree::BoostTree;
use super::engine::{argmax_index, DecodedSequence};
use super::phrase_boost::{BoostingTrie, NodeId};
use super::types::{DecoderState, ParakeetError, ParakeetModel};

/// Beam width of final transcriptions. 1 means greedy.
pub const DEFAULT_BEAM_WIDTH: usize = 5;
/// Symbols a hypothesis may emit on one frame before it is forced forward.
const MAES_NUM_STEPS: usize = 2;
/// Extra candidates per expansion beyond the beam width.
const MAES_EXPANSION_BETA: usize = 2;
/// Non-blank candidates scoring this far below the step's best are dropped.
const MAES_EXPANSION_GAMMA: f64 = 2.3;
/// Longest extension (in tokens) prefix search recombines.
const MAES_PREFIX_ALPHA: usize = 1;

/// Duration logits of a joint with no duration head (plain RNN-T): a single
/// duration 0 that every step takes with certainty.
const NO_DURATION: [f32; 1] = [0.0];

pub(super) fn log_sum_exp(values: &[f32]) -> f64 {
    let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max) as f64;
    if !max.is_finite() {
        return max;
    }
    max + values
        .iter()
        .map(|&v| (v as f64 - max).exp())
        .sum::<f64>()
        .ln()
}

fn log_add_exp(a: f64, b: f64) -> f64 {
    if a == f64::NEG_INFINITY {
        return b;
    }
    if b == f64::NEG_INFINITY {
        return a;
    }
    let max = a.max(b);
    max + ((a - max).exp() + (b - max).exp()).ln()
}

fn value_at(values: &[f32], idx: usize) -> f64 {
    values.get(idx).copied().unwrap_or(f32::NEG_INFINITY) as f64
}

/// Indices of the `k` largest values, best first.
fn top_k_indices(values: &[f32], k: usize) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..values.len()).collect();
    let desc = |a: &usize, b: &usize| {
        values[*b]
            .partial_cmp(&values[*a])
            .unwrap_or(std::cmp::Ordering::Equal)
    };
    let k = k.min(idx.len());
    if k == 0 {
        return Vec::new();
    }
    if k < idx.len() {
        idx.select_nth_unstable_by(k - 1, desc);
        idx.truncate(k);
    }
    idx.sort_by(desc);
    idx
}

/// One way to extend a hypothesis: emit `token` (or blank) and advance `step`
/// frames. `delta` is the score it adds, `prob` the raw confidence of `token`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Candidate {
    token: i32,
    step: usize,
    delta: f64,
    prob: f32,
}

/// ParakeetWeb's `_expandHyp` scoring. The `expand_k` best tokens by BOOSTED
/// logit (plus blank) are crossed with every duration, each pair scoring the
/// true token log-probability plus its boost bonus plus the duration
/// log-probability; the `expand_k` best pairs are kept (plus the best blank
/// pair if none made it), then non-blank pairs more than `gamma` below the
/// best are dropped. Blank with duration 0 still advances one frame.
fn expand_candidates(
    logits: &[f32],
    boosted: &[f32],
    durations: &[f32],
    blank_idx: i32,
    expand_k: usize,
    gamma: f64,
) -> Vec<Candidate> {
    let blank = blank_idx as usize;
    let mut tokens = top_k_indices(boosted, expand_k);
    if !tokens.contains(&blank) {
        tokens.push(blank);
    }
    let log_z = log_sum_exp(logits);
    let dur_z = log_sum_exp(durations);
    let min_step = usize::from(durations.len() > 1);

    let mut pairs: Vec<Candidate> = Vec::with_capacity(tokens.len() * durations.len());
    let mut max_total = f64::NEG_INFINITY;
    for &token in &tokens {
        let logit = value_at(logits, token);
        let logp = (logit - log_z) + (value_at(boosted, token) - logit);
        let prob = (logit - log_z).exp() as f32;
        for (d, &dur) in durations.iter().enumerate() {
            let delta = logp + (dur as f64 - dur_z);
            if !delta.is_finite() {
                continue;
            }
            let step = match (token == blank, d) {
                (true, 0) => min_step,
                _ => d,
            };
            pairs.push(Candidate {
                token: token as i32,
                step,
                delta,
                prob,
            });
            max_total = max_total.max(delta);
        }
    }
    pairs.sort_by(|a, b| {
        b.delta
            .partial_cmp(&a.delta)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut kept: Vec<Candidate> = pairs.iter().take(expand_k).copied().collect();
    if !kept.iter().any(|c| c.token == blank_idx) {
        if let Some(best_blank) = pairs.iter().find(|c| c.token == blank_idx) {
            kept.push(*best_blank);
        }
    }
    let threshold = max_total - gamma;
    kept.retain(|c| c.token == blank_idx || c.delta >= threshold);
    kept
}

/// ParakeetWeb's `_advanceDecision`: next (frame, symbols emitted on it).
fn advance(t: usize, emitted_at_frame: usize, is_blank: bool, step: usize) -> (usize, usize) {
    match step {
        0 if !is_blank && emitted_at_frame + 1 < MAES_NUM_STEPS => (t, emitted_at_frame + 1),
        0 => (t + 1, 0),
        _ => (t + step, 0),
    }
}

#[derive(Debug, Clone)]
struct Hypothesis {
    tokens: Vec<i32>,
    timestamps: Vec<usize>,
    probs: Vec<f32>,
    /// Decoder state after feeding the last emitted token.
    state: Rc<DecoderState>,
    /// Frame this hypothesis is expanded on next.
    t: usize,
    emitted_at_frame: usize,
    score: f64,
    boost_state: usize,
    active: Vec<NodeId>,
}

impl Hypothesis {
    fn last_token(&self, blank_idx: i32) -> i32 {
        self.tokens.last().copied().unwrap_or(blank_idx)
    }

    fn length_normalized_score(&self) -> f64 {
        self.score / self.tokens.len().max(1) as f64
    }

    /// The hypothesis extended by `cand`. An emission takes the state of the
    /// step that scored it (`new_state`) and advances its own boost states; a
    /// blank keeps everything of the parent but the frame and the score.
    fn child(
        &self,
        cand: &Candidate,
        new_state: &Rc<DecoderState>,
        blank_idx: i32,
        tree: Option<&BoostTree>,
        trie: Option<&BoostingTrie>,
    ) -> Self {
        let is_blank = cand.token == blank_idx;
        let (t, emitted_at_frame) = advance(self.t, self.emitted_at_frame, is_blank, cand.step);
        let mut child = Self {
            t,
            emitted_at_frame,
            score: self.score + cand.delta,
            ..self.clone()
        };
        if !is_blank {
            child.tokens.push(cand.token);
            child.timestamps.push(self.t);
            child.probs.push(cand.prob);
            child.state = Rc::clone(new_state);
            if let Some(tree) = tree {
                child.boost_state = tree.advance(self.boost_state, cand.token);
            }
            if let Some(trie) = trie {
                child.active = trie.advance(&self.active, cand.token);
            }
        }
        child
    }
}

/// Collapse hypotheses with the same tokens on the same frame, reached by
/// different alignments, into the best-scoring one, carrying the log-sum-exp
/// of the group's scores. First-seen order is kept.
fn merge_hypotheses(hyps: Vec<Hypothesis>) -> Vec<Hypothesis> {
    let mut index: HashMap<(Vec<i32>, usize), usize> = HashMap::new();
    let mut merged: Vec<Hypothesis> = Vec::with_capacity(hyps.len());
    for hyp in hyps {
        let key = (hyp.tokens.clone(), hyp.t);
        match index.get(&key) {
            None => {
                index.insert(key, merged.len());
                merged.push(hyp);
            }
            Some(&i) => {
                let rep = &mut merged[i];
                let score = log_add_exp(rep.score, hyp.score);
                if hyp.score > rep.score {
                    *rep = hyp;
                }
                rep.score = score;
            }
        }
    }
    merged
}

fn sort_by_score(hyps: &mut [Hypothesis]) {
    hyps.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// The `width` best hypotheses by RAW score: only the final pick is length
/// normalized, like NeMo and ParakeetWeb.
fn prune_beam(mut hyps: Vec<Hypothesis>, width: usize) -> Vec<Hypothesis> {
    sort_by_score(&mut hyps);
    hyps.truncate(width);
    hyps
}

/// Keep `finished` as the best final hypothesis if its length normalized
/// score is strictly higher.
fn keep_best(best: &mut Option<Hypothesis>, finished: Hypothesis) {
    let better = match best {
        Some(b) => finished.length_normalized_score() > b.length_normalized_score(),
        None => true,
    };
    if better {
        *best = Some(finished);
    }
}

/// Output of one joint row.
struct JointOut {
    logits: Vec<f32>,
    durations: Vec<f32>,
    state: Rc<DecoderState>,
}

/// A prefix search extension of `beam[short]` toward `beam[long]`.
struct PrefixJob {
    long: usize,
    short: usize,
    state: Rc<DecoderState>,
    prev: i32,
    ext_logp: f64,
}

impl ParakeetModel {
    /// One batched joint step on `frame`: each row is (last token, decoder
    /// state), each output (vocab logits, duration logits, new decoder state).
    /// A single ONNX run whatever the number of rows.
    fn joint_steps(
        &mut self,
        encodings: &ArrayViewD<f32>,
        frame: usize,
        rows: &[(i32, &DecoderState)],
    ) -> Result<Vec<JointOut>, ParakeetError> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let encoder_frame = encodings.slice(ndarray::s![frame, ..]);
        let vocab_size = self.vocab_size;
        Ok(self
            .decode_steps(&encoder_frame, rows)?
            .into_iter()
            .map(|(row, state)| {
                let (logits, durations) = Self::split_joint_row(&row, vocab_size);
                let durations = match durations.is_empty() {
                    true => &NO_DURATION[..],
                    false => durations,
                };
                JointOut {
                    logits: logits.to_vec(),
                    durations: durations.to_vec(),
                    state: Rc::new(state),
                }
            })
            .collect())
    }

    /// MAES beam decode of `width` hypotheses, boosted by `tree` and `trie`
    /// when given. Mirrors ParakeetWeb's `_decodeBeam` step for step,
    /// including its batching: every hypothesis of a MAES step shares one
    /// joint run, so do the blank closures and each prefix search step.
    pub(super) fn decode_sequence_beam(
        &mut self,
        encodings: &ArrayViewD<f32>,
        encodings_len: usize,
        width: usize,
        tree: Option<&BoostTree>,
        trie: Option<&BoostingTrie>,
    ) -> Result<DecodedSequence, ParakeetError> {
        let blank = self.blank_idx;
        let expand_k = width + MAES_EXPANSION_BETA;
        let mut kept = vec![Hypothesis {
            tokens: Vec::new(),
            timestamps: Vec::new(),
            probs: Vec::new(),
            state: Rc::new(self.create_decoder_state()?),
            t: 0,
            emitted_at_frame: 0,
            score: 0.0,
            boost_state: tree.map_or(0, BoostTree::root),
            active: trie.map_or_else(Vec::new, BoostingTrie::start),
        }];
        let mut best: Option<Hypothesis> = None;

        for frame in 0..encodings_len {
            if kept.is_empty() {
                break;
            }
            let (mut current, futures): (Vec<_>, Vec<_>) =
                kept.into_iter().partition(|h| h.t == frame);
            if current.is_empty() {
                kept = futures;
                continue;
            }
            self.prefix_search(&mut current, encodings, frame)?;

            let mut produced = Vec::new();
            let mut working = current;
            for _ in 0..MAES_NUM_STEPS {
                if working.is_empty() {
                    break;
                }
                let rows: Vec<_> = working
                    .iter()
                    .map(|h| (h.last_token(blank), h.state.as_ref()))
                    .collect();
                let outs = self.joint_steps(encodings, frame, &rows)?;
                // (child, whether it takes a blank closure), in expansion order
                let mut children = Vec::new();
                for (hyp, out) in working.iter().zip(&outs) {
                    let bias = tree.map_or_else(Vec::new, |tree| tree.bias(hyp.boost_state));
                    let boosted = self.boosted_logits(&out.logits, &bias, trie, &hyp.active);
                    let cands = expand_candidates(
                        &out.logits,
                        &boosted,
                        &out.durations,
                        blank,
                        expand_k,
                        MAES_EXPANSION_GAMMA,
                    );
                    for cand in &cands {
                        let closes = cand.token != blank
                            && cand.step == 0
                            && hyp.emitted_at_frame + 1 >= MAES_NUM_STEPS;
                        children.push((hyp.child(cand, &out.state, blank, tree, trie), closes));
                    }
                }
                self.blank_closures(&mut children, encodings, frame)?;
                let mut stayed = Vec::new();
                for (child, _) in children {
                    match child.t {
                        t if t >= encodings_len => keep_best(&mut best, child),
                        t if t > frame => produced.push(child),
                        _ => stayed.push(child),
                    }
                }
                working = prune_beam(stayed, width);
            }

            let mut next = futures;
            next.extend(produced);
            kept = prune_beam(merge_hypotheses(next), width);
        }

        Ok(best.map_or_else(Default::default, |h| (h.tokens, h.timestamps, h.probs)))
    }

    /// A hypothesis out of emissions on this frame takes a forced blank with
    /// its most likely non-zero duration (scored, state discarded), like
    /// ParakeetWeb's last-MAES-step blank closure. Applies to the children
    /// flagged `true`, all in one joint run.
    fn blank_closures(
        &mut self,
        children: &mut [(Hypothesis, bool)],
        encodings: &ArrayViewD<f32>,
        frame: usize,
    ) -> Result<(), ParakeetError> {
        let blank = self.blank_idx;
        let closing: Vec<usize> = (0..children.len()).filter(|&i| children[i].1).collect();
        let rows: Vec<_> = closing
            .iter()
            .map(|&i| {
                (
                    children[i].0.last_token(blank),
                    children[i].0.state.as_ref(),
                )
            })
            .collect();
        let outs = self.joint_steps(encodings, frame, &rows)?;
        for (&i, out) in closing.iter().zip(&outs) {
            let step = match argmax_index(&out.durations) {
                0 if out.durations.len() > 1 => 1,
                best => best,
            };
            let child = &mut children[i].0;
            child.score += value_at(&out.logits, blank as usize) - log_sum_exp(&out.logits)
                + (value_at(&out.durations, step) - log_sum_exp(&out.durations));
            child.t = frame + step;
        }
        Ok(())
    }

    /// ParakeetWeb's `_prefixSearch`: when a hypothesis is a strict prefix of
    /// another on the same frame, shorter by at most MAES_PREFIX_ALPHA tokens,
    /// the longer one also gets the probability of being reached through the
    /// shorter one (unboosted extension, duration 0).
    fn prefix_search(
        &mut self,
        beam: &mut [Hypothesis],
        encodings: &ArrayViewD<f32>,
        frame: usize,
    ) -> Result<(), ParakeetError> {
        if beam.len() < 2 {
            return Ok(());
        }
        let mut order: Vec<usize> = (0..beam.len()).collect();
        order.sort_by(|&a, &b| beam[b].tokens.len().cmp(&beam[a].tokens.len()));
        let mut jobs: Vec<PrefixJob> = Vec::new();
        for (i, &long) in order.iter().enumerate() {
            for &short in &order[i + 1..] {
                let (long_h, short_h) = (&beam[long], &beam[short]);
                let gap = long_h.tokens.len() - short_h.tokens.len();
                if !(1..=MAES_PREFIX_ALPHA).contains(&gap)
                    || !long_h.tokens.starts_with(&short_h.tokens)
                {
                    continue;
                }
                jobs.push(PrefixJob {
                    long,
                    short,
                    state: Rc::clone(&short_h.state),
                    prev: short_h.last_token(self.blank_idx),
                    ext_logp: 0.0,
                });
            }
        }
        // Step k extends every job by its k-th missing token, all jobs in one
        // joint run (jobs only differ in length when MAES_PREFIX_ALPHA > 1).
        for k in 0..MAES_PREFIX_ALPHA {
            let live: Vec<usize> = (0..jobs.len())
                .filter(|&j| beam[jobs[j].short].tokens.len() + k < beam[jobs[j].long].tokens.len())
                .collect();
            let rows: Vec<_> = live
                .iter()
                .map(|&j| (jobs[j].prev, jobs[j].state.as_ref()))
                .collect();
            let outs = self.joint_steps(encodings, frame, &rows)?;
            for (&j, out) in live.iter().zip(outs) {
                let job = &mut jobs[j];
                let token = beam[job.long].tokens[beam[job.short].tokens.len() + k];
                job.ext_logp += value_at(&out.logits, token as usize) - log_sum_exp(&out.logits)
                    + (value_at(&out.durations, 0) - log_sum_exp(&out.durations));
                job.state = out.state;
                job.prev = token;
            }
        }
        for job in jobs {
            beam[job.long].score =
                log_add_exp(beam[job.long].score, beam[job.short].score + job.ext_logp);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::phrase_boost::EncodedPhrase;
    use ndarray::Array3;

    fn hyp(tokens: &[i32], t: usize, score: f64) -> Hypothesis {
        Hypothesis {
            tokens: tokens.to_vec(),
            timestamps: vec![0; tokens.len()],
            probs: vec![1.0; tokens.len()],
            state: Rc::new((Array3::zeros((1, 1, 1)), Array3::zeros((1, 1, 1)))),
            t,
            emitted_at_frame: 0,
            score,
            boost_state: 0,
            active: Vec::new(),
        }
    }

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn log_add_exp_sums_probabilities() {
        assert!(approx(log_add_exp(0.5f64.ln(), 0.25f64.ln()), 0.75f64.ln()));
        assert_eq!(log_add_exp(f64::NEG_INFINITY, -1.0), -1.0);
    }

    #[test]
    fn advance_follows_the_tdt_rules() {
        // Duration > 0 jumps; blank or the per-frame cap moves one frame.
        assert_eq!(advance(4, 1, false, 3), (7, 0));
        assert_eq!(advance(4, 0, true, 0), (5, 0));
        assert_eq!(advance(4, 0, false, 0), (4, 1));
        assert_eq!(advance(4, 1, false, 0), (5, 0));
    }

    #[test]
    fn prune_keeps_the_best_raw_scores() {
        // [7, 8, 9] has the best normalized score (-1) but the worst raw one.
        let hyps = vec![
            hyp(&[], 1, -1.0),
            hyp(&[7, 8, 9], 1, -3.0),
            hyp(&[7], 1, -2.0),
        ];
        let kept = prune_beam(hyps, 2);
        let tokens: Vec<_> = kept.iter().map(|h| h.tokens.clone()).collect();
        assert_eq!(tokens, vec![vec![], vec![7]]);
    }

    #[test]
    fn final_pick_is_length_normalized() {
        let mut best = None;
        keep_best(&mut best, hyp(&[7], 9, -2.0));
        // Raw -3 is worse, but -1 per token beats -2.
        keep_best(&mut best, hyp(&[7, 8, 9], 9, -3.0));
        keep_best(&mut best, hyp(&[5, 6, 7], 9, -3.0)); // tie: first kept
        assert_eq!(best.map(|h| h.tokens), Some(vec![7, 8, 9]));
    }

    #[test]
    fn duplicates_merge_with_log_add_exp() {
        let merged = merge_hypotheses(vec![
            hyp(&[1, 2], 3, 0.25f64.ln()),
            hyp(&[1], 3, -5.0),
            hyp(&[1, 2], 3, 0.5f64.ln()),
            hyp(&[1, 2], 4, -1.0), // same tokens, other frame: kept apart
        ]);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].tokens, vec![1, 2]);
        assert!(approx(merged[0].score, 0.75f64.ln()));
        assert_eq!(merged[1].tokens, vec![1]);
        assert_eq!(merged[2].t, 4);
    }

    #[test]
    fn merge_keeps_the_better_alignment_in_first_seen_place() {
        let mut better = hyp(&[1], 3, -0.1);
        better.timestamps = vec![2];
        let merged = merge_hypotheses(vec![hyp(&[1], 3, -2.0), hyp(&[9], 3, -1.0), better]);
        assert_eq!(merged[0].timestamps, vec![2]);
        assert_eq!(merged[1].tokens, vec![9]);
    }

    #[test]
    fn expansion_ranks_by_boosted_logits_and_keeps_the_bonus() {
        // Vocab [a, b, c, blank], one duration. b is boosted by 3 past a.
        let logits = [2.0, 0.0, -1.0, 1.0];
        let boosted = [2.0, 3.0, -1.0, 1.0];
        let cands = expand_candidates(&logits, &boosted, &[0.0], 3, 1, 10.0);
        let log_z = log_sum_exp(&logits);
        // Top-1 by boosted logit is b, blank is always added.
        assert_eq!(cands.len(), 2);
        assert_eq!(cands[0].token, 1);
        assert!(approx(cands[0].delta, (0.0 - log_z) + 3.0));
        // Confidence comes from the raw logits.
        assert!((cands[0].prob as f64 - (0.0 - log_z).exp()).abs() < 1e-6);
        assert_eq!(cands[1].token, 3);
    }

    #[test]
    fn expansion_crosses_durations_and_prunes_by_gamma() {
        let logits = [3.0, 0.0, -9.0, 0.0];
        let durations = [0.0, 0.0];
        let cands = expand_candidates(&logits, &logits, &durations, 3, 4, 2.3);
        // Token 2 is far below the best: dropped. Blank with duration 0 still
        // advances one frame.
        assert!(cands.iter().all(|c| c.token != 2));
        assert!(cands.iter().any(|c| c.token == 0 && c.step == 0));
        assert!(cands.iter().any(|c| c.token == 0 && c.step == 1));
        assert!(cands.iter().filter(|c| c.token == 3).all(|c| c.step == 1));
        assert!(cands.iter().any(|c| c.token == 3));
    }

    /// Beam vs greedy on a real clip (the bundled French smoke sample unless
    /// PLUMED_BEAM_WAV names another 16 kHz mono wav), with the bundled model
    /// unless PLUMED_MODEL_DIR names another:
    /// `cargo test --release --lib engine::beam -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn beam_vs_greedy_on_a_sample() {
        use crate::audio::pipeline::pad_with_silence;
        use crate::engine::types::ParakeetEngine;
        use std::path::Path;

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let model_dir = std::env::var("PLUMED_MODEL_DIR").map_or_else(
            |_| root.join("resources/parakeet-tdt-0.6b-v3-int8"),
            Into::into,
        );
        let wav = std::env::var("PLUMED_BEAM_WAV")
            .map_or_else(|_| root.join(".github/smoke/smoke-fr.wav"), Into::into);
        let mut engine =
            ParakeetEngine::load_int8(&model_dir, Some(root.join("resources/tokenizer.json")))
                .expect("model");
        let model = engine.model.as_mut().expect("loaded");
        let samples: Vec<f32> = hound::WavReader::open(&wav)
            .expect("wav")
            .samples::<i16>()
            .map(|s| s.expect("sample") as f32 / 32768.0)
            .collect();
        let padded = pad_with_silence(&samples, 16000);
        for width in [1, DEFAULT_BEAM_WIDTH] {
            let start = std::time::Instant::now();
            let result = model
                .transcribe_samples(padded.clone(), width)
                .expect("transcribe");
            println!("width {width} in {:?}: {}", start.elapsed(), result.text);
            assert!(!result.text.trim().is_empty());
            assert_eq!(result.tokens.len(), result.probs.len());
            assert!(result.timestamps.windows(2).all(|w| w[0] <= w[1]));
        }
    }

    #[test]
    fn each_hypothesis_keeps_its_own_boost_state() {
        let trie = BoostingTrie::new(&[EncodedPhrase {
            ids: vec![1, 2],
            weight: 1.0,
            minp: 0.1,
        }]);
        let tree = BoostTree::new(&[vec![1, 2]]);
        let mut parent = hyp(&[], 0, 0.0);
        parent.active = trie.start();
        parent.boost_state = tree.root();
        let state = Rc::clone(&parent.state);
        let cand = |token| Candidate {
            token,
            step: 1,
            delta: 0.0,
            prob: 1.0,
        };
        let engaged = parent.child(&cand(1), &state, 9, Some(&tree), Some(&trie));
        let other = parent.child(&cand(5), &state, 9, Some(&tree), Some(&trie));
        let blank = engaged.child(&cand(9), &state, 9, Some(&tree), Some(&trie));
        // Only the hypothesis that emitted the phrase start is inside it.
        assert_ne!(engaged.boost_state, tree.root());
        assert_ne!(engaged.active, trie.start());
        assert_eq!(other.boost_state, tree.root());
        assert_eq!(other.active, trie.start());
        assert_eq!(parent.active, trie.start());
        // A blank inherits its parent's boost state untouched.
        assert_eq!(blank.boost_state, engaged.boost_state);
        assert_eq!(blank.active, engaged.active);
        assert_eq!(blank.tokens, vec![1]);
    }
}
