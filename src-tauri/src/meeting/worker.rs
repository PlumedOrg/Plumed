//! Runs a meeting: consumes the `ChunkJob`s of any audio source (microphone
//! `Chunker`, PhoneMic, file), feeds the new audio to the diarization and the
//! whole chunk to the speech engine, and emits the transcript lines.

use super::backup::{Backup, Restored};
use super::session::MeetingCore;
use super::speakers::SpeakerTrack;
use super::transcript::Line;
use super::voices::VoiceTracker;
use super::words::chunk_words;
use crate::audio::chunking::ChunkJob;
use crate::audio::helpers::resample;
use crate::audio::pipeline::{
    pad_with_silence, slow_down_speed, with_engine, INFERENCE_SILENCE_PAD_SECS,
};
use crate::audio::time_stretch::time_stretch;
use crate::diarization::diarization::{DiarizationModel, SortformerStream, FRAME_SEC, SAMPLE_RATE};
use crate::diarization::voice::VoiceModel;
use crate::engine::transcription_engine::TranscriptionEngine;
use crate::engine::types::{ParakeetInferenceParams, TimestampGranularity};
use crate::model::catalog::{DIARIZATION, DIARIZATION_STEP, VOICE, VOICE_MODEL_FILE};
use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread::JoinHandle;
use tauri::{AppHandle, Emitter};

/// Transcript lines that were added or grew, by index.
pub const LINES_EVENT: &str = "meeting-lines";
/// The whole transcript, after voices were merged.
pub const LINES_RESET_EVENT: &str = "meeting-lines-reset";
/// Seconds of meeting audio processed so far.
pub const PROGRESS_EVENT: &str = "meeting-progress";
/// The recording ended and all its audio is transcribed.
pub const IDLE_EVENT: &str = "meeting-idle";

#[derive(Serialize, Clone)]
pub struct IndexedLine {
    pub index: usize,
    #[serde(flatten)]
    pub line: Line,
}

/// Shows a transcript line with the medical spelling fixes when they are on
/// (the only post-processing of a meeting). The transcript keeps the model's
/// words: a line still grows while the meeting goes on.
pub fn spelling(app: &AppHandle) -> impl Fn(&Line) -> Line {
    let fixes = crate::formatting_rules::load(app).is_ok_and(|s| s.built_in.medical_fixes);
    move |line: &Line| Line {
        text: if fixes {
            crate::medical::apply_medical_fixes(line.text.clone())
        } else {
            line.text.clone()
        },
        ..line.clone()
    }
}

/// One meeting, kept across its recordings: a new recording continues the
/// timeline and the speakers.
pub struct Meeting {
    pub core: MeetingCore,
    model: DiarizationModel,
    stream: SortformerStream,
    /// 16 kHz samples of meeting audio so far.
    timeline: usize,
    /// Diarization frames before this meeting's stream (a resumed backup).
    frame_offset: usize,
    /// None without the voice model: the speakers are then never merged.
    voices: Option<VoiceTracker<VoiceModel>>,
    backup: Option<Backup>,
}

impl Meeting {
    pub fn new(app: &AppHandle) -> Result<Self> {
        Self::open(app, None)
    }

    /// Continues a backed up transcript. The timeline goes on from a frame
    /// boundary near where it stopped; the diarization starts anew, so the
    /// speakers get new letters until their voices are recognized (which
    /// needs the voice model and the backed up audio).
    pub fn resume(app: &AppHandle, restored: Restored) -> Result<Self> {
        Self::open(app, Some(restored))
    }

    fn open(app: &AppHandle, restored: Option<Restored>) -> Result<Self> {
        let dir = crate::model::manager::auxiliary_dir(app, &DIARIZATION)
            .ok_or_else(|| anyhow!("the speaker diarization model is not downloaded"))?;
        let model = DiarizationModel::load(&dir, DIARIZATION_STEP, None)?;
        let stream = model.stream();
        let num_speakers = model.config.num_speakers;
        let resumed = restored.is_some();
        let frame = restored
            .as_ref()
            .map_or(0, |r| (r.processed_secs / FRAME_SEC).round() as usize);
        let mut voices = voice_model(app).map(|m| VoiceTracker::new(m, num_speakers, frame));
        let mut core = MeetingCore::new(SpeakerTrack::new(num_speakers, FRAME_SEC));
        if let Some(restored) = restored {
            if let (Some(voices), Some(wav)) = (voices.as_mut(), &restored.audio) {
                // before the backup appends to the file
                voices.learn_voices(wav, &restored.lines);
            }
            core.resume(restored.lines, frame);
        }
        Ok(Self {
            core,
            model,
            stream,
            timeline: (frame as f64 * FRAME_SEC * SAMPLE_RATE as f64).round() as usize,
            frame_offset: frame,
            voices,
            backup: Backup::open(app, resumed),
        })
    }

    pub fn processed_secs(&self) -> f64 {
        self.timeline as f64 / SAMPLE_RATE as f64
    }

    fn speaker_rows(&mut self, stream_finished: bool, audio: &[f32]) -> Result<()> {
        let frame = self.frame_offset + self.stream.frames_written();
        let rows = if stream_finished {
            // a clone settles the audio so far; the meeting may go on
            self.stream.clone().finish(&mut self.model.runner)?
        } else {
            self.stream.push(&mut self.model.runner, audio)?
        };
        self.core.add_speaker_probs(frame, &rows);
        if let Some(voices) = self.voices.as_mut() {
            for found in voices.add_rows(frame, &rows) {
                if let Some((from, into)) = self.core.link(found) {
                    info!("Meeting: speaker {from} has the voice of speaker {into}, merged");
                    voices.merged(from, into);
                }
            }
        }
        Ok(())
    }

    fn add_chunk(
        &mut self,
        app: &AppHandle,
        samples: Vec<f32>,
        sample_rate: u32,
        overlap_prefix: usize,
    ) -> Result<()> {
        let audio = if sample_rate as usize == SAMPLE_RATE {
            samples
        } else {
            resample(&samples, sample_rate as usize, SAMPLE_RATE)
        };
        let overlap = (overlap_prefix * SAMPLE_RATE / sample_rate.max(1) as usize).min(audio.len());
        let chunk_start = self.timeline.saturating_sub(overlap) as f64 / SAMPLE_RATE as f64;
        if let Some(voices) = self.voices.as_mut() {
            voices.add_audio(&audio[overlap..]);
        }
        self.speaker_rows(false, &audio[overlap..])?;
        if let Some(backup) = self.backup.as_mut() {
            backup.add_audio(&audio[overlap..]);
        }
        self.timeline += audio.len() - overlap;

        let speed = slow_down_speed(app);
        let input = pad_with_silence(&time_stretch(&audio, speed), SAMPLE_RATE as u32);
        let params = ParakeetInferenceParams {
            timestamp_granularity: TimestampGranularity::Word,
            ..Default::default()
        };
        let result = with_engine(app, |engine| engine.transcribe_samples(input, Some(params)))?
            .map_err(|e| anyhow!("transcription failed: {e}"))?;
        let words = chunk_words(
            &result.segments,
            chunk_start,
            INFERENCE_SILENCE_PAD_SECS as f64,
            speed as f64,
        );
        debug!("Meeting chunk at {chunk_start:.1}s: {} words", words.len());
        self.core.add_chunk(
            words,
            chunk_start,
            self.processed_secs(),
            overlap as f64 / SAMPLE_RATE as f64,
        );
        Ok(())
    }

    /// The current recording ended: settle every word heard so far.
    fn pause(&mut self) -> Result<()> {
        self.speaker_rows(true, &[])?;
        self.core.finish();
        Ok(())
    }

    fn emit_released(&mut self, app: &AppHandle) {
        let changed = self.core.release();
        if self.core.take_relabeled() {
            let lines: Vec<Line> = self
                .core
                .transcript
                .lines
                .iter()
                .map(spelling(app))
                .collect();
            let _ = app.emit(LINES_RESET_EVENT, lines);
        } else if !changed.is_empty() {
            let spell = spelling(app);
            let lines: Vec<IndexedLine> = changed
                .into_iter()
                .map(|index| IndexedLine {
                    index,
                    line: spell(&self.core.transcript.lines[index]),
                })
                .collect();
            let _ = app.emit(LINES_EVENT, lines);
        }
        if let Some(backup) = &self.backup {
            backup.save(&self.core.transcript.lines, self.processed_secs());
        }
        let _ = app.emit(PROGRESS_EVENT, self.processed_secs());
    }
}

/// The voice model, when downloaded and loadable.
fn voice_model(app: &AppHandle) -> Option<VoiceModel> {
    let Some(dir) = crate::model::manager::auxiliary_dir(app, &VOICE) else {
        info!("Meeting: no voice model, speakers will not be merged");
        return None;
    };
    VoiceModel::load(&dir.join(VOICE_MODEL_FILE))
        .map_err(|e| warn!("Meeting: voice model not loaded: {e:#}"))
        .ok()
}

/// Processes the chunks of one recording until `Finalize` or until every
/// sender is gone, then settles the transcript.
pub fn spawn(
    app: AppHandle,
    rx: Receiver<ChunkJob>,
    meeting: Arc<Mutex<Meeting>>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        for job in rx.iter() {
            let ChunkJob::Audio {
                samples,
                sample_rate,
                overlap_prefix,
                ..
            } = job
            else {
                break;
            };
            let mut meeting = meeting.lock();
            if let Err(e) = meeting.add_chunk(&app, samples, sample_rate, overlap_prefix) {
                error!("Meeting chunk failed: {e:#}");
            }
            meeting.emit_released(&app);
        }
        let mut meeting = meeting.lock();
        if let Err(e) = meeting.pause() {
            error!("Meeting pause failed: {e:#}");
        }
        meeting.emit_released(&app);
        let _ = app.emit(IDLE_EVENT, ());
    })
}
