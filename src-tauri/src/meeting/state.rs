//! The meeting of the Réunion page and its current recording. A recording has
//! one source (the microphone, a file, a PhoneMic recording); stopping it
//! keeps the meeting, which the next recording continues.

use super::backup::{self, Resume};
use super::transcript::Line;
use super::worker::{self, Meeting};
use crate::audio::chunking::{ChunkJob, Chunker};
use crate::audio::recorder::AudioRecorder;
use anyhow::{anyhow, Context, Result};
use log::warn;
use parking_lot::Mutex;
use rodio::Source;
use serde::Serialize;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tauri::AppHandle;

/// A file or PhoneMic source is read at most this far ahead of the
/// transcription, so a 4 h file is never held in memory.
const MAX_AHEAD_SECS: f64 = 180.0;
/// Decoded blocks of this many seconds.
const BLOCK_SECS: usize = 1;
/// PhoneMic recordings are resampled to this rate (`finalize_buffer`).
const PHONE_SAMPLE_RATE: u32 = 16000;

#[derive(Default)]
pub struct MeetingState {
    meeting: Mutex<Option<Arc<Mutex<Meeting>>>>,
    recording: Mutex<Option<Recording>>,
    /// Set while PhoneMic is the source: its recordings go to the meeting.
    phone: Mutex<Option<Sender<Block>>>,
}

/// Decoded audio for the feeder.
struct Block {
    samples: Vec<f32>,
    /// The last block of a recording: its tail is transcribed right away.
    ends_recording: bool,
}

struct Recording {
    /// Live microphone source.
    recorder: Option<AudioRecorder>,
    /// Stops a file or PhoneMic source.
    cancel: Arc<AtomicBool>,
    worker: JoinHandle<()>,
}

impl Recording {
    fn is_running(&self) -> bool {
        !self.worker.is_finished()
    }
}

#[derive(Serialize)]
pub struct MeetingSnapshot {
    pub lines: Vec<Line>,
    pub processed_secs: f64,
    /// A recording is running or its last chunks are still transcribed.
    pub running: bool,
}

impl MeetingState {
    /// Starts recording the microphone into the meeting.
    pub fn start_mic(&self, app: &AppHandle) -> Result<()> {
        self.start(app, |tx, _, _| {
            let mut recorder = AudioRecorder::open(app.clone(), None, Some(tx), false)?;
            recorder.start(false, std::time::Instant::now())?;
            Ok(Some(recorder))
        })
    }

    /// Transcribes an audio file (WAV, MP3, M4A, FLAC, OGG) into the meeting.
    /// Returns its duration in seconds when the format tells it.
    pub fn start_file(&self, app: &AppHandle, path: &Path) -> Result<Option<f64>> {
        let file = std::fs::File::open(path).with_context(|| format!("cannot open {path:?}"))?;
        let decoder = rodio::Decoder::try_from(file).context("unsupported audio file")?;
        let sample_rate = decoder.sample_rate().get();
        let duration = decoder.total_duration().map(|d| d.as_secs_f64());
        let blocks = mono_blocks(decoder);
        self.start(app, move |tx, meeting, cancel| {
            spawn_feeder(tx, meeting, cancel, blocks, sample_rate);
            Ok(None)
        })?;
        Ok(duration)
    }

    /// Makes PhoneMic the source: each phone recording goes to the meeting.
    pub fn start_phone(&self, app: &AppHandle) -> Result<()> {
        self.start(app, |tx, meeting, cancel| {
            let (phone, blocks) = mpsc::channel();
            spawn_feeder(tx, meeting, cancel, blocks.into_iter(), PHONE_SAMPLE_RATE);
            *self.phone.lock() = Some(phone);
            Ok(None)
        })
    }

    pub fn takes_phone(&self) -> bool {
        self.phone.lock().is_some()
    }

    /// A PhoneMic recording (16 kHz) while the phone is the source.
    pub fn push_phone(&self, samples: Vec<f32>) {
        if let Some(phone) = self.phone.lock().as_ref() {
            let _ = phone.send(Block {
                samples,
                ends_recording: true,
            });
        }
    }

    /// Opens a recording: loads the meeting on first use, waits for the
    /// previous (stopped) recording to be fully transcribed, and starts the
    /// worker on the chunks of the source opened by `open_source`.
    fn start(
        &self,
        app: &AppHandle,
        open_source: impl FnOnce(
            Sender<ChunkJob>,
            Arc<Mutex<Meeting>>,
            Arc<AtomicBool>,
        ) -> Result<Option<AudioRecorder>>,
    ) -> Result<()> {
        let mut recording = self.recording.lock();
        if let Some(previous) = recording.take() {
            if previous.is_running() && !previous.cancel.load(Ordering::SeqCst) {
                *recording = Some(previous);
                return Err(anyhow!("a meeting recording is already running"));
            }
            // its last chunks are transcribed first
            let _ = previous.worker.join();
        }
        let meeting = self.meeting(app)?;
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let recorder = open_source(tx, meeting.clone(), cancel.clone())?;
        let worker = keep_awake_while(worker::spawn(app.clone(), rx, meeting));
        *recording = Some(Recording {
            recorder,
            cancel,
            worker,
        });
        Ok(())
    }

    fn meeting(&self, app: &AppHandle) -> Result<Arc<Mutex<Meeting>>> {
        let mut meeting = self.meeting.lock();
        if meeting.is_none() {
            *meeting = Some(Arc::new(Mutex::new(Meeting::new(app)?)));
        }
        Ok(meeting.clone().expect("just set"))
    }

    /// Stops the source; the audio already captured is still transcribed.
    pub fn stop(&self) {
        // the phone feeder ends once its queued recordings are read
        self.phone.lock().take();
        let mut recording = self.recording.lock();
        if let Some(recording) = recording.as_mut() {
            recording.cancel.store(true, Ordering::SeqCst);
            if let Some(mut recorder) = recording.recorder.take() {
                if let Err(e) = recorder.stop(false) {
                    warn!("Meeting: stopping the microphone failed: {e:#}");
                }
            }
        }
    }

    /// Stops the source, waits for its last chunks and drops the meeting.
    fn forget(&self) {
        self.stop();
        if let Some(recording) = self.recording.lock().take() {
            let _ = recording.worker.join();
        }
        *self.meeting.lock() = None;
    }

    /// Forgets the meeting (stopped first) and its backup: the next
    /// recording starts anew.
    pub fn reset(&self, app: &AppHandle) {
        self.forget();
        backup::delete(app);
    }

    /// Continues the meeting backed up by a previous run: its transcript, or
    /// else its audio transcribed again (then returns the audio duration).
    pub fn resume_backup(&self, app: &AppHandle) -> Result<Option<f64>> {
        self.forget();
        match backup::take(app)? {
            Some(Resume::Text(restored)) => {
                let meeting = Meeting::resume(app, restored)?;
                *self.meeting.lock() = Some(Arc::new(Mutex::new(meeting)));
                Ok(None)
            }
            Some(Resume::Audio(path)) => self.start_file(app, &path),
            None => Err(anyhow!("no meeting backup found")),
        }
    }

    /// A recording is running or its last chunks are still transcribed.
    pub fn is_running(&self) -> bool {
        self.recording
            .lock()
            .as_ref()
            .is_some_and(Recording::is_running)
    }

    pub fn snapshot(&self, app: &AppHandle) -> MeetingSnapshot {
        let running = self.is_running();
        let meeting = self.meeting.lock().clone();
        match meeting {
            Some(meeting) => {
                let meeting = meeting.lock();
                MeetingSnapshot {
                    lines: meeting
                        .core
                        .transcript
                        .lines
                        .iter()
                        .map(worker::spelling(app))
                        .collect(),
                    processed_secs: meeting.processed_secs(),
                    running,
                }
            }
            None => MeetingSnapshot {
                lines: Vec::new(),
                processed_secs: 0.0,
                running,
            },
        }
    }
}

/// Plumed fork (written with Claude Code): the computer does not fall asleep
/// on its own while a meeting is recorded or transcribed (a manual suspend
/// still works). The returned handle ends with `worker`. The inhibition is
/// taken and released on one thread: on Windows it belongs to a thread.
fn keep_awake_while(worker: JoinHandle<()>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let awake = keepawake::Builder::default()
            .idle(true)
            .reason("Meeting transcription")
            .app_name("Plumed")
            .app_reverse_domain("org.olicorne.plumed")
            .create();
        if let Err(e) = &awake {
            warn!("Meeting: cannot keep the computer awake: {e}");
        }
        let _ = worker.join();
        drop(awake);
    })
}

/// Mono blocks of `BLOCK_SECS` decoded on demand.
fn mono_blocks(
    decoder: rodio::Decoder<std::io::BufReader<std::fs::File>>,
) -> impl Iterator<Item = Block> + Send {
    let channels = decoder.channels().get() as usize;
    let block = decoder.sample_rate().get() as usize * BLOCK_SECS * channels;
    let mut samples = decoder.peekable();
    std::iter::from_fn(move || {
        samples.peek()?;
        let interleaved: Vec<f32> = samples.by_ref().take(block).collect();
        Some(Block {
            samples: interleaved
                .chunks(channels)
                .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
                .collect(),
            ends_recording: false,
        })
    })
}

/// Cuts already recorded audio into meeting chunks, reading it no further
/// than `MAX_AHEAD_SECS` ahead of the transcription. Dropping `tx` at the end
/// lets the worker settle the transcript.
fn spawn_feeder(
    tx: Sender<ChunkJob>,
    meeting: Arc<Mutex<Meeting>>,
    cancel: Arc<AtomicBool>,
    blocks: impl Iterator<Item = Block> + Send + 'static,
    sample_rate: u32,
) {
    std::thread::spawn(move || {
        let start = meeting.lock().processed_secs();
        let mut processed = start;
        let mut fed = 0usize;
        let mut chunker = Chunker::new(tx.clone(), sample_rate, None);
        for block in blocks {
            while start + fed as f64 / sample_rate as f64 - processed > MAX_AHEAD_SECS {
                if cancel.load(Ordering::SeqCst) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(250));
                // the worker holds the lock while it transcribes
                if let Some(meeting) = meeting.try_lock() {
                    processed = meeting.processed_secs();
                }
            }
            if cancel.load(Ordering::SeqCst) {
                break;
            }
            chunker.push_ticked(&block.samples);
            fed += block.samples.len();
            if block.ends_recording {
                chunker.flush_remaining();
                chunker = Chunker::new(tx.clone(), sample_rate, None);
            }
        }
        if fed == 0 {
            warn!("Meeting: no audio from the source");
        }
        chunker.flush_remaining();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stereo_file_is_decoded_into_mono_blocks_of_one_second() {
        let path = std::env::temp_dir().join("plumed_meeting_stereo.wav");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 8000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        // 1.5 s: left at 0.5, right at 0.0, so the mono mix is 0.25
        for _ in 0..12000 {
            writer.write_sample((0.5 * i16::MAX as f32) as i16).unwrap();
            writer.write_sample(0i16).unwrap();
        }
        writer.finalize().unwrap();

        let decoder = rodio::Decoder::try_from(std::fs::File::open(&path).unwrap()).unwrap();
        let blocks: Vec<Block> = mono_blocks(decoder).collect();
        std::fs::remove_file(&path).unwrap();

        let lengths: Vec<usize> = blocks.iter().map(|b| b.samples.len()).collect();
        assert_eq!(lengths, vec![8000, 4000]);
        assert!(blocks
            .iter()
            .flat_map(|b| &b.samples)
            .all(|s| (s - 0.25).abs() < 1e-3));
        assert!(blocks.iter().all(|b| !b.ends_recording));
    }
}
