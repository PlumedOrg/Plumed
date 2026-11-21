//! Aggregated usage counters of the current period: sums only, never one
//! record per dictation and never a dictation timestamp.

use serde::{Deserialize, Serialize};

/// Upper bounds (seconds) of the audio duration buckets; the last bucket
/// takes everything above the last bound.
const AUDIO_BUCKET_BOUNDS: [f64; 3] = [10.0, 30.0, 120.0];

/// The errors counted where they happen once (see `telemetry::count_error`).
#[derive(Clone, Copy, Debug)]
pub enum ErrorKind {
    ModelLoad,
    Microphone,
    Paste,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct ErrorCounts {
    pub model_load: u64,
    pub microphone: u64,
    pub paste: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Counters {
    pub transcriptions: u64,
    pub audio_secs_sum: f64,
    pub audio_secs_sq_sum: f64,
    /// Transcriptions per duration: <10 s, 10-30 s, 30-120 s, >120 s.
    pub audio_buckets: [u64; 4],
    pub chars_sum: f64,
    pub chars_sq_sum: f64,
    /// Sum of processing time / audio duration, one term per transcription.
    pub rtf_sum: f64,
    pub errors: ErrorCounts,
}

/// Index in `audio_buckets` of a dictation lasting `secs`.
pub fn audio_bucket(secs: f64) -> usize {
    AUDIO_BUCKET_BOUNDS
        .iter()
        .position(|bound| secs < *bound)
        .unwrap_or(AUDIO_BUCKET_BOUNDS.len())
}

/// Mean and population standard deviation of `n` values given their sum and
/// sum of squares, `(0, 0)` without values.
pub fn mean_stdev(n: u64, sum: f64, sq_sum: f64) -> (f64, f64) {
    if n == 0 {
        return (0.0, 0.0);
    }
    let n = n as f64;
    let mean = sum / n;
    let variance = (sq_sum / n - mean * mean).max(0.0);
    (mean, variance.sqrt())
}

impl Counters {
    pub fn add_transcription(&mut self, audio_secs: f64, chars: usize, processing_secs: f64) {
        let chars = chars as f64;
        self.transcriptions += 1;
        self.audio_secs_sum += audio_secs;
        self.audio_secs_sq_sum += audio_secs * audio_secs;
        self.audio_buckets[audio_bucket(audio_secs)] += 1;
        self.chars_sum += chars;
        self.chars_sq_sum += chars * chars;
        if audio_secs > 0.0 {
            self.rtf_sum += processing_secs / audio_secs;
        }
    }

    pub fn add_error(&mut self, kind: ErrorKind) {
        match kind {
            ErrorKind::ModelLoad => self.errors.model_load += 1,
            ErrorKind::Microphone => self.errors.microphone += 1,
            ErrorKind::Paste => self.errors.paste += 1,
        }
    }

    /// Nothing happened during the period (no dictation, no error).
    pub fn is_empty(&self) -> bool {
        self.transcriptions == 0 && self.errors == ErrorCounts::default()
    }

    /// Removes what was `sent`, keeping what was counted while it was sent.
    pub fn subtract(&mut self, sent: &Counters) {
        let sub = |a: f64, b: f64| (a - b).max(0.0);
        self.transcriptions = self.transcriptions.saturating_sub(sent.transcriptions);
        self.audio_secs_sum = sub(self.audio_secs_sum, sent.audio_secs_sum);
        self.audio_secs_sq_sum = sub(self.audio_secs_sq_sum, sent.audio_secs_sq_sum);
        for (bucket, sent_bucket) in self.audio_buckets.iter_mut().zip(sent.audio_buckets) {
            *bucket = bucket.saturating_sub(sent_bucket);
        }
        self.chars_sum = sub(self.chars_sum, sent.chars_sum);
        self.chars_sq_sum = sub(self.chars_sq_sum, sent.chars_sq_sum);
        self.rtf_sum = sub(self.rtf_sum, sent.rtf_sum);
        self.errors.model_load = self
            .errors
            .model_load
            .saturating_sub(sent.errors.model_load);
        self.errors.microphone = self
            .errors
            .microphone
            .saturating_sub(sent.errors.microphone);
        self.errors.paste = self.errors.paste.saturating_sub(sent.errors.paste);
        if self.transcriptions == 0 {
            // Float sums may keep a rounding residue: reset them exactly.
            *self = Counters {
                errors: self.errors.clone(),
                ..Counters::default()
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_split_at_10_30_and_120_seconds() {
        assert_eq!(audio_bucket(0.0), 0);
        assert_eq!(audio_bucket(9.9), 0);
        assert_eq!(audio_bucket(10.0), 1);
        assert_eq!(audio_bucket(29.9), 1);
        assert_eq!(audio_bucket(30.0), 2);
        assert_eq!(audio_bucket(119.9), 2);
        assert_eq!(audio_bucket(120.0), 3);
        assert_eq!(audio_bucket(3600.0), 3);
    }

    #[test]
    fn mean_and_stdev_come_from_the_sums() {
        // Values 2, 4, 4, 4, 5, 5, 7, 9: mean 5, population stdev 2.
        let values = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let sum: f64 = values.iter().sum();
        let sq_sum: f64 = values.iter().map(|v| v * v).sum();
        let (mean, stdev) = mean_stdev(values.len() as u64, sum, sq_sum);
        assert!((mean - 5.0).abs() < 1e-9);
        assert!((stdev - 2.0).abs() < 1e-9);
        assert_eq!(mean_stdev(0, 0.0, 0.0), (0.0, 0.0));
    }

    #[test]
    fn subtracting_what_was_sent_keeps_later_counts() {
        let mut sent = Counters::default();
        sent.add_transcription(5.0, 40, 2.5);
        sent.add_error(ErrorKind::Paste);
        let mut current = sent.clone();
        current.add_transcription(40.0, 300, 10.0);
        current.subtract(&sent);
        let mut expected = Counters::default();
        expected.add_transcription(40.0, 300, 10.0);
        assert_eq!(current, expected);

        current.subtract(&expected);
        assert!(current.is_empty());
    }
}
