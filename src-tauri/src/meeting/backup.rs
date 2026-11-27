//! Plumed fork (written with Claude Code): opt-in backup of the meeting in
//! progress, so a crash or a power cut does not lose it. Each part has its own
//! setting: the audio (`audio.wav`, 16 kHz mono, flushed after each chunk so
//! the file stays readable), the raw transcript (`transcript.json`) and the
//! metadata (`meeting.json`). The backup lives in the temporary folder, or in
//! the app data folder when persistent, and is deleted by "New meeting".

use super::transcript::Line;
use anyhow::{Context, Result};
use hound::{WavReader, WavSpec, WavWriter};
use log::warn;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const AUDIO: &str = "audio.wav";
const TEXT: &str = "transcript.json";
const METADATA: &str = "meeting.json";
const DIR_NAME: &str = "plumed-meeting-backup";
/// The audio of a backup resumed without its transcript is moved here and
/// transcribed again like a file.
const RESUMED_AUDIO: &str = "plumed-meeting-resumed.wav";
const SAMPLE_RATE: u32 = 16000;

/// What the settings ask to save.
#[derive(Clone, Copy, Default)]
pub struct Parts {
    pub audio: bool,
    pub text: bool,
    pub metadata: bool,
}

impl Parts {
    fn any(self) -> bool {
        self.audio || self.text || self.metadata
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
struct Metadata {
    /// Unix seconds.
    started_at: u64,
    updated_at: u64,
    processed_secs: f64,
}

/// A backup left by a previous run, for the startup dialog.
#[derive(Serialize, Clone)]
pub struct Found {
    #[serde(skip)]
    dir: PathBuf,
    pub lines: usize,
    pub processed_secs: f64,
    pub has_audio: bool,
    pub has_text: bool,
    /// Unix seconds, when the metadata was saved.
    pub updated_at: Option<u64>,
}

/// A transcript to continue.
pub struct Restored {
    pub lines: Vec<Line>,
    pub processed_secs: f64,
}

/// The backup of the meeting in progress.
pub struct Backup {
    dir: PathBuf,
    parts: Parts,
    audio: Option<WavWriter<BufWriter<File>>>,
    started_at: u64,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn temp_dir() -> PathBuf {
    std::env::temp_dir().join(DIR_NAME)
}

fn persistent_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join("meeting-backup"))
}

/// Both locations: a backup may predate a change of the setting.
fn locations(app: &AppHandle) -> Vec<PathBuf> {
    std::iter::once(temp_dir())
        .chain(persistent_dir(app))
        .collect()
}

fn parts_of(app: &AppHandle) -> (Parts, bool) {
    let s = crate::settings::load_settings(app);
    let parts = Parts {
        audio: s.meeting_backup_audio,
        text: s.meeting_backup_text,
        metadata: s.meeting_backup_metadata,
    };
    (parts, s.meeting_backup_persistent)
}

fn remove_dir(dir: &Path) {
    if dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(dir) {
            warn!("Meeting backup: cannot delete {dir:?}: {e}");
        }
    }
}

fn delete_backups(app: &AppHandle) {
    for dir in locations(app) {
        remove_dir(&dir);
    }
}

/// Deletes every backup and the audio of a resumed one.
pub fn delete(app: &AppHandle) {
    delete_backups(app);
    let resumed = std::env::temp_dir().join(RESUMED_AUDIO);
    if resumed.exists() {
        if let Err(e) = std::fs::remove_file(&resumed) {
            warn!("Meeting backup: cannot delete {resumed:?}: {e}");
        }
    }
}

pub fn find(app: &AppHandle) -> Option<Found> {
    locations(app).into_iter().find_map(|dir| find_in(&dir))
}

fn find_in(dir: &Path) -> Option<Found> {
    let audio = dir.join(AUDIO);
    let lines = read_lines(dir);
    let has_audio = audio_secs(&audio).is_some_and(|secs| secs > 0.0);
    if lines.is_none() && !has_audio {
        return None;
    }
    let metadata = read_metadata(dir);
    let restored = restored_in(dir, lines.clone().unwrap_or_default(), metadata);
    Some(Found {
        dir: dir.to_path_buf(),
        lines: lines.as_ref().map_or(0, Vec::len),
        processed_secs: restored.processed_secs,
        has_audio,
        has_text: lines.is_some(),
        updated_at: metadata.map(|m| m.updated_at),
    })
}

/// How far the meeting went: the metadata knows, else the audio length, else
/// the end of the last line.
fn restored_in(dir: &Path, lines: Vec<Line>, metadata: Option<Metadata>) -> Restored {
    let processed_secs = metadata
        .map(|m| m.processed_secs)
        .or_else(|| audio_secs(&dir.join(AUDIO)))
        .or_else(|| lines.last().map(|line| line.end))
        .unwrap_or(0.0);
    Restored {
        lines,
        processed_secs,
    }
}

fn read_lines(dir: &Path) -> Option<Vec<Line>> {
    let text = std::fs::read_to_string(dir.join(TEXT)).ok()?;
    serde_json::from_str(&text)
        .map_err(|e| warn!("Meeting backup: unreadable transcript: {e}"))
        .ok()
}

fn read_metadata(dir: &Path) -> Option<Metadata> {
    let text = std::fs::read_to_string(dir.join(METADATA)).ok()?;
    serde_json::from_str(&text).ok()
}

fn audio_secs(path: &Path) -> Option<f64> {
    let reader = WavReader::open(path).ok()?;
    Some(reader.len() as f64 / reader.spec().sample_rate as f64)
}

/// How to continue a backup left by a previous run.
pub enum Resume {
    /// Continue its transcript.
    Text(Restored),
    /// Transcribe its audio again, from this file.
    Audio(PathBuf),
}

/// The transcript when there is one, else the audio, moved out of the
/// backup folder (the new meeting starts a new backup).
pub fn take(app: &AppHandle) -> Result<Option<Resume>> {
    let Some(found) = find(app) else {
        return Ok(None);
    };
    if found.has_text {
        let lines = read_lines(&found.dir).unwrap_or_default();
        return Ok(Some(Resume::Text(restored_in(
            &found.dir,
            lines,
            read_metadata(&found.dir),
        ))));
    }
    let resumed = std::env::temp_dir().join(RESUMED_AUDIO);
    let audio = found.dir.join(AUDIO);
    // a rename cannot cross volumes (persistent backup, temporary folder)
    if std::fs::rename(&audio, &resumed).is_err() {
        std::fs::copy(&audio, &resumed).context("cannot copy the backed up audio")?;
    }
    Ok(Some(Resume::Audio(resumed)))
}

impl Backup {
    /// The backup of a new meeting (`resumed` is false: older backups are
    /// deleted) or of a resumed one (its files are continued). None when the
    /// settings save nothing.
    pub fn open(app: &AppHandle, resumed: bool) -> Option<Self> {
        let (parts, persistent) = parts_of(app);
        let dir = if persistent {
            persistent_dir(app)?
        } else {
            temp_dir()
        };
        if !resumed {
            // not the audio of a resumed backup: it may be read right now
            delete_backups(app);
        }
        if !parts.any() {
            return None;
        }
        Self::open_in(dir, parts)
            .map_err(|e| warn!("Meeting backup disabled: {e:#}"))
            .ok()
    }

    fn open_in(dir: PathBuf, parts: Parts) -> Result<Self> {
        std::fs::create_dir_all(&dir).with_context(|| format!("cannot create {dir:?}"))?;
        // a part turned off since is not kept
        for (on, name) in [
            (parts.audio, AUDIO),
            (parts.text, TEXT),
            (parts.metadata, METADATA),
        ] {
            if !on {
                let _ = std::fs::remove_file(dir.join(name));
            }
        }
        let audio = if parts.audio {
            let path = dir.join(AUDIO);
            Some(match WavWriter::append(&path) {
                Ok(writer) if writer.spec().sample_rate == SAMPLE_RATE => writer,
                _ => WavWriter::create(&path, wav_spec())
                    .with_context(|| format!("cannot create {path:?}"))?,
            })
        } else {
            None
        };
        let started_at = read_metadata(&dir).map_or_else(now, |m| m.started_at);
        Ok(Self {
            dir,
            parts,
            audio,
            started_at,
        })
    }

    /// New meeting audio, 16 kHz.
    pub fn add_audio(&mut self, samples: &[f32]) {
        let Some(writer) = self.audio.as_mut() else {
            return;
        };
        let written = samples
            .iter()
            .try_for_each(|s| writer.write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16))
            .and_then(|()| writer.flush());
        if let Err(e) = written {
            warn!("Meeting backup: audio not saved, stopping its backup: {e}");
            self.audio = None;
        }
    }

    /// The transcript and how far the meeting went.
    pub fn save(&self, lines: &[Line], processed_secs: f64) {
        if self.parts.text {
            self.write(TEXT, &lines);
        }
        if self.parts.metadata {
            let metadata = Metadata {
                started_at: self.started_at,
                updated_at: now(),
                processed_secs,
            };
            self.write(METADATA, &metadata);
        }
    }

    /// Written aside then renamed, so a crash leaves the previous version.
    fn write(&self, name: &str, value: &impl Serialize) {
        let path = self.dir.join(name);
        let tmp = self.dir.join(format!("{name}.tmp"));
        let written = serde_json::to_vec(value)
            .map_err(std::io::Error::other)
            .and_then(|json| std::fs::write(&tmp, json))
            .and_then(|()| std::fs::rename(&tmp, &path));
        if let Err(e) = written {
            warn!("Meeting backup: cannot save {path:?}: {e}");
        }
    }
}

fn wav_spec() -> WavSpec {
    WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(speaker: usize, start: f64, end: f64, text: &str) -> Line {
        Line {
            speaker,
            start,
            end,
            text: text.into(),
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("plumed_backup_test_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    const ALL: Parts = Parts {
        audio: true,
        text: true,
        metadata: true,
    };

    #[test]
    fn a_backup_is_found_with_its_lines_and_progress() {
        let dir = scratch("found");
        let lines = vec![line(0, 0.0, 2.0, "bonjour"), line(1, 2.5, 4.0, "salut")];
        let mut backup = Backup::open_in(dir.clone(), ALL).unwrap();
        backup.add_audio(&[0.5; 16000]);
        backup.save(&lines, 1.0);

        let found = find_in(&dir).unwrap();
        assert_eq!(found.lines, 2);
        assert!(found.has_audio && found.has_text);
        // the metadata wins over the audio length and the last line
        assert_eq!(found.processed_secs, 1.0);
        assert_eq!(read_lines(&dir).unwrap(), lines);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_resumed_backup_appends_to_its_audio() {
        let dir = scratch("append");
        let parts = Parts {
            audio: true,
            ..Parts::default()
        };
        Backup::open_in(dir.clone(), parts)
            .unwrap()
            .add_audio(&[0.25; 8000]);
        // the first writer was dropped without finalize, like after a crash
        Backup::open_in(dir.clone(), parts)
            .unwrap()
            .add_audio(&[-0.25; 8000]);

        let mut reader = WavReader::open(dir.join(AUDIO)).unwrap();
        let samples: Vec<i16> = reader.samples::<i16>().map(Result::unwrap).collect();
        assert_eq!(samples.len(), 16000);
        assert!(samples[0] > 0 && samples[15999] < 0);
        // no metadata, no transcript: the audio tells how far the meeting went
        let found = find_in(&dir).unwrap();
        assert_eq!((found.processed_secs, found.has_text), (1.0, false));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn without_metadata_the_last_line_tells_the_progress() {
        let dir = scratch("text_only");
        let parts = Parts {
            text: true,
            ..Parts::default()
        };
        Backup::open_in(dir.clone(), parts)
            .unwrap()
            .save(&[line(0, 0.0, 7.5, "bonjour")], 9.0);
        assert_eq!(find_in(&dir).unwrap().processed_secs, 7.5);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_part_turned_off_is_deleted_and_an_empty_folder_is_no_backup() {
        let dir = scratch("turned_off");
        Backup::open_in(dir.clone(), ALL)
            .unwrap()
            .save(&[line(0, 0.0, 1.0, "bonjour")], 1.0);
        let audio_only = Parts {
            audio: true,
            ..Parts::default()
        };
        let _backup = Backup::open_in(dir.clone(), audio_only).unwrap();
        assert!(!dir.join(TEXT).exists() && !dir.join(METADATA).exists());
        // an audio file without samples is nothing to resume
        assert!(find_in(&dir).is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
