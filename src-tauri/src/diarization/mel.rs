//! Raw log-mel front end of the diarization model (Plumed fork), a port of
//! parakeet_web's `computeRawMel` (`app/src/mel.js`), which matches NeMo's
//! preprocessor without the per-feature normalization:
//!
//! 1. pre-emphasis `x[n] - 0.97 x[n-1]`, rounded to f32;
//! 2. 256 zero samples of padding on each side;
//! 3. symmetric Hann window of 400 centered in 512, FFT in f64;
//! 4. power spectrum rounded to f32;
//! 5. slaney mel filterbank (f32 weights, f64 accumulation);
//! 6. `ln(mel + 2^-24)`.

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

pub const HOP: usize = 160;
pub const N_MELS: usize = 128;
const SAMPLE_RATE: f64 = 16000.0;
const N_FFT: usize = 512;
const WIN_LENGTH: usize = 400;
const PREEMPH: f64 = 0.97;
const N_FREQ_BINS: usize = N_FFT / 2 + 1;
// A frame is exact only when its window (and the one sample pre-emphasis reads
// before it) lies inside the slice it is computed from: three hops of margin on
// each side cover the 256-sample half window plus that sample.
pub const MEL_MARGIN_FRAMES: usize = 3;

// Slaney mel scale, as torchaudio.functional.melscale_fbanks.
const F_SP: f64 = 200.0 / 3.0;
const MIN_LOG_HZ: f64 = 1000.0;
const MIN_LOG_MEL: f64 = MIN_LOG_HZ / F_SP;

fn log_step() -> f64 {
    6.4f64.ln() / 27.0
}

fn hz_to_mel(freq: f64) -> f64 {
    if freq >= MIN_LOG_HZ {
        MIN_LOG_MEL + (freq / MIN_LOG_HZ).ln() / log_step()
    } else {
        freq / F_SP
    }
}

fn mel_to_hz(mel: f64) -> f64 {
    if mel >= MIN_LOG_MEL {
        MIN_LOG_HZ * (log_step() * (mel - MIN_LOG_MEL)).exp()
    } else {
        mel * F_SP
    }
}

/// Number of 10 ms mel frames a clip of `num_samples` yields (centered STFT).
pub fn num_mel_frames(num_samples: usize) -> usize {
    num_samples / HOP + 1
}

pub struct MelFrontEnd {
    /// `[N_MELS][N_FREQ_BINS]`, row-major.
    filterbank: Vec<f32>,
    window: Vec<f64>,
    fft: Arc<dyn Fft<f64>>,
}

impl Default for MelFrontEnd {
    fn default() -> Self {
        Self::new()
    }
}

impl MelFrontEnd {
    pub fn new() -> Self {
        let f_max = SAMPLE_RATE / 2.0;
        let all_freqs: Vec<f64> = (0..N_FREQ_BINS)
            .map(|i| f_max * i as f64 / (N_FREQ_BINS - 1) as f64)
            .collect();
        let (mel_min, mel_max) = (hz_to_mel(0.0), hz_to_mel(f_max));
        let n_points = N_MELS + 2;
        let f_pts: Vec<f64> = (0..n_points)
            .map(|i| mel_to_hz(mel_min + (mel_max - mel_min) * i as f64 / (n_points - 1) as f64))
            .collect();
        let mut filterbank = vec![0f32; N_MELS * N_FREQ_BINS];
        for m in 0..N_MELS {
            let enorm = 2.0 / (f_pts[m + 2] - f_pts[m]);
            for (k, &freq) in all_freqs.iter().enumerate() {
                let down = (freq - f_pts[m]) / (f_pts[m + 1] - f_pts[m]);
                let up = (f_pts[m + 2] - freq) / (f_pts[m + 2] - f_pts[m + 1]);
                filterbank[m * N_FREQ_BINS + k] = (down.min(up).max(0.0) * enorm) as f32;
            }
        }
        let pad_left = (N_FFT - WIN_LENGTH) / 2;
        let mut window = vec![0f64; N_FFT];
        for n in 0..WIN_LENGTH {
            window[pad_left + n] = 0.5
                * (1.0 - (2.0 * std::f64::consts::PI * n as f64 / (WIN_LENGTH - 1) as f64).cos());
        }
        let fft = FftPlanner::new().plan_fft_forward(N_FFT);
        Self {
            filterbank,
            window,
            fft,
        }
    }

    /// Raw log-mel of a whole clip, frame-major (`[frames][N_MELS]`), with
    /// `num_mel_frames(pcm.len())` frames, or none for a clip under one hop.
    pub fn raw_mel(&self, pcm: &[f32]) -> (Vec<f32>, usize) {
        let n = pcm.len();
        if n < HOP {
            return (Vec::new(), 0);
        }
        let pad = N_FFT / 2;
        let mut padded = vec![0f64; n + 2 * pad];
        padded[pad] = pcm[0] as f64;
        for i in 1..n {
            padded[pad + i] = (pcm[i] as f64 - PREEMPH * pcm[i - 1] as f64) as f32 as f64;
        }
        let n_frames = num_mel_frames(n);
        let mut out = vec![0f32; n_frames * N_MELS];
        let mut buf = vec![Complex::new(0f64, 0f64); N_FFT];
        let mut power = [0f32; N_FREQ_BINS];
        for t in 0..n_frames {
            let offset = t * HOP;
            for (k, slot) in buf.iter_mut().enumerate() {
                *slot = Complex::new(padded[offset + k] * self.window[k], 0.0);
            }
            self.fft.process(&mut buf);
            for (k, p) in power.iter_mut().enumerate() {
                *p = buf[k].norm_sqr() as f32;
            }
            let row = &mut out[t * N_MELS..(t + 1) * N_MELS];
            for (m, value) in row.iter_mut().enumerate() {
                let weights = &self.filterbank[m * N_FREQ_BINS..(m + 1) * N_FREQ_BINS];
                let mel: f64 = weights
                    .iter()
                    .zip(power.iter())
                    .map(|(&w, &p)| w as f64 * p as f64)
                    .sum();
                *value = (mel + 2f64.powi(-24)).ln() as f32;
            }
        }
        (out, n_frames)
    }

    /// Mel frames `[f0, f1)` (frame-major), identical to the same frames of
    /// one whole-clip pass. `buf` holds the clip's samples from absolute index
    /// `buf_start`, and must cover `[(f0 - 3) * 160, (f1 + 3) * 160)` or run to
    /// the clip's end. Frames at or past `valid_frames` (`floor(N / 160)`) are
    /// zero: the reference extractor's attention mask drops the last frame of
    /// the centered STFT.
    pub fn mel_frames(
        &self,
        buf: &[f32],
        buf_start: usize,
        f0: usize,
        f1: usize,
        valid_frames: usize,
    ) -> Vec<f32> {
        let m0 = f0.saturating_sub(MEL_MARGIN_FRAMES);
        let s0 = m0 * HOP - buf_start;
        let s1 = buf.len().min((f1 + MEL_MARGIN_FRAMES) * HOP - buf_start);
        let (raw, n_frames) = self.raw_mel(&buf[s0..s1]);
        let mut out = vec![0f32; (f1 - f0) * N_MELS];
        for f in f0..f1.min(valid_frames) {
            let t = f - m0;
            if t >= n_frames {
                break;
            }
            out[(f - f0) * N_MELS..(f - f0 + 1) * N_MELS]
                .copy_from_slice(&raw[t * N_MELS..(t + 1) * N_MELS]);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Port of parakeet_web's melFrames test: slices equal one whole-clip
    // pass, and the frame past floor(N / 160) is zeroed.
    #[test]
    fn slices_equal_one_whole_clip_pass() {
        let n = 16000 * 3 + 77;
        let mut seed: u32 = 7;
        let pcm: Vec<f32> = (0..n)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                ((seed as f64 / u32::MAX as f64 - 0.5) * 0.4) as f32
            })
            .collect();
        let front = MelFrontEnd::new();
        let total = num_mel_frames(n);
        let valid = n / HOP;
        let whole = front.mel_frames(&pcm, 0, 0, total, valid);
        assert_eq!(whole.len(), total * N_MELS);
        assert!(whole[(total - 1) * N_MELS..].iter().all(|&v| v == 0.0));
        for (a, b) in [
            (0, 1),
            (0, 100),
            (37, 140),
            (150, total),
            (total - 3, total),
        ] {
            let part = front.mel_frames(&pcm, 0, a, b, valid);
            // and from a buffer that starts exactly where the slice needs it
            let start = a.saturating_sub(MEL_MARGIN_FRAMES) * HOP;
            let trimmed = front.mel_frames(&pcm[start..], start, a, b, valid);
            let expect = &whole[a * N_MELS..b * N_MELS];
            let max_diff = part
                .iter()
                .zip(expect)
                .map(|(x, y)| (x - y).abs())
                .fold(0f32, f32::max);
            assert!(max_diff < 1e-4, "[{a}, {b}) max diff {max_diff}");
            assert_eq!(part, trimmed);
        }
    }
}
