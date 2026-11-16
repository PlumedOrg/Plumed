use crate::audio::helpers::resample;

const SMARTMIC_CHUNK_SAMPLES: usize = 960_000;
const SMARTMIC_MAX_SAMPLES: usize = 19_200_000;

/// Accumulate raw PCM bytes (Int16 LE) into the buffer.
/// Returns `true` if samples were added, `false` if the buffer is full.
pub fn accumulate_pcm(buffer: &mut Vec<i16>, payload: &[u8]) -> bool {
    // Each sample is 2 bytes (Int16 LE)
    let sample_count = payload.len() / 2;

    if buffer.len() + sample_count > SMARTMIC_MAX_SAMPLES {
        return false;
    }

    buffer.reserve(sample_count);

    for chunk in payload.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
        buffer.push(sample);
    }

    true
}

/// Convert the accumulated Int16 buffer to Vec<f32>, resampling if needed
pub fn finalize_buffer(buffer: Vec<i16>, source_sample_rate: u32) -> Vec<f32> {
    // Convert i16 to f32 (normalize to -1.0..1.0)
    let samples_f32: Vec<f32> = buffer
        .into_iter()
        .map(|s| s as f32 / i16::MAX as f32)
        .collect();

    // Resample to 16kHz if needed
    if source_sample_rate != 16000 {
        resample(&samples_f32, source_sample_rate as usize, 16000)
    } else {
        samples_f32
    }
}

pub fn split_into_chunks(buffer: Vec<i16>, source_sample_rate: u32) -> Vec<Vec<f32>> {
    buffer
        .chunks(SMARTMIC_CHUNK_SAMPLES)
        .map(|c| finalize_buffer(c.to_vec(), source_sample_rate))
        .collect()
}

#[derive(Debug, thiserror::Error)]
pub enum TranscribeError {
    #[error("model not available")]
    ModelUnavailable(#[source] anyhow::Error),
    #[error("transcription failed")]
    Failed(#[source] anyhow::Error),
}

/// Transcribes a phone recording (pcm-s16 at `sample_rate`), chunk by chunk.
/// Shared by Smart Mic and the Plumed remote mic.
pub fn transcribe_pcm(
    app: &tauri::AppHandle,
    buffer: Vec<i16>,
    sample_rate: u32,
) -> Result<String, TranscribeError> {
    let chunks = split_into_chunks(buffer, sample_rate);
    crate::audio::preload_engine(app).map_err(TranscribeError::ModelUnavailable)?;
    let mut parts: Vec<String> = Vec::with_capacity(chunks.len());
    for samples in chunks {
        let text = crate::audio::pipeline::process_recording_from_samples(app, samples)
            .map_err(TranscribeError::Failed)?;
        if !text.is_empty() {
            parts.push(text);
        }
    }
    Ok(parts.join(" "))
}

/// Calculate the RMS (Root Mean Square) level of the given samples, normalized to 0.0-1.0
pub fn calculate_rms(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum_squares: f64 = samples
        .iter()
        .map(|&s| {
            let f = s as f64 / i16::MAX as f64;
            f * f
        })
        .sum();

    let rms = (sum_squares / samples.len() as f64).sqrt() as f32;

    // Clamp to 0.0-1.0
    rms.clamp(0.0, 1.0)
}
