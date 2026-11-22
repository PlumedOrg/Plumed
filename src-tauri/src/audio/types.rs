use crate::audio::chunking::ChunkPipeline;
use crate::audio::recorder::AudioRecorder;
use crate::audio::short_context::ContextAudio;
use crate::engine::ParakeetEngine;
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct MicInfo {
    pub id: String,
    pub label: String,
}

#[derive(Default)]
pub struct PreviewSnapshot {
    pub queue: Vec<f32>,
    pub generation: u64,
    pub revision: u64,
}

pub struct AudioState {
    pub recorder: Mutex<Option<AudioRecorder>>,
    pub engine: Mutex<Option<ParakeetEngine>>,
    pub current_file_name: Mutex<Option<String>>,
    pub streaming_handle: Mutex<Option<std::thread::JoinHandle<()>>>,
    pub streaming_stop: Arc<AtomicBool>,
    pub preview_snapshot: Arc<Mutex<PreviewSnapshot>>,
    pub chunk_inference_active: Arc<AtomicBool>,
    /// The chunking pipeline of the active session
    pub chunk_pipeline: Mutex<Option<ChunkPipeline>>,
    session_gen: AtomicU64,
    /// Spans the whole session, from the record request to the end of
    /// post-processing. Unlike `recorder`, reading it never blocks on the
    /// microphone init and it stays true while the transcript is post-processed.
    session_active: AtomicBool,
    /// Plumed fork: RAM-only tail of recent dictations, see short_context.
    pub short_context: Arc<Mutex<ContextAudio>>,
}

impl AudioState {
    pub fn new() -> Self {
        Self {
            recorder: Mutex::new(None),
            engine: Mutex::new(None),
            current_file_name: Mutex::new(None),
            streaming_handle: Mutex::new(None),
            streaming_stop: Arc::new(AtomicBool::new(false)),
            preview_snapshot: Arc::new(Mutex::new(PreviewSnapshot::default())),
            chunk_inference_active: Arc::new(AtomicBool::new(false)),
            chunk_pipeline: Mutex::new(None),
            session_gen: AtomicU64::new(0),
            session_active: AtomicBool::new(false),
            short_context: Arc::new(Mutex::new(ContextAudio::default())),
        }
    }

    pub fn begin_session(&self) -> u64 {
        self.session_gen.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn invalidate_session(&self) {
        self.session_gen.fetch_add(1, Ordering::SeqCst);
    }

    pub fn current_session(&self) -> u64 {
        self.session_gen.load(Ordering::SeqCst)
    }

    pub fn set_session_active(&self, active: bool) {
        self.session_active.store(active, Ordering::SeqCst);
    }

    pub fn is_session_active(&self) -> bool {
        self.session_active.load(Ordering::SeqCst)
    }
}

pub(super) enum RecorderStartError {
    Busy,
    DirUnavailable,
    InitFailed,
    StartFailed,
    Superseded,
}
