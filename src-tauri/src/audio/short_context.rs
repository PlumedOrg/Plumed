//! Plumed fork: short dictations (often a single corrected word) are hard to
//! recognize in isolation. They are transcribed again behind the last seconds of
//! the previous dictation, then only the words after the gap are kept.
//!
//! The context audio lives in RAM only, is never written to disk, and is
//! dropped after CONTEXT_EXPIRY without a new dictation.

use crate::audio::pipeline::{pad_with_silence, INFERENCE_SILENCE_PAD_SECS};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};

const SAMPLE_RATE: usize = 16000;
/// Seconds of previous speech prepended to a short dictation.
const CONTEXT_SECS: usize = 10;
/// Silence between the context and the short dictation.
const GAP_SECS: f32 = 2.0;
/// Silence inserted between two remembered dictations so their words do not merge.
const REMEMBER_GAP_SECS: f32 = 0.25;
/// A dictation shorter than this is always considered short.
const SHORT_AUDIO_SECS: usize = 2;
/// A dictation transcribed to at most this many words is considered short.
const SHORT_MAX_WORDS: usize = 3;
pub const CONTEXT_EXPIRY: Duration = Duration::from_secs(5 * 60);
const JANITOR_INTERVAL: Duration = Duration::from_secs(30);

/// Whether a 16 kHz dictation and its first-pass text deserve a context pass.
pub fn is_short(samples_16k: usize, text: &str) -> bool {
    samples_16k < SHORT_AUDIO_SECS * SAMPLE_RATE
        || text.split_whitespace().count() <= SHORT_MAX_WORDS
}

/// Builds the padded model input `context + gap + speech` and returns it with the
/// time (in seconds) after which decoded words belong to `speech`: the middle of the gap.
pub fn build_input(context: &[f32], speech: &[f32]) -> (Vec<f32>, f32) {
    let gap = vec![0.0; (GAP_SECS * SAMPLE_RATE as f32) as usize];
    let mut joined = Vec::with_capacity(context.len() + gap.len() + speech.len());
    joined.extend_from_slice(context);
    joined.extend_from_slice(&gap);
    joined.extend_from_slice(speech);
    let cut_secs =
        INFERENCE_SILENCE_PAD_SECS + context.len() as f32 / SAMPLE_RATE as f32 + GAP_SECS / 2.0;
    (pad_with_silence(&joined, SAMPLE_RATE as u32), cut_secs)
}

/// Tail of the most recent dictations, 16 kHz mono, in RAM only.
#[derive(Default)]
pub struct ContextAudio {
    samples: Vec<f32>,
    updated: Option<Instant>,
    janitor_running: bool,
}

impl ContextAudio {
    /// Drops the audio if it is older than CONTEXT_EXPIRY. Returns true when empty.
    fn expire(&mut self, now: Instant) -> bool {
        let stale = self
            .updated
            .is_none_or(|t| now.saturating_duration_since(t) >= CONTEXT_EXPIRY);
        if stale {
            self.clear();
        }
        self.samples.is_empty()
    }

    pub fn clear(&mut self) {
        self.samples = Vec::new();
        self.updated = None;
    }

    /// The remembered context, if any and not expired.
    pub fn get(&mut self, now: Instant) -> Option<Vec<f32>> {
        match self.expire(now) {
            true => None,
            false => Some(self.samples.clone()),
        }
    }

    /// Appends a dictation and keeps only the last CONTEXT_SECS.
    pub fn remember(&mut self, speech: &[f32], now: Instant) {
        if speech.is_empty() {
            return;
        }
        if !self.expire(now) {
            let gap = (REMEMBER_GAP_SECS * SAMPLE_RATE as f32) as usize;
            self.samples.extend(std::iter::repeat_n(0.0, gap));
        }
        self.samples.extend_from_slice(speech);
        let excess = self
            .samples
            .len()
            .saturating_sub(CONTEXT_SECS * SAMPLE_RATE);
        self.samples.drain(..excess);
        self.updated = Some(now);
    }
}

/// Remembers a dictation and makes sure a janitor thread frees it once expired,
/// even if no further dictation ever reads it.
pub fn remember(store: &Arc<Mutex<ContextAudio>>, speech: &[f32]) {
    let spawn = {
        let mut ctx = store.lock();
        ctx.remember(speech, Instant::now());
        !std::mem::replace(&mut ctx.janitor_running, true)
    };
    if spawn {
        let store = store.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(JANITOR_INTERVAL);
            let mut ctx = store.lock();
            if ctx.expire(Instant::now()) {
                ctx.janitor_running = false;
                break;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: usize = SAMPLE_RATE;

    #[test]
    fn short_by_duration_or_word_count() {
        assert!(is_short(SR, "une phrase assez longue pour ne pas compter"));
        assert!(is_short(5 * SR, "Dépakote 500 mg"));
        assert!(!is_short(5 * SR, "on augmente le Dépakote ce soir"));
        assert!(!is_short(2 * SR, "on augmente le Dépakote ce soir"));
    }

    #[test]
    fn input_puts_the_cut_in_the_middle_of_the_gap() {
        let context = vec![0.1; 10 * SR];
        let speech = vec![0.2; SR];
        let (input, cut) = build_input(&context, &speech);
        let pad = (INFERENCE_SILENCE_PAD_SECS * SR as f32) as usize;
        assert_eq!(input.len(), pad + 10 * SR + 2 * SR + SR + pad);
        assert!((cut - (0.25 + 10.0 + 1.0)).abs() < 1e-6);
        // The cut falls in silence: context before it, speech after it
        let cut_idx = (cut * SR as f32) as usize;
        assert_eq!(input[cut_idx], 0.0);
        assert_eq!(input[pad + 10 * SR - 1], 0.1);
        assert_eq!(input[pad + 12 * SR], 0.2);
    }

    #[test]
    fn keeps_only_the_last_ten_seconds() {
        let mut ctx = ContextAudio::default();
        let now = Instant::now();
        ctx.remember(&vec![0.1; 8 * SR], now);
        ctx.remember(&vec![0.2; 4 * SR], now);
        let kept = ctx.get(now).unwrap();
        assert_eq!(kept.len(), 10 * SR);
        assert_eq!(*kept.last().unwrap(), 0.2);
        // 8 s + 0.25 s gap + 4 s = 12.25 s, so the first 2.25 s are dropped
        assert_eq!(kept[0], 0.1);
        assert_eq!(kept[(5.75 * SR as f32) as usize], 0.0);
    }

    #[test]
    fn expires_after_five_minutes() {
        let mut ctx = ContextAudio::default();
        let now = Instant::now();
        ctx.remember(&vec![0.1; SR], now);
        assert!(ctx.get(now + Duration::from_secs(299)).is_some());
        assert!(ctx.get(now + CONTEXT_EXPIRY).is_none());
        assert!(ctx.samples.is_empty());
    }

    #[test]
    fn remembering_after_expiry_starts_fresh() {
        let mut ctx = ContextAudio::default();
        let now = Instant::now();
        ctx.remember(&vec![0.1; SR], now);
        let later = now + CONTEXT_EXPIRY;
        ctx.remember(&vec![0.2; SR], later);
        assert_eq!(ctx.get(later).unwrap(), vec![0.2; SR]);
    }

    #[test]
    fn empty_store_has_no_context() {
        assert!(ContextAudio::default().get(Instant::now()).is_none());
    }

    /// Word errors between two texts, after case/accent/punctuation normalization.
    fn word_errors(reference: &str, hypothesis: &str) -> usize {
        let norm = crate::wake_word::wake_word::normalize_text;
        let (r, h) = (norm(reference), norm(hypothesis));
        let (r, h): (Vec<&str>, Vec<&str>) = (
            r.split_whitespace().collect(),
            h.split_whitespace().collect(),
        );
        let mut prev: Vec<usize> = (0..=h.len()).collect();
        for (i, rw) in r.iter().enumerate() {
            let mut cur = vec![i + 1; h.len() + 1];
            for (j, hw) in h.iter().enumerate() {
                cur[j + 1] = (prev[j] + usize::from(rw != hw))
                    .min(prev[j + 1] + 1)
                    .min(cur[j] + 1);
            }
            prev = cur;
        }
        prev[h.len()]
    }

    /// Benchmark: for each 16 kHz mono wav listed in PLUMED_BENCH_WAVS, splits off
    /// the last PLUMED_SHORT_WORDS (default 2) words as a "short dictation" and
    /// transcribes it alone and behind the preceding audio. The reference is the
    /// full-clip decode of those words (an approximation: it also had context).
    /// `PLUMED_MODEL_DIR=... PLUMED_BENCH_WAVS=... cargo test --release --lib
    /// short_context::tests::bench_short_context -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn bench_short_context() {
        use crate::engine::transcription_engine::TranscriptionEngine;
        use crate::engine::ParakeetEngine;
        let env = |k: &str| std::env::var(k).ok();
        let tokenizer =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../resources/tokenizer.json");
        let model_dir = env("PLUMED_MODEL_DIR").expect("PLUMED_MODEL_DIR");
        let n_words: usize = env("PLUMED_SHORT_WORDS").map_or(2, |v| v.parse().expect("number"));
        let mut engine =
            ParakeetEngine::load_int8(std::path::Path::new(&model_dir), Some(tokenizer))
                .expect("model");
        let wavs = std::fs::read_to_string(env("PLUMED_BENCH_WAVS").expect("PLUMED_BENCH_WAVS"))
            .expect("list");
        let (mut ref_words, mut iso_err, mut ctx_err, mut clips) = (0, 0, 0, 0);
        for wav in wavs.lines() {
            let samples: Vec<f32> = hound::WavReader::open(wav)
                .expect("wav")
                .samples::<i16>()
                .map(|s| s.expect("sample") as f32 / 32768.0)
                .collect();
            let full = engine
                .transcribe_samples(pad_with_silence(&samples, SR as u32), None)
                .expect("transcribe");
            let starts: Vec<f32> = full
                .segments
                .iter()
                .filter(|seg| seg.text.starts_with(' '))
                .map(|seg| seg.start)
                .collect();
            if starts.len() <= n_words {
                continue;
            }
            // Cut 80 ms (one encoder frame) before the onset of the n-th last word
            let onset = starts[starts.len() - n_words] - INFERENCE_SILENCE_PAD_SECS - 0.08;
            let cut = ((onset.max(0.0) * SR as f32) as usize).min(samples.len());
            let reference = full
                .text
                .split_whitespace()
                .skip(starts.len() - n_words)
                .collect::<Vec<_>>()
                .join(" ");
            let (before, short) = samples.split_at(cut);
            let context = &before[before.len().saturating_sub(CONTEXT_SECS * SR)..];
            let isolated = engine
                .transcribe_samples(pad_with_silence(short, SR as u32), None)
                .expect("transcribe")
                .text;
            let (input, cut_secs) = build_input(context, short);
            let with_context = engine
                .transcribe_samples_from(input, cut_secs)
                .expect("transcribe")
                .text;
            println!(
                "REF\t{}\nISO\t{}\nCTX\t{}\n",
                reference, isolated, with_context
            );
            ref_words += reference.split_whitespace().count();
            iso_err += word_errors(&reference, &isolated);
            ctx_err += word_errors(&reference, &with_context);
            clips += 1;
        }
        let pct = |e: usize| 100.0 * e as f32 / ref_words.max(1) as f32;
        eprintln!(
            "{} clips, {} reference words: isolated WER {:.1}%, with context WER {:.1}%",
            clips,
            ref_words,
            pct(iso_err),
            pct(ctx_err)
        );
    }
}
