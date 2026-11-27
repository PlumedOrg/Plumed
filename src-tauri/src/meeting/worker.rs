//! Runs a meeting: consumes the `ChunkJob`s of any audio source (microphone
//! `Chunker`, PhoneMic, file), feeds the new audio to the diarization and the
//! whole chunk to the speech engine, and emits the transcript lines.

use super::session::MeetingCore;
use super::speakers::SpeakerTrack;
use super::transcript::Line;
use super::words::chunk_words;
use crate::audio::chunking::ChunkJob;
use crate::audio::helpers::resample;
use crate::audio::pipeline::{
    pad_with_silence, slow_down_speed, with_engine, INFERENCE_SILENCE_PAD_SECS,
};
use crate::audio::time_stretch::time_stretch;
use crate::diarization::diarization::{DiarizationModel, SortformerStream, FRAME_SEC, SAMPLE_RATE};
use crate::engine::transcription_engine::TranscriptionEngine;
use crate::engine::types::{ParakeetInferenceParams, TimestampGranularity};
use crate::model::catalog::{DIARIZATION, DIARIZATION_STEP};
use anyhow::{anyhow, Result};
use log::{debug, error};
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread::JoinHandle;
use tauri::{AppHandle, Emitter};

/// Transcript lines that were added or grew, by index.
pub const LINES_EVENT: &str = "meeting-lines";
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

/// One meeting, kept across its recordings: a new recording continues the
/// timeline and the speakers.
pub struct Meeting {
    pub core: MeetingCore,
    model: DiarizationModel,
    stream: SortformerStream,
    /// 16 kHz samples of meeting audio so far.
    timeline: usize,
}

impl Meeting {
    pub fn new(app: &AppHandle) -> Result<Self> {
        let dir = crate::model::manager::auxiliary_dir(app, &DIARIZATION)
            .ok_or_else(|| anyhow!("the speaker diarization model is not downloaded"))?;
        let model = DiarizationModel::load(&dir, DIARIZATION_STEP, None)?;
        let stream = model.stream();
        let speakers = SpeakerTrack::new(model.config.num_speakers, FRAME_SEC);
        Ok(Self {
            core: MeetingCore::new(speakers),
            model,
            stream,
            timeline: 0,
        })
    }

    pub fn processed_secs(&self) -> f64 {
        self.timeline as f64 / SAMPLE_RATE as f64
    }

    fn speaker_rows(&mut self, stream_finished: bool, audio: &[f32]) -> Result<()> {
        let frame = self.stream.frames_written();
        let rows = if stream_finished {
            // a clone settles the audio so far; the meeting may go on
            self.stream.clone().finish(&mut self.model.runner)?
        } else {
            self.stream.push(&mut self.model.runner, audio)?
        };
        self.core.add_speaker_probs(frame, &rows);
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
        self.speaker_rows(false, &audio[overlap..])?;
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
        if !changed.is_empty() {
            let lines: Vec<IndexedLine> = changed
                .into_iter()
                .map(|index| IndexedLine {
                    index,
                    line: self.core.transcript.lines[index].clone(),
                })
                .collect();
            let _ = app.emit(LINES_EVENT, lines);
        }
        let _ = app.emit(PROGRESS_EVENT, self.processed_secs());
    }
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
