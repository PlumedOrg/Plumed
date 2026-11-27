use serde::Deserialize;

/// The model repo's `diarization-config.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct SortformerConfig {
    pub hidden_size: usize,
    pub subsampling_factor: usize,
    pub num_speakers: usize,
    #[serde(default = "default_mel_bins")]
    pub num_mel_bins: usize,
    pub offline: ChunkConfig,
    pub speaker_cache: SpeakerCacheConfig,
}

fn default_mel_bins() -> usize {
    128
}

/// Chunking of the offline loop: `chunk_length` encoder frames (80 ms each)
/// per step, seen with `chunk_right_context` frames of look-ahead.
#[derive(Debug, Clone, Deserialize)]
pub struct ChunkConfig {
    pub chunk_length: usize,
    pub chunk_right_context: usize,
    pub fifo_length: usize,
    pub speaker_cache_update_period: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpeakerCacheConfig {
    pub length: usize,
    pub silence_frames_per_speaker: usize,
    pub prediction_score_threshold: f64,
    pub latest_frames_score_boost: f64,
    pub min_positive_scores_rate: f64,
    pub strong_boost_rate: f64,
    pub weak_boost_rate: f64,
}

/// One speaker turn, in seconds from the start of the audio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub start: f64,
    pub end: f64,
    pub speaker: usize,
}

/// The ONNX graphs behind the diarization loop, abstracted so the loop can be
/// tested with a closed-form fake step.
pub trait SortformerRunner {
    /// Raw log-mel `[num_frames][128]` to embeds `[ceil(num_frames / 8)][hidden]`.
    fn run_embed(&mut self, features: &[f32], num_frames: usize) -> anyhow::Result<Vec<f32>>;
    /// Embeds `[num_rows][hidden]` to logits `[num_rows * 8][num_speakers]`.
    fn run_step(&mut self, embeds: &[f64], num_rows: usize) -> anyhow::Result<Vec<f64>>;
}
