//! The meeting transcript: one line per speaker turn.

use super::words::TimedWord;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Line {
    /// Voice id: 0 is shown as A, 1 as B, ... (gaps after merges).
    pub speaker: usize,
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Debug, Default, Clone)]
pub struct Transcript {
    pub lines: Vec<Line>,
}

impl Transcript {
    /// Appends `word` to the last line, or starts a line when the speaker
    /// changes. Returns the index of the line it went to.
    pub fn push(&mut self, speaker: usize, word: &TimedWord) -> usize {
        self.append(Line {
            speaker,
            start: word.start,
            end: word.end,
            text: word.text.clone(),
        });
        self.lines.len() - 1
    }

    /// `line` continues the last line when the speaker is the same.
    fn append(&mut self, line: Line) {
        match self.lines.last_mut() {
            Some(last) if last.speaker == line.speaker => {
                last.text.push(' ');
                last.text.push_str(&line.text);
                last.end = line.end;
            }
            _ => self.lines.push(line),
        }
    }

    /// The voice `from` turned out to be `into`: its lines get relabeled and
    /// the turns of `into` that now follow each other become one line.
    pub fn relabel(&mut self, from: usize, into: usize) {
        for line in std::mem::take(&mut self.lines) {
            let speaker = if line.speaker == from { into } else { line.speaker };
            self.append(Line { speaker, ..line });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(start: f64, text: &str) -> TimedWord {
        TimedWord {
            start,
            end: start + 0.3,
            text: text.into(),
        }
    }

    #[test]
    fn consecutive_words_of_a_speaker_share_a_line() {
        let mut transcript = Transcript::default();
        assert_eq!(transcript.push(0, &word(0.0, "Bonjour")), 0);
        assert_eq!(transcript.push(0, &word(0.5, "Paul.")), 0);
        assert_eq!(transcript.push(1, &word(1.0, "Salut.")), 1);
        assert_eq!(transcript.push(0, &word(2.0, "Alors")), 2);
        let texts: Vec<_> = transcript
            .lines
            .iter()
            .map(|l| (l.speaker, l.text.as_str()))
            .collect();
        assert_eq!(
            texts,
            vec![(0, "Bonjour Paul."), (1, "Salut."), (0, "Alors")]
        );
        assert!((transcript.lines[0].end - 0.8).abs() < 1e-9);
    }

    #[test]
    fn relabeling_a_voice_joins_the_turns_it_splits() {
        let mut transcript = Transcript::default();
        transcript.push(0, &word(0.0, "Bonjour"));
        transcript.push(2, &word(1.0, "Paul."));
        transcript.push(1, &word(2.0, "Salut."));
        transcript.push(2, &word(3.0, "Alors"));
        transcript.relabel(2, 0);
        let texts: Vec<_> = transcript
            .lines
            .iter()
            .map(|l| (l.speaker, l.text.as_str(), l.start))
            .collect();
        assert_eq!(
            texts,
            vec![(0, "Bonjour Paul.", 0.0), (1, "Salut.", 2.0), (0, "Alors", 3.0)]
        );
        assert!((transcript.lines[0].end - 1.3).abs() < 1e-9);
    }
}
