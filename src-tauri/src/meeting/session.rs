//! The state of one meeting, independent of the audio source and the models:
//! chunk words and speaker probabilities come in, transcript lines go out.

use super::speakers::SpeakerTrack;
use super::transcript::{Line, Transcript};
use super::words::TimedWord;

/// How long the end of each chunk is held back: a forced cut repeats this much
/// audio at the head of the next chunk (see `audio::chunking`), whose words
/// replace the held ones.
const HOLD_SECS: f64 = 1.0;

pub struct MeetingCore {
    /// Transcribed words not yet in the transcript, in time order.
    pending: Vec<TimedWord>,
    /// Pending words starting from here may still be replaced by the next chunk.
    stable_until: f64,
    finished: bool,
    speakers: SpeakerTrack,
    last_speaker: usize,
    pub transcript: Transcript,
}

impl MeetingCore {
    pub fn new(speakers: SpeakerTrack) -> Self {
        Self {
            pending: Vec::new(),
            stable_until: 0.0,
            finished: false,
            speakers,
            last_speaker: 0,
            transcript: Transcript::default(),
        }
    }

    /// Continues a backed up transcript: the next audio comes from frame
    /// `frame` of the timeline on.
    pub fn resume(&mut self, lines: Vec<Line>, frame: usize) {
        self.last_speaker = lines.last().map_or(0, |line| line.speaker);
        self.transcript.lines = lines;
        self.speakers.skip_to(frame);
    }

    /// Adds the words of the chunk `[chunk_start, chunk_end)`, whose first
    /// `overlap_secs` repeat the end of the previous chunk. The two chunks
    /// split the overlap in its middle, so a word cut by the boundary of one
    /// is taken whole from the other.
    pub fn add_chunk(
        &mut self,
        words: Vec<TimedWord>,
        chunk_start: f64,
        chunk_end: f64,
        overlap_secs: f64,
    ) {
        let boundary = chunk_start + overlap_secs / 2.0;
        self.pending.retain(|w| w.start < boundary);
        self.pending
            .extend(words.into_iter().filter(|w| w.start >= boundary));
        self.stable_until = chunk_end - HOLD_SECS;
        self.finished = false;
    }

    /// Speaker rows from absolute row `frame` on (see `SpeakerTrack::push_at`).
    pub fn add_speaker_probs(&mut self, frame: usize, rows: &[f32]) {
        self.speakers.push_at(frame, rows);
    }

    /// The audio has ended (for now): no chunk will replace the held words.
    /// A later recording continues the meeting through `add_chunk`.
    pub fn finish(&mut self) {
        self.finished = true;
    }

    /// Moves to the transcript every pending word that is settled (not held
    /// back) and whose speakers are known. Returns the indices of the lines
    /// that changed, in increasing order.
    pub fn release(&mut self) -> Vec<usize> {
        let covered = self.speakers.covered_secs();
        let settled = self
            .pending
            .iter()
            .take_while(|w| (self.finished || w.start < self.stable_until) && w.end <= covered)
            .count();
        let mut changed: Vec<usize> = Vec::new();
        for word in self.pending.drain(..settled) {
            let speaker = self
                .speakers
                .speaker_of(word.start, word.end)
                .unwrap_or(self.last_speaker);
            self.last_speaker = speaker;
            let line = self.transcript.push(speaker, &word);
            if changed.last() != Some(&line) {
                changed.push(line);
            }
        }
        let oldest_needed = self.pending.first().map_or(covered, |w| w.start);
        self.speakers.drop_before(oldest_needed);
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: f64 = 0.1;

    fn word(start: f64, text: &str) -> TimedWord {
        TimedWord {
            start,
            end: start + 0.4,
            text: text.into(),
        }
    }

    /// Rows for `secs` seconds where only `speaker` (of 2) talks.
    fn rows(speaker: usize, secs: f64) -> Vec<f32> {
        let mut row = [0.0f32; 2];
        row[speaker] = 0.9;
        (0..(secs / FRAME).round() as usize)
            .flat_map(|_| row)
            .collect()
    }

    fn texts(core: &MeetingCore) -> Vec<(usize, &str)> {
        core.transcript
            .lines
            .iter()
            .map(|l| (l.speaker, l.text.as_str()))
            .collect()
    }

    #[test]
    fn words_wait_for_the_diarization_then_split_by_speaker() {
        let mut core = MeetingCore::new(SpeakerTrack::new(2, FRAME));
        core.add_chunk(
            vec![
                word(0.5, "Bonjour."),
                word(2.5, "Salut."),
                word(3.5, "Ça va ?"),
            ],
            0.0,
            10.0,
            0.0,
        );
        assert!(core.release().is_empty()); // no speakers known yet
        core.add_speaker_probs(0, &rows(0, 2.0));
        assert_eq!(core.release(), vec![0]);
        assert_eq!(texts(&core), vec![(0, "Bonjour.")]);
        core.add_speaker_probs(20, &rows(1, 3.0));
        assert_eq!(core.release(), vec![1]);
        assert_eq!(texts(&core), vec![(0, "Bonjour."), (1, "Salut. Ça va ?")]);
    }

    #[test]
    fn a_word_in_silence_goes_to_the_previous_speaker() {
        let mut core = MeetingCore::new(SpeakerTrack::new(2, FRAME));
        core.add_chunk(vec![word(1.5, "Paul"), word(2.5, "hum")], 0.0, 10.0, 0.0);
        let mut probs = rows(1, 2.0);
        probs.extend(vec![0.0; 20 * 2]); // 2 s of silence
        core.add_speaker_probs(0, &probs);
        core.release();
        assert_eq!(texts(&core), vec![(1, "Paul hum")]);
    }

    #[test]
    fn the_end_of_a_chunk_waits_for_the_next_one_or_the_finish() {
        let mut core = MeetingCore::new(SpeakerTrack::new(2, FRAME));
        core.add_speaker_probs(0, &rows(0, 30.0));
        core.add_chunk(vec![word(1.0, "un"), word(9.5, "deux")], 0.0, 10.0, 0.0);
        core.release();
        assert_eq!(texts(&core), vec![(0, "un")]);
        core.finish();
        core.release();
        assert_eq!(texts(&core), vec![(0, "un deux")]);
    }

    #[test]
    fn a_forced_cut_overlap_keeps_each_word_once() {
        let mut core = MeetingCore::new(SpeakerTrack::new(2, FRAME));
        core.add_speaker_probs(0, &rows(0, 30.0));
        // the first chunk ends mid-word at 10 s; the second starts at 9 s
        core.add_chunk(
            vec![word(8.0, "avant"), word(9.2, "pareil"), word(9.8, "coup")],
            0.0,
            10.0,
            0.0,
        );
        core.add_chunk(
            vec![word(9.2, "pareil"), word(9.8, "coupé"), word(11.0, "après")],
            9.0,
            20.0,
            1.0,
        );
        core.finish();
        core.release();
        assert_eq!(texts(&core), vec![(0, "avant pareil coupé après")]);
    }

    #[test]
    fn a_resumed_transcript_goes_on_from_its_frame() {
        let mut core = MeetingCore::new(SpeakerTrack::new(2, FRAME));
        let before = Line {
            speaker: 1,
            start: 0.0,
            end: 99.0,
            text: "Avant la coupure.".into(),
        };
        core.resume(vec![before], 1000); // 100 s
        core.add_chunk(
            vec![word(100.5, "Après."), word(102.5, "Oui.")],
            100.0,
            110.0,
            0.0,
        );
        core.add_speaker_probs(1000, &rows(0, 2.0));
        core.add_speaker_probs(1020, &rows(1, 0.5));
        assert_eq!(core.release(), vec![1]);
        // a word without its frames yet waits
        assert_eq!(texts(&core), vec![(1, "Avant la coupure."), (0, "Après.")]);
    }
}
