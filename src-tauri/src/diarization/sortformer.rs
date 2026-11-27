//! Streaming Sortformer chunk loop and Arrival-Order Speaker Cache (Plumed
//! fork), ported line for line from parakeet_web's `app/src/sortformer.js`,
//! itself a port of transformers' `Nemotron3DiarizationSpeakerCache` and the
//! offline loop of `Nemotron3DiarizationForAudioFrameClassification.forward`.
//!
//! Unlike the JS port, the loop is incremental: embeds are pushed as audio
//! arrives and a chunk runs as soon as its look-ahead is available, which
//! gives exactly the probabilities of one whole-clip pass.

use super::types::{SortformerConfig, SortformerRunner};
use std::cmp::Ordering;

type Row = Vec<f64>;

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Descending order that stays consistent on infinite scores.
fn by_score_desc(x: f64, y: f64) -> Ordering {
    y.partial_cmp(&x).unwrap_or(Ordering::Equal)
}

#[derive(Clone)]
pub struct SpeakerCache {
    num_speakers: usize,
    factor: usize,
    fifo_length: usize,
    update_period: usize,
    cache_length: usize,
    num_silence: usize,
    threshold: f64,
    latest_boost: f64,
    min_positive: usize,
    num_strong: usize,
    num_weak: usize,
    silence: Row,
    cache_embeds: Vec<Row>,
    cache_probs: Vec<Row>,
    fifo: Vec<Row>,
    is_compressed: bool,
}

impl SpeakerCache {
    pub fn new(config: &SortformerConfig, silence: Row) -> Self {
        let sc = &config.speaker_cache;
        let budget =
            (sc.length / config.num_speakers) as f64 - sc.silence_frames_per_speaker as f64;
        Self {
            num_speakers: config.num_speakers,
            factor: config.subsampling_factor,
            fifo_length: config.offline.fifo_length,
            update_period: config.offline.speaker_cache_update_period,
            cache_length: sc.length,
            num_silence: sc.silence_frames_per_speaker,
            threshold: sc.prediction_score_threshold,
            latest_boost: sc.latest_frames_score_boost,
            min_positive: (budget * sc.min_positive_scores_rate).floor() as usize,
            num_strong: (budget * sc.strong_boost_rate).floor() as usize,
            num_weak: (budget * sc.weak_boost_rate).floor() as usize,
            silence,
            cache_embeds: Vec::new(),
            cache_probs: Vec::new(),
            fifo: Vec::new(),
            is_compressed: false,
        }
    }

    /// Embeds of [cache | FIFO], the frames every chunk attends to before itself.
    pub fn cached_rows(&self) -> Vec<Row> {
        self.cache_embeds
            .iter()
            .chain(self.fifo.iter())
            .cloned()
            .collect()
    }

    /// Speaker probabilities at the encoder frame rate: sigmoid then mean over
    /// each group of `factor` upsampled logit rows.
    fn pool_probs(&self, logits: &[f64], num_rows: usize) -> Vec<Row> {
        let s_count = self.num_speakers;
        (0..num_rows)
            .map(|r| {
                let mut p = vec![0f64; s_count];
                for j in 0..self.factor {
                    let base = (r * self.factor + j) * s_count;
                    for (s, v) in p.iter_mut().enumerate() {
                        *v += sigmoid(logits[base + s]);
                    }
                }
                p.iter_mut().for_each(|v| *v /= self.factor as f64);
                p
            })
            .collect()
    }

    /// Push a processed chunk, moving FIFO overflow into the cache and
    /// compressing it when it outgrows its capacity. `input_rows` are the
    /// step's input frames (cache, FIFO, chunk, look-ahead).
    pub fn update(&mut self, input_rows: &[Row], logits: &[f64], num_chunk_frames: usize) {
        let num_cache = self.cache_embeds.len();
        let num_fifo = self.fifo.len();
        let probs = self.pool_probs(logits, input_rows.len());
        let chunk_start = num_cache + num_fifo;
        let mut fifo_embeds: Vec<Row> = std::mem::take(&mut self.fifo);
        fifo_embeds.extend_from_slice(&input_rows[chunk_start..chunk_start + num_chunk_frames]);

        let num_popped = self.num_popped(fifo_embeds.len());
        if num_popped > 0 {
            let fifo_probs = &probs[num_cache..num_cache + fifo_embeds.len()];
            // an uncompressed cache still holds plain chunk frames, whose
            // probabilities this step re-estimates; a compressed one is out of
            // order, so the probs stored alongside its frames are the only ones
            let mut cache_probs = if self.is_compressed {
                std::mem::take(&mut self.cache_probs)
            } else {
                probs[..num_cache].to_vec()
            };
            let mut cache_embeds = std::mem::take(&mut self.cache_embeds);
            cache_embeds.extend(fifo_embeds.drain(..num_popped));
            cache_probs.extend_from_slice(&fifo_probs[..num_popped]);
            if cache_embeds.len() > self.cache_length {
                (cache_embeds, cache_probs) = self.compress(&cache_embeds, &cache_probs);
                self.is_compressed = true;
            }
            self.cache_embeds = cache_embeds;
            self.cache_probs = cache_probs;
        }
        self.fifo = fifo_embeds;
    }

    /// No frames move to the cache until the FIFO overflows; then at least
    /// `update_period` oldest frames move, plus whatever restores capacity.
    fn num_popped(&self, num_fifo_frames: usize) -> usize {
        if num_fifo_frames <= self.fifo_length {
            return 0;
        }
        self.update_period
            .max(num_fifo_frames - self.fifo_length)
            .min(num_fifo_frames)
    }

    fn frame_scores(&self, probs: &[Row]) -> Vec<Row> {
        let s_count = self.num_speakers;
        let thr = self.threshold;
        let mut positives = vec![0usize; s_count];
        let mut scores: Vec<Row> = probs
            .iter()
            .map(|p| {
                let log_comp: Vec<f64> = p.iter().map(|&v| (1.0 - v).max(thr).ln()).collect();
                let sum_comp: f64 = log_comp.iter().sum();
                (0..s_count)
                    .map(|s| {
                        let v = if p[s] > 0.5 {
                            p[s].max(thr).ln() - log_comp[s] + sum_comp - 0.5f64.ln()
                        } else {
                            f64::NEG_INFINITY
                        };
                        if v > 0.0 {
                            positives[s] += 1;
                        }
                        v
                    })
                    .collect()
            })
            .collect();
        // a speaker with enough positive-score frames keeps only those
        for (s, &count) in positives.iter().enumerate() {
            if count < self.min_positive {
                continue;
            }
            for (f, row) in scores.iter_mut().enumerate() {
                if probs[f][s] > 0.5 && row[s] <= 0.0 {
                    row[s] = f64::NEG_INFINITY;
                }
            }
        }
        scores
    }

    /// Add `boost` to each speaker's `k` highest-scoring frames.
    fn boost_scores(&self, scores: &mut [Row], k: usize, boost: f64) {
        for s in 0..self.num_speakers {
            let mut order: Vec<usize> = (0..scores.len()).collect();
            order.sort_by(|&a, &b| by_score_desc(scores[a][s], scores[b][s]).then(a.cmp(&b)));
            for &f in order.iter().take(k) {
                scores[f][s] += boost;
            }
        }
    }

    /// Keep the `cache_length` most important frames, grouped by speaker and
    /// in their original order within a speaker; each speaker's silence slot
    /// holds the learned silence embedding.
    #[allow(clippy::needless_range_loop)]
    fn compress(&self, embeds: &[Row], probs: &[Row]) -> (Vec<Row>, Vec<Row>) {
        let s_count = self.num_speakers;
        let f_count = probs.len();
        let mut scores = self.frame_scores(probs);
        // frames beyond the cache capacity are the ones popped from the FIFO
        for row in scores.iter_mut().skip(self.cache_length) {
            row.iter_mut().for_each(|v| *v += self.latest_boost);
        }
        self.boost_scores(&mut scores, self.num_strong, -2.0 * 0.5f64.ln());
        self.boost_scores(&mut scores, self.num_weak, -(0.5f64.ln()));

        // Flatten speaker-major over F + num_silence frames, silence frames
        // scoring +inf, and keep the top `cache_length`. Ranked candidates
        // scoring -inf turn into the silence frame. Sorting the kept flat
        // indices gives speaker-major order, frames ascending within a speaker.
        let rows = f_count + self.num_silence;
        let mut flat: Vec<(usize, f64)> = Vec::with_capacity(rows * s_count);
        for s in 0..s_count {
            for f in 0..rows {
                let score = if f < f_count {
                    scores[f][s]
                } else {
                    f64::INFINITY
                };
                flat.push((s * rows + f, score));
            }
        }
        flat.sort_by(|a, b| by_score_desc(a.1, b.1).then(a.0.cmp(&b.0)));
        let sentinel = rows * s_count;
        let mut kept: Vec<usize> = flat
            .iter()
            .take(self.cache_length)
            .map(|&(idx, score)| {
                if score == f64::NEG_INFINITY {
                    sentinel
                } else {
                    idx
                }
            })
            .collect();
        kept.sort_unstable();
        let zero = vec![0f64; s_count];
        kept.iter()
            .map(|&idx| {
                let frame = if idx == sentinel {
                    f_count
                } else {
                    (idx % rows).min(f_count)
                };
                if frame == f_count {
                    (self.silence.clone(), zero.clone())
                } else {
                    (embeds[frame].clone(), probs[frame].clone())
                }
            })
            .unzip()
    }
}

/// The offline chunk loop, fed incrementally. Embeds are pushed as they are
/// computed; [`ChunkLoop::run`] processes every chunk whose look-ahead is
/// available (or, once the clip length is known, every remaining chunk) and
/// returns the new rows of speaker probabilities, one per 10 ms mel frame.
#[derive(Clone)]
pub struct ChunkLoop {
    config: SortformerConfig,
    cache: SpeakerCache,
    /// Embeds from absolute index `pending_start` not yet consumed by a chunk
    /// (a chunk's look-ahead is fed again with the next chunk).
    pending: Vec<Row>,
    pending_start: usize,
    frames_written: usize,
    chunks_done: usize,
}

impl ChunkLoop {
    pub fn new(config: SortformerConfig, silence: Row) -> Self {
        let cache = SpeakerCache::new(&config, silence);
        Self {
            config,
            cache,
            pending: Vec::new(),
            pending_start: 0,
            frames_written: 0,
            chunks_done: 0,
        }
    }

    /// Append embeds (`[n][hidden]`, flattened) in clip order.
    pub fn push_embeds(&mut self, embeds: &[f32]) {
        let hidden = self.config.hidden_size;
        self.pending.extend(
            embeds
                .chunks_exact(hidden)
                .map(|row| row.iter().map(|&v| v as f64).collect::<Row>()),
        );
    }

    pub fn push_embed_rows(&mut self, rows: impl IntoIterator<Item = Row>) {
        self.pending.extend(rows);
    }

    pub fn frames_written(&self) -> usize {
        self.frames_written
    }

    pub fn chunks_done(&self) -> usize {
        self.chunks_done
    }

    /// Run every ready chunk. `final_frames` is the clip's mel frame count once
    /// the audio has ended (all its embeds pushed), `None` while it still grows.
    /// Returns probabilities `[new_frames][num_speakers]`, flattened.
    pub fn run(
        &mut self,
        runner: &mut dyn SortformerRunner,
        final_frames: Option<usize>,
    ) -> anyhow::Result<Vec<f32>> {
        let factor = self.config.subsampling_factor;
        let hidden = self.config.hidden_size;
        let s_count = self.config.num_speakers;
        let chunk_length = self.config.offline.chunk_length;
        let right_context = self.config.offline.chunk_right_context;
        let mut out = Vec::new();
        loop {
            let start = self.pending_start;
            let available = self.pending_start + self.pending.len();
            let (end, context_end) = match final_frames {
                Some(_) => {
                    if start >= available {
                        break;
                    }
                    let end = (start + chunk_length).min(available);
                    (end, (end + right_context).min(available))
                }
                None => {
                    let end = start + chunk_length;
                    if end + right_context > available {
                        break;
                    }
                    (end, end + right_context)
                }
            };
            let num_chunk_frames = end - start;
            let mut input_rows = self.cache.cached_rows();
            let cached_length = input_rows.len();
            input_rows.extend_from_slice(&self.pending[..context_end - start]);
            let input: Vec<f64> = input_rows.iter().flatten().copied().collect();
            debug_assert_eq!(input.len(), input_rows.len() * hidden);
            let logits = runner.run_step(&input, input_rows.len())?;
            self.cache.update(&input_rows, &logits, num_chunk_frames);

            // this chunk's own rows, minus the cache before and the look-ahead
            // after; at the clip's end the last row may be stacking padding
            let from = cached_length * factor;
            let mut to = (cached_length + num_chunk_frames) * factor;
            if let Some(total) = final_frames {
                to = to.min(from + total.saturating_sub(self.frames_written));
            }
            for r in from..to {
                out.extend(
                    logits[r * s_count..(r + 1) * s_count]
                        .iter()
                        .map(|&l| sigmoid(l) as f32),
                );
            }
            self.frames_written += to.saturating_sub(from);
            self.pending.drain(..num_chunk_frames);
            self.pending_start = end;
            self.chunks_done += 1;
        }
        Ok(out)
    }
}
