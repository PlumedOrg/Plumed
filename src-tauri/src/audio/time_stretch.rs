// Plumed fork (written with Claude Code): slows speech down without changing its
// pitch (WSOLA), for the "slow down the audio" setting. A fast speaker is then
// heard at a pace closer to the model's training data.

/// Playback speed of the "slow down the audio" setting: the audio lasts 1.25x longer.
pub const SLOW_DOWN_SPEED: f32 = 0.8;

const FRAME: usize = 400; // 25 ms at 16 kHz
const HOP_OUT: usize = FRAME / 2;
const TOLERANCE: usize = 80; // 5 ms at 16 kHz: how far a frame may move to stay in phase

/// Returns `samples` played at `speed` (0.8 = slower, the output is 1.25x longer)
/// with the same pitch. Each output frame is taken near its nominal input
/// position, shifted by up to `TOLERANCE` samples to best continue the previous
/// frame, then overlap-added with a Hann window.
pub fn time_stretch(samples: &[f32], speed: f32) -> Vec<f32> {
    if samples.len() < FRAME + 2 * TOLERANCE || (speed - 1.0).abs() < f32::EPSILON {
        return samples.to_vec();
    }
    let hop_in = HOP_OUT as f32 * speed;
    let out_len = (samples.len() as f32 / speed) as usize;
    let window: Vec<f32> = (0..FRAME)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / FRAME as f32).cos())
        .collect();
    let mut out = vec![0.0f32; out_len + FRAME];
    let mut weight = vec![0.0f32; out_len + FRAME];
    let last_start = samples.len() - FRAME;
    let mut previous: Option<usize> = None;
    let mut k = 0;
    while k * HOP_OUT < out_len {
        let nominal = ((k as f32 * hop_in) as usize).min(last_start);
        let start = match previous {
            Some(prev) if prev + HOP_OUT <= last_start => {
                best_offset(samples, prev + HOP_OUT, nominal, last_start)
            }
            _ => nominal,
        };
        let at = k * HOP_OUT;
        for i in 0..FRAME {
            out[at + i] += samples[start + i] * window[i];
            weight[at + i] += window[i];
        }
        previous = Some(start);
        k += 1;
    }
    out.truncate(out_len);
    for (sample, w) in out.iter_mut().zip(weight) {
        if w > 1e-3 {
            *sample /= w;
        }
    }
    out
}

/// Start, within `TOLERANCE` of `nominal`, of the input frame most correlated
/// with the natural continuation of the previous frame (starting at `target`).
fn best_offset(samples: &[f32], target: usize, nominal: usize, last_start: usize) -> usize {
    let reference = &samples[target..target + FRAME];
    let lo = nominal.saturating_sub(TOLERANCE);
    let hi = (nominal + TOLERANCE).min(last_start);
    (lo..=hi)
        .map(|s| {
            let score: f32 = reference
                .iter()
                .zip(&samples[s..s + FRAME])
                .map(|(x, y)| x * y)
                .sum();
            (s, score)
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(nominal, |(s, _)| s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 16000.0;

    fn sine(freq: f32, secs: f32) -> Vec<f32> {
        (0..(secs * SR) as usize)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / SR).sin() * 0.5)
            .collect()
    }

    /// Frequency estimated from the zero crossings, away from the edges.
    fn frequency(samples: &[f32]) -> f32 {
        let middle = &samples[samples.len() / 4..samples.len() * 3 / 4];
        let crossings = middle
            .windows(2)
            .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
            .count();
        crossings as f32 / (middle.len() as f32 / SR)
    }

    #[test]
    fn slowing_down_lengthens_and_keeps_the_pitch() {
        let input = sine(220.0, 2.0);
        let output = time_stretch(&input, SLOW_DOWN_SPEED);
        assert_eq!(output.len(), 40000); // 2 s * 1.25
        let f = frequency(&output);
        assert!((f - 220.0).abs() < 5.0, "pitch moved to {f} Hz");
        let peak = output[4000..36000]
            .iter()
            .fold(0.0f32, |m, x| m.max(x.abs()));
        assert!((peak - 0.5).abs() < 0.08, "amplitude changed to {peak}");
    }

    #[test]
    fn normal_speed_and_short_audio_are_untouched() {
        let input = sine(220.0, 1.0);
        assert_eq!(time_stretch(&input, 1.0), input);
        let short = sine(220.0, 0.02);
        assert_eq!(time_stretch(&short, SLOW_DOWN_SPEED), short);
    }
}
