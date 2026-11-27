//! Speaker diarization with NVIDIA's Nemotron-3-Diarization (Streaming
//! Sortformer), Plumed fork. The model repo
//! (Olicorne/Nemotron-3-Diarization-web-onnx) ships two graphs: `embed.onnx`
//! (feature stacking and projection, local to each group of 8 mel frames)
//! and `step` (one chunk through the encoder and head). Everything stateful
//! lives in `sortformer.rs`.
//!
//! [`SortformerStream`] takes audio as it arrives and returns speaker
//! probabilities as soon as a chunk (340 encoder frames, 27.2 s, plus 3.2 s
//! of look-ahead) is complete, identical to one pass over the whole clip.

use super::mel::{num_mel_frames, MelFrontEnd, HOP, MEL_MARGIN_FRAMES, N_MELS};
use super::sortformer::ChunkLoop;
use super::types::{SortformerConfig, SortformerRunner};
use crate::engine::engine::build_session;
use anyhow::{anyhow, Context, Result};
use ndarray::{Array3, ArrayView3};
use ort::inputs;
use ort::session::Session;
use ort::value::TensorRef;
use std::path::Path;

pub const SAMPLE_RATE: usize = 16000;
/// Duration of one probability row (one mel frame).
pub const FRAME_SEC: f64 = HOP as f64 / SAMPLE_RATE as f64;
pub const EMBED_FILE: &str = "embed.onnx";
pub const CONFIG_FILE: &str = "diarization-config.json";
pub const SILENCE_FILE: &str = "silence_embeds.bin";
/// The step graph used by Plumed: int8 runs about twice as fast as fp32 on
/// CPU for the same decisions on the repo's verification clip.
pub const STEP_FILE_INT8: &str = "int8/step.int8.onnx";
pub const STEP_FILE_FP32: &str = "fp32/step.onnx";
// Embed in batches of at least this many mel frames (3.2 s) while the audio
// grows, to keep the number of tiny graph calls down.
const EMBED_BATCH_FRAMES: usize = 320;

/// The two ONNX sessions of the model.
pub struct OrtSortformer {
    embed: Session,
    step: Session,
    hidden: usize,
}

impl SortformerRunner for OrtSortformer {
    fn run_embed(&mut self, features: &[f32], num_frames: usize) -> Result<Vec<f32>> {
        let view = ArrayView3::from_shape((1, num_frames, N_MELS), features)?;
        let outputs = self
            .embed
            .run(inputs!["features" => TensorRef::from_array_view(view)?])?;
        let embeds = outputs
            .get("embeds")
            .ok_or_else(|| anyhow!("embed.onnx has no `embeds` output"))?
            .try_extract_array::<f32>()?;
        Ok(embeds.iter().copied().collect())
    }

    fn run_step(&mut self, embeds: &[f64], num_rows: usize) -> Result<Vec<f64>> {
        let input = Array3::from_shape_vec(
            (1, num_rows, self.hidden),
            embeds.iter().map(|&v| v as f32).collect(),
        )?;
        let outputs = self
            .step
            .run(inputs!["embeds" => TensorRef::from_array_view(input.view())?])?;
        let logits = outputs
            .get("logits")
            .ok_or_else(|| anyhow!("step graph has no `logits` output"))?
            .try_extract_array::<f32>()?;
        Ok(logits.iter().map(|&v| v as f64).collect())
    }
}

/// A loaded model: its config, learned silence embedding and sessions.
pub struct DiarizationModel {
    pub config: SortformerConfig,
    pub silence: Vec<f64>,
    pub runner: OrtSortformer,
}

impl DiarizationModel {
    /// Load from a local copy of the model repo; `step_file` is one of the
    /// `STEP_FILE_*` paths.
    pub fn load(dir: &Path, step_file: &str, intra_threads: Option<usize>) -> Result<Self> {
        let config: SortformerConfig = serde_json::from_slice(
            &std::fs::read(dir.join(CONFIG_FILE)).context("reading diarization-config.json")?,
        )?;
        let bytes = std::fs::read(dir.join(SILENCE_FILE)).context("reading silence_embeds.bin")?;
        if bytes.len() != config.hidden_size * 4 {
            return Err(anyhow!(
                "silence_embeds.bin is {} bytes, expected {}",
                bytes.len(),
                config.hidden_size * 4
            ));
        }
        let silence = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b) as f64)
            .collect();
        let runner = OrtSortformer {
            embed: build_session(&dir.join(EMBED_FILE), intra_threads)?,
            step: build_session(&dir.join(step_file), intra_threads)?,
            hidden: config.hidden_size,
        };
        Ok(Self {
            config,
            silence,
            runner,
        })
    }

    pub fn stream(&self) -> SortformerStream {
        SortformerStream::new(self.config.clone(), self.silence.clone())
    }
}

/// Incremental diarization of one recording: push 16 kHz mono audio with
/// [`push`](Self::push), close with [`finish`](Self::finish). Both return the
/// new rows of speaker probabilities (`[frames][num_speakers]`, one row per
/// 10 ms from the start of the recording). Only the audio a pending mel frame
/// still needs is kept, so memory stays flat over hours. A clone can be
/// finished to settle the audio so far while the original goes on.
#[derive(Clone)]
pub struct SortformerStream {
    front: MelFrontEnd,
    chunks: ChunkLoop,
    factor: usize,
    chunk_frames: usize,
    /// Samples from absolute index `pcm_start`.
    pcm: Vec<f32>,
    pcm_start: usize,
    total_samples: usize,
    /// Next mel frame without an embed (a multiple of `factor` until the end).
    next_frame: usize,
}

impl SortformerStream {
    pub fn new(config: SortformerConfig, silence: Vec<f64>) -> Self {
        let factor = config.subsampling_factor;
        let chunk_frames = config.offline.chunk_length * factor;
        Self {
            front: MelFrontEnd::new(),
            chunks: ChunkLoop::new(config, silence),
            factor,
            chunk_frames,
            pcm: Vec::new(),
            pcm_start: 0,
            total_samples: 0,
            next_frame: 0,
        }
    }

    /// Samples pushed so far.
    pub fn total_samples(&self) -> usize {
        self.total_samples
    }

    /// Probability rows returned so far.
    pub fn frames_written(&self) -> usize {
        self.chunks.frames_written()
    }

    pub fn push(&mut self, runner: &mut dyn SortformerRunner, samples: &[f32]) -> Result<Vec<f32>> {
        self.pcm.extend_from_slice(samples);
        self.total_samples += samples.len();
        // frame f is final once its window's margin is in: (f + 3) * 160 <= N
        let ready = (self.total_samples / HOP).saturating_sub(MEL_MARGIN_FRAMES);
        let ready = ready - ready % self.factor;
        if ready >= self.next_frame + EMBED_BATCH_FRAMES {
            self.embed_until(runner, ready, self.total_samples / HOP)?;
        }
        self.chunks.run(runner, None)
    }

    /// The audio has ended: run the remaining chunks.
    pub fn finish(&mut self, runner: &mut dyn SortformerRunner) -> Result<Vec<f32>> {
        let num_frames = num_mel_frames(self.total_samples);
        self.embed_until(runner, num_frames, self.total_samples / HOP)?;
        self.chunks.run(runner, Some(num_frames))
    }

    /// Embed mel frames `[next_frame, f_end)`, in slices of one chunk.
    fn embed_until(
        &mut self,
        runner: &mut dyn SortformerRunner,
        f_end: usize,
        valid_frames: usize,
    ) -> Result<()> {
        while self.next_frame < f_end {
            let f0 = self.next_frame;
            let f1 = f_end.min(f0 + self.chunk_frames);
            let features = self
                .front
                .mel_frames(&self.pcm, self.pcm_start, f0, f1, valid_frames);
            let embeds = runner.run_embed(&features, f1 - f0)?;
            self.chunks.push_embeds(&embeds);
            self.next_frame = f1;
        }
        // drop the audio no pending frame reads any more
        let keep_from = self.next_frame.saturating_sub(MEL_MARGIN_FRAMES) * HOP;
        if keep_from > self.pcm_start {
            let n = (keep_from - self.pcm_start).min(self.pcm.len());
            self.pcm.drain(..n);
            self.pcm_start += n;
        }
        Ok(())
    }
}
