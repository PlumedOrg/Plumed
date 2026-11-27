//! Words with their time on the meeting timeline.

use crate::engine::transcription_engine::TranscriptionSegment;

/// A recognized word, in seconds from the start of the meeting.
#[derive(Debug, Clone, PartialEq)]
pub struct TimedWord {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

/// Places a chunk's word segments on the meeting timeline. The engine saw the
/// chunk with `pad_secs` of leading silence, played at `speed` (below 1 when
/// the "slow down the audio" setting stretched it), so a time `t` in its
/// output is `chunk_start + (t - pad_secs) * speed` in the meeting.
pub fn chunk_words(
    segments: &[TranscriptionSegment],
    chunk_start: f64,
    pad_secs: f64,
    speed: f64,
) -> Vec<TimedWord> {
    let place = |t: f32| chunk_start + ((t as f64 - pad_secs) * speed).max(0.0);
    segments
        .iter()
        .filter(|s| !s.text.trim().is_empty())
        .map(|s| TimedWord {
            start: place(s.start),
            end: place(s.end),
            text: s.text.trim().to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(start: f32, end: f32, text: &str) -> TranscriptionSegment {
        TranscriptionSegment {
            start,
            end,
            text: text.to_string(),
        }
    }

    #[test]
    fn words_are_shifted_by_the_chunk_start_and_unpadded() {
        let words = chunk_words(
            &[segment(0.25, 0.75, " Bonjour,"), segment(1.25, 1.5, "Paul")],
            100.0,
            0.25,
            1.0,
        );
        assert_eq!(
            words,
            vec![
                TimedWord {
                    start: 100.0,
                    end: 100.5,
                    text: "Bonjour,".into()
                },
                TimedWord {
                    start: 101.0,
                    end: 101.25,
                    text: "Paul".into()
                },
            ]
        );
    }

    #[test]
    fn slowed_down_audio_maps_back_to_real_time() {
        // 2 s into audio slowed to 0.8x is 1.6 s of real speech
        let words = chunk_words(&[segment(2.25, 2.75, "oui")], 10.0, 0.25, 0.8);
        assert!((words[0].start - 11.6).abs() < 1e-6);
        assert!((words[0].end - 12.0).abs() < 1e-6);
    }

    #[test]
    fn blank_segments_are_dropped_and_padding_clamps_to_the_chunk_start() {
        let words = chunk_words(
            &[segment(0.1, 0.3, "euh"), segment(0.3, 0.4, " ")],
            5.0,
            0.25,
            1.0,
        );
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].start, 5.0);
    }
}
