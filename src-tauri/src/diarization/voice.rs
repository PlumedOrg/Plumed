//! Voice embeddings with 3D-Speaker's CAM++ (Plumed fork, written with
//! Claude Code): a 192 value fingerprint of who speaks in a clip, compared
//! by cosine similarity. The Réunion page uses it to merge the speakers the
//! diarization split and to find them again after a pause.
//!
//! The graph (`csukuangfj/speaker-embedding-models`, sherpa-onnx export)
//! takes `x` = `[N, T, 80]` global-mean fbank (see `fbank.rs`) and returns
//! `embedding` = `[N, 192]`.

use super::fbank::{Fbank, NUM_BINS};
use crate::engine::engine::build_session;
use anyhow::{anyhow, Result};
use ndarray::ArrayView3;
use ort::inputs;
use ort::session::Session;
use ort::value::TensorRef;
use std::path::Path;

pub struct VoiceModel {
    session: Session,
    fbank: Fbank,
}

impl VoiceModel {
    pub fn load(path: &Path) -> Result<Self> {
        Ok(Self {
            session: build_session(path, Some(1))?,
            fbank: Fbank::new(),
        })
    }

    /// L2-normalized embedding of 16 kHz mono audio (at least 25 ms).
    pub fn embed(&mut self, pcm: &[f32]) -> Result<Vec<f32>> {
        let (feats, n_frames) = self.fbank.normalized(pcm);
        if n_frames == 0 {
            return Err(anyhow!("voice clip too short"));
        }
        let view = ArrayView3::from_shape((1, n_frames, NUM_BINS), &feats)?;
        let outputs = self
            .session
            .run(inputs!["x" => TensorRef::from_array_view(view)?])?;
        let embedding = outputs
            .get("embedding")
            .ok_or_else(|| anyhow!("the voice model has no `embedding` output"))?
            .try_extract_array::<f32>()?;
        Ok(normalized(embedding.iter().copied().collect()))
    }
}

fn normalized(mut v: Vec<f32>) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.iter_mut().for_each(|x| *x /= norm);
    }
    v
}

/// Cosine similarity of two normalized embeddings.
pub fn similarity(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn similarity_of_normalized_vectors_is_their_cosine() {
        let a = normalized(vec![3.0, 4.0]);
        assert!((a[0] - 0.6).abs() < 1e-6);
        assert!((similarity(&a, &a) - 1.0).abs() < 1e-6);
        assert!(similarity(&a, &normalized(vec![-4.0, 3.0])).abs() < 1e-6);
        assert_eq!(normalized(vec![0.0, 0.0]), vec![0.0, 0.0]);
    }

    /// Needs the model file:
    /// `PLUMED_VOICE_MODEL=.../campplus.onnx cargo test --lib voice -- --ignored`
    #[test]
    #[ignore]
    fn embeds_like_the_reference_pipeline() {
        let model = PathBuf::from(std::env::var("PLUMED_VOICE_MODEL").expect("PLUMED_VOICE_MODEL"));
        let mut model = VoiceModel::load(&model).unwrap();
        let wav = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/diarization/two-speakers.wav");
        let mut reader = hound::WavReader::open(wav).unwrap();
        assert_eq!(reader.spec().sample_rate, 16000);
        let pcm: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / 32768.0)
            .collect();
        let embedding = model.embed(&pcm).unwrap();
        assert_eq!(embedding.len(), 192);
        let norm: f32 = embedding.iter().map(|x| x * x).sum();
        assert!((norm - 1.0).abs() < 1e-4);
        // sherpa-onnx's pipeline (kaldi-native-fbank, onnxruntime in python)
        let reference = [-0.1335, -0.0866, -0.0885, -0.0103, -0.0815, -0.0137];
        for (value, expected) in embedding.iter().zip(reference) {
            assert!((value - expected).abs() < 1e-3, "{:?}", &embedding[..6]);
        }
    }
}
