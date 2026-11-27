//! Per-frame speaker probabilities to speaker turns (Plumed fork), a port of
//! parakeet_web's `probsToSegments`.

use super::types::Segment;

#[derive(Debug, Clone, Copy)]
pub struct SegmentOptions {
    pub threshold: f32,
    /// 0 keeps every detected speaker.
    pub max_speakers: usize,
    pub min_duration_on: f64,
    pub min_duration_off: f64,
}

impl Default for SegmentOptions {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            max_speakers: 0,
            min_duration_on: 0.3,
            min_duration_off: 0.5,
        }
    }
}

/// A frame counts as speech of a speaker when its probability exceeds
/// `threshold`. With `max_speakers` set and more speakers detected, the most
/// active ones are kept and each dropped speaker's frames go to the most
/// probable kept speaker. Per speaker, gaps shorter than `min_duration_off` are
/// bridged, then turns shorter than `min_duration_on` dropped. Speakers are
/// renumbered 0.. in the model's channel order (its order of first arrival).
/// `probs` is `[frames][num_speakers]`; `frame_sec` the frame duration.
pub fn probs_to_segments(
    probs: &[f32],
    num_speakers: usize,
    frame_sec: f64,
    options: SegmentOptions,
) -> Vec<Segment> {
    let s_count = num_speakers;
    let num_frames = probs.len() / s_count;
    let mut active: Vec<bool> = probs.iter().map(|&p| p > options.threshold).collect();
    let mut activity = vec![0usize; s_count];
    for (i, &a) in active.iter().enumerate() {
        if a {
            activity[i % s_count] += 1;
        }
    }
    let present: Vec<usize> = (0..s_count).filter(|&s| activity[s] > 0).collect();

    let mut kept = present.clone();
    if options.max_speakers > 0 && present.len() > options.max_speakers {
        kept.sort_by(|&a, &b| activity[b].cmp(&activity[a]).then(a.cmp(&b)));
        kept.truncate(options.max_speakers);
        kept.sort_unstable();
        for t in 0..num_frames {
            let base = t * s_count;
            let mut dropped = false;
            for &s in &present {
                if !kept.contains(&s) && active[base + s] {
                    active[base + s] = false;
                    dropped = true;
                }
            }
            if !dropped {
                continue;
            }
            let mut best = kept[0];
            for &s in &kept {
                if probs[base + s] > probs[base + best] {
                    best = s;
                }
            }
            active[base + best] = true;
        }
    }

    let min_on = options.min_duration_on / frame_sec;
    let min_off = options.min_duration_off / frame_sec;
    let mut segments = Vec::new();
    // labels go to speakers with a surviving turn only, so smoothing away a
    // speaker's every turn cannot leave a hole in the numbering
    let mut label = 0;
    for &s in &kept {
        let mut runs: Vec<(usize, usize)> = Vec::new();
        let mut t = 0;
        while t < num_frames {
            if !active[t * s_count + s] {
                t += 1;
                continue;
            }
            let mut e = t;
            while e < num_frames && active[e * s_count + s] {
                e += 1;
            }
            match runs.last_mut() {
                Some(last) if ((t - last.1) as f64) < min_off => last.1 = e,
                _ => runs.push((t, e)),
            }
            t = e;
        }
        let before = segments.len();
        for (a, b) in runs {
            if ((b - a) as f64) < min_on {
                continue;
            }
            segments.push(Segment {
                start: a as f64 * frame_sec,
                end: b as f64 * frame_sec,
                speaker: label,
            });
        }
        if segments.len() > before {
            label += 1;
        }
    }
    segments.sort_by(|x, y| {
        x.start
            .partial_cmp(&y.start)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(x.speaker.cmp(&y.speaker))
    });
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames(rows: &[[f32; 2]]) -> Vec<f32> {
        rows.iter().flatten().copied().collect()
    }

    #[test]
    fn bridges_short_gaps_drops_short_turns_and_renumbers() {
        // 0.1 s frames: speaker 1 talks first (0..0.5 s with a 0.1 s gap), then
        // speaker 0 for 0.2 s only, which is under min_duration_on
        let mut rows = vec![[0.0, 0.9]; 5];
        rows[2] = [0.0, 0.1];
        rows.extend([[0.9, 0.0]; 2]);
        let got = probs_to_segments(&frames(&rows), 2, 0.1, SegmentOptions::default());
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].speaker, 0);
        assert!((got[0].end - 0.5).abs() < 1e-9);
    }

    #[test]
    fn max_speakers_folds_the_least_active_into_the_most_probable() {
        let probs = [0.9, 0.0, 0.0, 0.9, 0.6, 0.7];
        let options = SegmentOptions {
            max_speakers: 1,
            min_duration_on: 0.0,
            min_duration_off: 0.0,
            ..Default::default()
        };
        // frames 0 and 2 for speaker 0, frame 1 for speaker 1 (only 1 frame)
        let got = probs_to_segments(&[0.9, 0.0, 0.0, 0.9, 0.9, 0.0], 2, 1.0, options);
        assert_eq!(
            got,
            vec![Segment {
                start: 0.0,
                end: 3.0,
                speaker: 0
            }]
        );
        // the dropped speaker's frame goes to the kept speaker
        let got = probs_to_segments(&probs, 3, 1.0, options);
        assert_eq!(
            got,
            vec![Segment {
                start: 0.0,
                end: 2.0,
                speaker: 0
            }]
        );
    }
}
