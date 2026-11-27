//! Kaldi-compatible 80 bin log-mel filterbank (Plumed fork, written with
//! Claude Code), the front end of the CAM++ voice model. Port of
//! parakeet_web's `app/src/fbank.js`, matching kaldi-native-fbank's defaults
//! (what sherpa-onnx and 3D-Speaker feed CAM++):
//!
//! 1. 25 ms frames every 10 ms (400 / 160 samples), `snip_edges`: only whole
//!    frames, so `1 + (n - 400) / 160` of them;
//! 2. DC removal, then pre-emphasis `x[i] - 0.97 x[i-1]` (`x[0]` uses itself);
//! 3. povey window `(0.5 - 0.5 cos(2 pi i / 399))^0.85`, zero padded to 512;
//! 4. power spectrum, 80 triangular mel bins from 20 Hz to 8 kHz on the
//!    `1127 ln(1 + f / 700)` scale (the nyquist bin excluded);
//! 5. `ln(max(energy, f32::EPSILON))`.
//!
//! The model expects "global-mean" features: `normalized` also subtracts the
//! mean of each bin over time.

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

pub const NUM_BINS: usize = 80;
const FRAME_LEN: usize = 400;
const FRAME_SHIFT: usize = 160;
const NFFT: usize = 512;
const SAMPLE_RATE: f64 = 16000.0;
const LOW_FREQ: f64 = 20.0;
const HIGH_FREQ: f64 = 8000.0;
const PREEMPH: f64 = 0.97;

fn hz_to_mel(freq: f64) -> f64 {
    1127.0 * (1.0 + freq / 700.0).ln()
}

pub struct Fbank {
    /// `[NUM_BINS][NFFT / 2]`, row-major.
    filters: Vec<f64>,
    window: Vec<f64>,
    fft: Arc<dyn Fft<f64>>,
}

impl Default for Fbank {
    fn default() -> Self {
        Self::new()
    }
}

impl Fbank {
    pub fn new() -> Self {
        let n_bins = NFFT / 2;
        let bin_width = SAMPLE_RATE / NFFT as f64;
        let (mel_low, mel_high) = (hz_to_mel(LOW_FREQ), hz_to_mel(HIGH_FREQ));
        let delta = (mel_high - mel_low) / (NUM_BINS + 1) as f64;
        let mut filters = vec![0f64; NUM_BINS * n_bins];
        for b in 0..NUM_BINS {
            let left = mel_low + b as f64 * delta;
            let center = left + delta;
            let right = center + delta;
            for i in 0..n_bins {
                let mel = hz_to_mel(bin_width * i as f64);
                filters[b * n_bins + i] = if mel > left && mel <= center {
                    (mel - left) / (center - left)
                } else if mel > center && mel < right {
                    (right - mel) / (right - center)
                } else {
                    0.0
                };
            }
        }
        let window = (0..FRAME_LEN)
            .map(|i| {
                let cos = (2.0 * std::f64::consts::PI * i as f64 / (FRAME_LEN - 1) as f64).cos();
                (0.5 - 0.5 * cos).powf(0.85)
            })
            .collect();
        Self {
            filters,
            window,
            fft: FftPlanner::new().plan_fft_forward(NFFT),
        }
    }

    /// Raw log fbank, frame-major (`[frames][NUM_BINS]`), and the frame count
    /// (0 under 25 ms of audio).
    pub fn raw(&self, pcm: &[f32]) -> (Vec<f32>, usize) {
        if pcm.len() < FRAME_LEN {
            return (Vec::new(), 0);
        }
        let n_frames = 1 + (pcm.len() - FRAME_LEN) / FRAME_SHIFT;
        let n_bins = NFFT / 2;
        let mut out = vec![0f32; n_frames * NUM_BINS];
        let mut frame = [0f64; FRAME_LEN];
        let mut buf = vec![Complex::new(0f64, 0f64); NFFT];
        let mut power = vec![0f64; n_bins];
        for t in 0..n_frames {
            let samples = &pcm[t * FRAME_SHIFT..t * FRAME_SHIFT + FRAME_LEN];
            let mean = samples.iter().map(|&s| s as f64).sum::<f64>() / FRAME_LEN as f64;
            for (f, &s) in frame.iter_mut().zip(samples) {
                *f = s as f64 - mean;
            }
            for i in (1..FRAME_LEN).rev() {
                frame[i] -= PREEMPH * frame[i - 1];
            }
            frame[0] -= PREEMPH * frame[0];
            for (k, c) in buf.iter_mut().enumerate() {
                let x = if k < FRAME_LEN {
                    frame[k] * self.window[k]
                } else {
                    0.0
                };
                *c = Complex::new(x, 0.0);
            }
            self.fft.process(&mut buf);
            for (p, c) in power.iter_mut().zip(&buf) {
                *p = c.norm_sqr();
            }
            for (b, value) in out[t * NUM_BINS..(t + 1) * NUM_BINS].iter_mut().enumerate() {
                let energy: f64 = self.filters[b * n_bins..(b + 1) * n_bins]
                    .iter()
                    .zip(&power)
                    .map(|(w, p)| w * p)
                    .sum();
                *value = energy.max(f32::EPSILON as f64).ln() as f32;
            }
        }
        (out, n_frames)
    }

    /// What CAM++ takes: the raw fbank minus the mean of each bin over time.
    pub fn normalized(&self, pcm: &[f32]) -> (Vec<f32>, usize) {
        let (mut feats, n_frames) = self.raw(pcm);
        for b in 0..NUM_BINS {
            let mean = (0..n_frames)
                .map(|t| feats[t * NUM_BINS + b] as f64)
                .sum::<f64>()
                / n_frames as f64;
            for t in 0..n_frames {
                feats[t * NUM_BINS + b] -= mean as f32;
            }
        }
        (feats, n_frames)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// The signal of `scripts/gen-fbank-fixture.py`: a chirp plus a tone in
    /// a slow envelope, computed in f64 then cast to f32 on both sides.
    fn signal() -> Vec<f32> {
        (0..16000 + 77)
            .map(|i| {
                let t = i as f64 / 16000.0;
                let tau = 2.0 * std::f64::consts::PI;
                let chirp = (tau * (100.0 * t + 1500.0 * t * t)).sin();
                let tone = 0.3 * (tau * 440.0 * t).sin();
                let envelope = 0.5 + 0.4 * (tau * 1.5 * t).sin();
                (0.3 * envelope * (chirp + tone) + 0.01) as f32
            })
            .collect()
    }

    #[test]
    fn matches_kaldi_native_fbank() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/diarization/fbank-reference.bin");
        let expected: Vec<f32> = std::fs::read(path)
            .unwrap()
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let (feats, n_frames) = Fbank::new().raw(&signal());
        assert_eq!(n_frames, 98);
        assert_eq!(feats.len(), expected.len());
        let max_diff = feats
            .iter()
            .zip(&expected)
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);
        // 1.7e-3 here: kaldi-native-fbank computes in f32, this port in f64
        assert!(max_diff < 5e-3, "max diff {max_diff}");
    }

    #[test]
    fn normalized_bins_average_zero_and_short_clips_give_nothing() {
        let fbank = Fbank::new();
        let (feats, n_frames) = fbank.normalized(&signal());
        for b in 0..NUM_BINS {
            let mean: f32 =
                (0..n_frames).map(|t| feats[t * NUM_BINS + b]).sum::<f32>() / n_frames as f32;
            assert!(mean.abs() < 1e-4, "bin {b}: {mean}");
        }
        assert_eq!(fbank.raw(&[0.1; FRAME_LEN - 1]).1, 0);
        assert_eq!(fbank.raw(&[0.1; FRAME_LEN]).1, 1);
        // silence floors at ln(f32::EPSILON) instead of -inf
        assert!(fbank
            .raw(&[0.0; FRAME_LEN])
            .0
            .iter()
            .all(|&v| v == f32::EPSILON.ln()));
    }
}
