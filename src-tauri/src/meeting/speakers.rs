//! Speaker probabilities over the meeting timeline, from `SortformerStream`.

/// Mean probability below which a word is not attributed from its own frames
/// (the diarization heard no one): it then goes to the previous speaker.
const MIN_ACTIVITY: f32 = 0.2;

/// The rows of speaker probabilities received so far, `[frames][num_speakers]`
/// with one row per `frame_sec`. Rows before `first_frame` were dropped once no
/// word could need them any more, so memory stays flat over hours.
pub struct SpeakerTrack {
    num_speakers: usize,
    frame_sec: f64,
    first_frame: usize,
    probs: Vec<f32>,
}

impl SpeakerTrack {
    pub fn new(num_speakers: usize, frame_sec: f64) -> Self {
        Self {
            num_speakers,
            frame_sec,
            first_frame: 0,
            probs: Vec::new(),
        }
    }

    pub fn push(&mut self, rows: &[f32]) {
        self.probs.extend_from_slice(rows);
    }

    fn frames_end(&self) -> usize {
        self.first_frame + self.probs.len() / self.num_speakers
    }

    /// Time up to which the speakers are known.
    pub fn covered_secs(&self) -> f64 {
        self.frames_end() as f64 * self.frame_sec
    }

    /// The speaker with the most probability mass over `[start, end)`, or
    /// `None` when no one is heard there.
    pub fn speaker_of(&self, start: f64, end: f64) -> Option<usize> {
        let f0 = (start / self.frame_sec).floor() as usize;
        let f1 = ((end / self.frame_sec).ceil() as usize).max(f0 + 1);
        let (f0, f1) = (f0.max(self.first_frame), f1.min(self.frames_end()));
        if f0 >= f1 {
            return None;
        }
        let mut mass = vec![0.0f32; self.num_speakers];
        for f in f0..f1 {
            let row = (f - self.first_frame) * self.num_speakers;
            for (m, p) in mass
                .iter_mut()
                .zip(&self.probs[row..row + self.num_speakers])
            {
                *m += p;
            }
        }
        let (speaker, best) = mass.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1))?;
        (best / (f1 - f0) as f32 >= MIN_ACTIVITY).then_some(speaker)
    }

    /// Forgets the rows before `secs`.
    pub fn drop_before(&mut self, secs: f64) {
        let frame = ((secs / self.frame_sec).floor() as usize).min(self.frames_end());
        if frame > self.first_frame {
            self.probs
                .drain(..(frame - self.first_frame) * self.num_speakers);
            self.first_frame = frame;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2 speakers, 0.1 s rows: speaker 0 for 1 s, silence for 1 s, speaker 1 for 1 s.
    fn track() -> SpeakerTrack {
        let mut track = SpeakerTrack::new(2, 0.1);
        for _ in 0..10 {
            track.push(&[0.9, 0.05]);
        }
        for _ in 0..10 {
            track.push(&[0.0, 0.0]);
        }
        for _ in 0..10 {
            track.push(&[0.1, 0.8]);
        }
        track
    }

    #[test]
    fn a_word_goes_to_the_speaker_heard_during_it() {
        let track = track();
        assert!((track.covered_secs() - 3.0).abs() < 1e-9);
        assert_eq!(track.speaker_of(0.2, 0.6), Some(0));
        assert_eq!(track.speaker_of(2.1, 2.5), Some(1));
        // mostly speaker 1, straddling the silence
        assert_eq!(track.speaker_of(1.8, 2.6), Some(1));
    }

    #[test]
    fn silence_and_uncovered_time_have_no_speaker() {
        let track = track();
        assert_eq!(track.speaker_of(1.2, 1.8), None);
        assert_eq!(track.speaker_of(3.5, 4.0), None);
    }

    #[test]
    fn dropping_old_rows_keeps_later_answers_and_coverage() {
        let mut track = track();
        track.drop_before(2.0);
        assert!((track.covered_secs() - 3.0).abs() < 1e-9);
        assert_eq!(track.speaker_of(2.1, 2.5), Some(1));
        assert_eq!(track.speaker_of(0.2, 0.6), None);
    }
}
