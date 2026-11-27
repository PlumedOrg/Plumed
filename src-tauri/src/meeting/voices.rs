//! Recognizes the voices of a meeting (Plumed fork, written with Claude
//! Code). The diarization sometimes gives one person two speaker slots, and
//! a resumed meeting starts with new slots. Each slot collects the audio
//! where it speaks alone, and its CAM++ embedding (`diarization::voice`) is
//! compared with the other slots and the voices of the resumed transcript:
//! a match means the same person, so the caller merges the two.
//!
//! Calibration on 10 French FLEURS speakers (5 to 15 s each): two clips of
//! one speaker score 0.62 to 0.95 (0.56 for 5 s halves), two speakers at
//! most 0.46. So 0.6 with at least 4 s of speech.
//!
//! The embeddings stay in memory: nothing about the voices is saved.

use super::transcript::Line;
use crate::diarization::voice::{similarity, VoiceModel};
use anyhow::Result;
use log::warn;
use std::path::Path;

pub const SAMPLE_RATE: usize = 16000;
/// Samples per row of speaker probabilities.
const HOP: usize = 160;
/// Same person at or above this cosine similarity.
pub const SAME_VOICE: f32 = 0.6;
/// A slot is embedded again when its audio reaches each of these lengths.
const EMBED_AT_SECS: [usize; 4] = [4, 8, 16, 30];
const MAX_SAMPLES: usize = 30 * SAMPLE_RATE;
const MIN_SAMPLES: usize = 4 * SAMPLE_RATE;
/// A slot speaks alone in a row when it is the only one this probable.
const ACTIVE: f32 = 0.5;

/// Turns audio into a normalized voice embedding: CAM++, or a fake in tests.
pub trait Embedder {
    fn embed(&mut self, pcm: &[f32]) -> Result<Vec<f32>>;
}

impl Embedder for VoiceModel {
    fn embed(&mut self, pcm: &[f32]) -> Result<Vec<f32>> {
        VoiceModel::embed(self, pcm)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Another diarization slot.
    Slot(usize),
    /// A voice of the resumed transcript.
    Voice(usize),
}

/// `slot` sounds like `like`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub slot: usize,
    pub like: Target,
}

#[derive(Default)]
struct Slot {
    audio: Vec<f32>,
    embedding: Option<Vec<f32>>,
}

pub struct VoiceTracker<E> {
    embedder: E,
    num_speakers: usize,
    /// Meeting audio not yet covered by speaker rows.
    audio: Vec<f32>,
    /// Timeline sample of `audio[0]`.
    audio_start: usize,
    /// Rows before this frame were seen (a paused meeting's provisional rows
    /// count: the real ones that replace them are skipped).
    next_frame: usize,
    slots: Vec<Slot>,
    /// Embeddings of the resumed transcript's voices.
    known: Vec<(usize, Vec<f32>)>,
}

impl<E: Embedder> VoiceTracker<E> {
    /// A tracker whose audio and rows start at `frame`.
    pub fn new(embedder: E, num_speakers: usize, frame: usize) -> Self {
        Self {
            embedder,
            num_speakers,
            audio: Vec::new(),
            audio_start: frame * HOP,
            next_frame: frame,
            slots: (0..num_speakers).map(|_| Slot::default()).collect(),
            known: Vec::new(),
        }
    }

    /// Learns the voices of a resumed transcript from its backed up audio.
    pub fn learn_voices(&mut self, wav: &Path, lines: &[Line]) {
        for (voice, audio) in voice_audio(wav, lines) {
            match self.embedder.embed(&audio) {
                Ok(embedding) => self.known.push((voice, embedding)),
                Err(e) => warn!("Voice {voice} of the resumed meeting not embedded: {e:#}"),
            }
        }
    }

    /// The next meeting audio, 16 kHz.
    pub fn add_audio(&mut self, samples: &[f32]) {
        self.audio.extend_from_slice(samples);
    }

    /// Speaker rows (`[frames][num_speakers]`) from `frame` on. Returns the
    /// slots that now sound like another slot or a known voice.
    pub fn add_rows(&mut self, frame: usize, rows: &[f32]) -> Vec<Match> {
        let mut grown = vec![false; self.num_speakers];
        for (i, row) in rows.chunks_exact(self.num_speakers).enumerate() {
            let f = frame + i;
            if f < self.next_frame {
                continue;
            }
            self.next_frame = f + 1;
            let mut active = row.iter().enumerate().filter(|(_, &p)| p >= ACTIVE);
            let (Some((slot, _)), None) = (active.next(), active.next()) else {
                continue;
            };
            let Some(start) = (f * HOP).checked_sub(self.audio_start) else {
                continue;
            };
            let (Some(samples), audio) = (self.audio.get(start..start + HOP), &mut self.slots[slot].audio)
            else {
                continue;
            };
            if audio.len() >= MAX_SAMPLES {
                continue;
            }
            let before = audio.len();
            audio.extend_from_slice(samples);
            grown[slot] |= EMBED_AT_SECS
                .iter()
                .any(|&secs| (before + 1..=audio.len()).contains(&(secs * SAMPLE_RATE)));
        }
        let seen = (self.next_frame * HOP).saturating_sub(self.audio_start).min(self.audio.len());
        self.audio.drain(..seen);
        self.audio_start += seen;

        let mut matches = Vec::new();
        for slot in (0..self.num_speakers).filter(|&s| grown[s]) {
            match self.embedder.embed(&self.slots[slot].audio) {
                Ok(embedding) => self.slots[slot].embedding = Some(embedding),
                Err(e) => {
                    warn!("Voice of speaker slot {slot} not embedded: {e:#}");
                    continue;
                }
            }
            matches.extend(self.matches_of(slot));
        }
        matches
    }

    fn matches_of(&self, slot: usize) -> Vec<Match> {
        let Some(embedding) = &self.slots[slot].embedding else {
            return Vec::new();
        };
        let same = |other: &[f32]| similarity(embedding, other) >= SAME_VOICE;
        let slots = self.slots.iter().enumerate().filter_map(|(other, s)| {
            (other != slot && s.embedding.as_deref().is_some_and(same)).then_some(Target::Slot(other))
        });
        let voices = self
            .known
            .iter()
            .filter(|(_, e)| same(e))
            .map(|&(voice, _)| Target::Voice(voice));
        slots.chain(voices).map(|like| Match { slot, like }).collect()
    }

    /// The voice `from` was merged into `into`.
    pub fn merged(&mut self, from: usize, into: usize) {
        for (voice, _) in &mut self.known {
            if *voice == from {
                *voice = into;
            }
        }
    }
}

/// Up to 30 s of each voice of `lines` from the meeting's 16 kHz WAV, its
/// latest lines first; voices with under 4 s are left out.
fn voice_audio(wav: &Path, lines: &[Line]) -> Vec<(usize, Vec<f32>)> {
    let mut reader = match hound::WavReader::open(wav) {
        Ok(reader) if reader.spec().sample_rate as usize == SAMPLE_RATE && reader.spec().channels == 1 => reader,
        Ok(_) => return Vec::new(),
        Err(e) => {
            warn!("Backed up meeting audio unreadable: {e}");
            return Vec::new();
        }
    };
    let mut voices: Vec<(usize, Vec<f32>)> = Vec::new();
    for line in lines.iter().rev() {
        let index = match voices.iter().position(|(v, _)| *v == line.speaker) {
            Some(index) => index,
            None => {
                voices.push((line.speaker, Vec::new()));
                voices.len() - 1
            }
        };
        let audio = &mut voices[index].1;
        let start = (line.start.max(0.0) * SAMPLE_RATE as f64) as u32;
        let len = ((line.end - line.start).max(0.0) * SAMPLE_RATE as f64) as usize;
        let len = len.min(MAX_SAMPLES - audio.len());
        if len == 0 || reader.seek(start).is_err() {
            continue;
        }
        audio.extend(
            reader
                .samples::<i16>()
                .take(len)
                .map_while(Result::ok)
                .map(|s| s as f32 / i16::MAX as f32),
        );
    }
    voices.retain(|(_, audio)| audio.len() >= MIN_SAMPLES);
    voices.sort_by_key(|(voice, _)| *voice);
    voices
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "Embeds" a voice by the level of its audio: 0.1 and -0.1 are two
    /// voices (cosine -0.6), equal levels are the same voice.
    struct LevelEmbedder {
        calls: usize,
    }

    impl Embedder for LevelEmbedder {
        fn embed(&mut self, pcm: &[f32]) -> Result<Vec<f32>> {
            self.calls += 1;
            let mean = pcm.iter().sum::<f32>() / pcm.len() as f32;
            let norm = (mean * mean + 0.0025).sqrt();
            Ok(vec![mean / norm, 0.05 / norm])
        }
    }

    fn tracker() -> VoiceTracker<LevelEmbedder> {
        VoiceTracker::new(LevelEmbedder { calls: 0 }, 3, 0)
    }

    /// `secs` of rows where only `slot` (of 3) speaks.
    fn rows(slot: usize, secs: usize) -> Vec<f32> {
        let mut row = [0.1f32; 3];
        row[slot] = 0.9;
        row.repeat(secs * 100)
    }

    fn level(value: f32, secs: usize) -> Vec<f32> {
        vec![value; secs * SAMPLE_RATE]
    }

    #[test]
    fn a_slot_with_the_voice_of_another_matches_it() {
        let mut voices = tracker();
        voices.add_audio(&level(0.1, 5));
        voices.add_audio(&level(-0.1, 5));
        voices.add_audio(&level(0.1, 5));
        let mut all = rows(0, 5);
        all.extend(rows(1, 5));
        all.extend(rows(2, 3));
        assert!(voices.add_rows(0, &all).is_empty(), "two voices, slot 2 under 4 s");
        assert_eq!(
            voices.add_rows(1300, &rows(2, 2)),
            vec![Match {
                slot: 2,
                like: Target::Slot(0)
            }]
        );
        assert_eq!(voices.embedder.calls, 3);
        assert!(voices.audio.is_empty(), "audio covered by rows is dropped");
    }

    #[test]
    fn overlapping_speech_and_rows_seen_before_are_left_out() {
        let mut voices = tracker();
        voices.add_audio(&level(0.1, 10));
        let mut both = [0.9f32, 0.9, 0.0].repeat(500);
        both.extend(rows(0, 3));
        assert!(voices.add_rows(0, &both).is_empty());
        assert_eq!(voices.slots[1].audio.len(), 0);
        // provisional rows of a pause, then the real ones again: counted once
        assert_eq!(voices.slots[0].audio.len(), 3 * SAMPLE_RATE);
        voices.add_rows(800, &rows(0, 2));
        assert_eq!(voices.slots[0].audio.len(), 5 * SAMPLE_RATE);
        voices.add_rows(800, &rows(0, 2));
        assert_eq!(voices.slots[0].audio.len(), 5 * SAMPLE_RATE);
        assert_eq!(voices.embedder.calls, 1);
    }

    #[test]
    fn a_slot_is_embedded_at_4_8_16_and_30_seconds_only() {
        let mut voices = tracker();
        for second in 0..40 {
            voices.add_audio(&level(0.1, 1));
            voices.add_rows(second * 100, &rows(0, 1));
        }
        assert_eq!(voices.embedder.calls, 4);
        assert_eq!(voices.slots[0].audio.len(), MAX_SAMPLES);
    }

    #[test]
    fn a_resumed_meeting_matches_the_voices_of_its_backed_up_audio() {
        let wav = std::env::temp_dir().join(format!("plumed-voices-{}.wav", std::process::id()));
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&wav, spec).unwrap();
        for value in [-0.1f32, 0.1, -0.1] {
            for _ in 0..5 * SAMPLE_RATE {
                writer.write_sample((value * i16::MAX as f32) as i16).unwrap();
            }
        }
        writer.finalize().unwrap();
        let line = |speaker, start: f64, end: f64| Line {
            speaker,
            start,
            end,
            text: String::new(),
        };
        // voice 1 gets 2 + 3 s from two lines, voice 0 only 2 s
        let lines = [line(1, 1.0, 3.0), line(0, 6.0, 8.0), line(1, 10.0, 13.0)];
        let audio = voice_audio(&wav, &lines);
        assert_eq!(audio.len(), 1);
        assert_eq!(audio[0].0, 1);
        assert_eq!(audio[0].1.len(), 5 * SAMPLE_RATE);
        assert!(audio[0].1.iter().all(|&s| (s + 0.1).abs() < 1e-3));

        let mut voices = VoiceTracker::new(LevelEmbedder { calls: 0 }, 3, 1500);
        voices.learn_voices(&wav, &lines);
        voices.merged(1, 0);
        voices.add_audio(&level(-0.1, 4));
        assert_eq!(
            voices.add_rows(1500, &rows(2, 4)),
            vec![Match {
                slot: 2,
                like: Target::Voice(0)
            }]
        );
        std::fs::remove_file(wav).unwrap();
    }
}
