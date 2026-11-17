//! One remote-mic session: room, QR code, pairing, then transcription of each
//! recording sent by the phone. Everything stays in RAM and dies with the session.
//!
//! Lifecycle (`run`): open a room, show its QR code, then serve it (`serve_room`)
//! until the user stops. The QR code stays visible once a phone is connected:
//! scanning it again pairs a new phone (it changes after each scan, see
//! `protocol`). If the relay drops the room (restart, 24 h limit), a new room
//! and QR code are opened.

use super::client::{phone_link, ClientError, RelayClient, Room};
use super::protocol::{Event, Pairing, Step};
use crate::smartmic::audio_bridge::transcribe_pcm;
use log::{info, warn};
use parking_lot::Mutex;
use plumed_relay_protocol::{op, Frame};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

/// After a confirmation, `BOUND` is sent again this often, `BOUND_RESENDS`
/// times: a stale phone tab polling the same mailbox may swallow the first one.
const BOUND_RESEND: Duration = Duration::from_secs(2);
const BOUND_RESENDS: u32 = 5;
/// Pause before polling again after a network error.
const RETRY_DELAY: Duration = Duration::from_secs(3);
/// Recordings waiting for the transcriber; more are dropped (a phone flooding
/// the desktop must not pile up pastes, nor memory).
const QUEUE_LEN: usize = 4;
/// A recording still queued after this long is dropped: by then the user has
/// moved on, and pasting it would land in whatever window has the focus now.
const MAX_QUEUE_AGE: Duration = Duration::from_secs(60);
pub const STATE_EVENT: &str = "remote-mic-state";
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Stopped,
    Starting,
    /// QR code shown, no phone yet.
    Waiting,
    /// Phone joined: both users must compare the fingerprint.
    Verify,
    Connected,
}

/// What the UI shows. `error` explains the last stop, if any.
#[derive(Serialize, Clone, Debug, Default)]
pub struct Snapshot {
    pub status: Status,
    pub qr: Option<String>,
    pub link: Option<String>,
    pub fingerprint: Option<String>,
    pub error: Option<String>,
}

pub enum Command {
    /// The user compared the fingerprint: `true` accepts, `false` refuses.
    Confirm(bool),
    Stop,
}

/// Tauri-managed handle on the (at most one) running session.
#[derive(Default)]
pub struct RemoteMicState {
    commands: Mutex<Option<mpsc::UnboundedSender<Command>>>,
    snapshot: Mutex<Snapshot>,
}

impl RemoteMicState {
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.lock().clone()
    }

    pub fn start(&self, app: AppHandle, client: RelayClient) -> Result<(), String> {
        let mut commands = self.commands.lock();
        if commands.as_ref().is_some_and(|tx| !tx.is_closed()) {
            return Err("the remote microphone is already running".to_string());
        }
        let (tx, rx) = mpsc::unbounded_channel();
        *commands = Some(tx);
        tauri::async_runtime::spawn(run(app, client, rx));
        Ok(())
    }

    /// Sends a command to the running session (ignored when none runs).
    pub fn send(&self, command: Command) {
        if let Some(tx) = self.commands.lock().as_ref() {
            let _ = tx.send(command);
        }
    }
}

/// Stores and broadcasts the UI state.
struct Ui<'a> {
    app: &'a AppHandle,
}

impl Ui<'_> {
    fn update(&self, change: impl FnOnce(&mut Snapshot)) {
        use tauri::Manager;
        let state = self.app.state::<RemoteMicState>();
        let snapshot = {
            let mut s = state.snapshot.lock();
            change(&mut s);
            s.clone()
        };
        let _ = self.app.emit(STATE_EVENT, snapshot);
    }

    /// Reflects the pairing state; the QR code is only re-rendered when it changed.
    fn show(&self, origin: &str, room: &Room, pairing: &Pairing) {
        let link = phone_link(origin, room, &pairing.link_key());
        let status = match (pairing.awaits_confirmation(), pairing.is_bound()) {
            (true, _) => Status::Verify,
            (false, true) => Status::Connected,
            (false, false) => Status::Waiting,
        };
        let fingerprint = pairing.fingerprint().map(str::to_owned);
        self.update(|s| {
            if s.link.as_deref() != Some(link.as_str()) {
                s.qr = crate::smartmic::qr::encode_svg_data_uri(&link).ok();
                s.link = Some(link);
            }
            s.status = status;
            s.fingerprint = fingerprint;
            s.error = None;
        });
    }

    fn stopped(&self, error: Option<String>) {
        self.update(|s| {
            *s = Snapshot {
                error,
                ..Snapshot::default()
            }
        });
    }
}

enum RoomOutcome {
    Gone,
    Stop,
    Failed(String),
}

async fn run(app: AppHandle, client: RelayClient, mut commands: mpsc::UnboundedReceiver<Command>) {
    let ui = Ui { app: &app };
    ui.update(|s| {
        *s = Snapshot {
            status: Status::Starting,
            ..Snapshot::default()
        }
    });
    let recordings = spawn_transcriber(app.clone());
    let error = loop {
        let room = match open_room(&client, &mut commands).await {
            Ok(Some(room)) => room,
            Ok(None) => break None,
            Err(e) => break Some(e.to_string()),
        };
        let outcome = serve_room(&client, &room, &mut commands, &ui, &recordings).await;
        match outcome {
            RoomOutcome::Gone => info!("Remote mic: the relay dropped the room, opening a new one"),
            RoomOutcome::Stop => {
                client.close(&room).await;
                break None;
            }
            RoomOutcome::Failed(e) => {
                client.close(&room).await;
                break Some(e);
            }
        }
    };
    // Nothing queued is pasted after a stop
    recordings.flush();
    // Close the channel so a new `start` is accepted before the UI hears "stopped"
    commands.close();
    ui.stopped(error);
}

/// Creates a room, waiting while the relay is full or rate limits us.
/// `Ok(None)` means the user stopped meanwhile.
async fn open_room(
    client: &RelayClient,
    commands: &mut mpsc::UnboundedReceiver<Command>,
) -> Result<Option<Room>, ClientError> {
    loop {
        let delay = match client.create_room().await {
            Ok(room) => return Ok(Some(room)),
            Err(ClientError::RateLimited(secs)) => Duration::from_secs(secs.min(60)),
            Err(e) => return Err(e),
        };
        tokio::select! {
            _ = tokio::time::sleep(delay) => {}
            cmd = commands.recv() => if matches!(cmd, Some(Command::Stop) | None) {
                return Ok(None);
            },
        }
    }
}

/// Long-polls the relay into a channel. Network errors are retried so a flaky
/// connection does not drop the session; only a closed room ends it.
fn spawn_poller(
    client: RelayClient,
    room: Room,
) -> (
    mpsc::Receiver<Result<Vec<Frame>, ClientError>>,
    tauri::async_runtime::JoinHandle<()>,
) {
    let (tx, rx) = mpsc::channel(16);
    let task = tauri::async_runtime::spawn(async move {
        loop {
            let delay = match client.poll(&room).await {
                Ok(frames) if frames.is_empty() => continue,
                Ok(frames) => match tx.send(Ok(frames)).await {
                    Ok(()) => continue,
                    Err(_) => break,
                },
                Err(ClientError::RoomGone) => {
                    let _ = tx.send(Err(ClientError::RoomGone)).await;
                    break;
                }
                Err(ClientError::RateLimited(secs)) => Duration::from_secs(secs.min(60)),
                Err(e) => {
                    warn!("Remote mic: relay poll failed, retrying: {}", e);
                    RETRY_DELAY
                }
            };
            tokio::time::sleep(delay).await;
        }
    });
    (rx, task)
}

/// Posts what the protocol produced. A lost reply is not fatal: `BOUND` is
/// resent, and `RECEIVED`, `stale` or `denied` only update the phone screen.
async fn post(client: &RelayClient, room: &Room, frames: &[Frame]) {
    if frames.is_empty() {
        return;
    }
    if let Err(e) = client.send(room, frames).await {
        warn!("Remote mic: could not reach the phone: {}", e);
    }
}

async fn serve_room(
    client: &RelayClient,
    room: &Room,
    commands: &mut mpsc::UnboundedReceiver<Command>,
    ui: &Ui<'_>,
    recordings: &RecordingQueue,
) -> RoomOutcome {
    let mut pairing = match Pairing::new() {
        Ok(p) => p,
        Err(e) => return RoomOutcome::Failed(e.to_string()),
    };
    ui.show(client.origin(), room, &pairing);
    let (mut incoming, poller) = spawn_poller(client.clone(), room.clone());
    let mut resend = tokio::time::interval(BOUND_RESEND);
    let mut bound_resends = 0;

    let outcome = loop {
        let step = tokio::select! {
            _ = resend.tick(), if bound_resends > 0 => {
                bound_resends -= 1;
                pairing.seal_op(op::BOUND).map(|frame| Step {
                    send: frame.into_iter().collect(),
                    event: None,
                })
            }
            cmd = commands.recv() => match cmd {
                Some(Command::Confirm(accepted)) => {
                    let step = pairing.confirm(accepted);
                    if accepted && pairing.is_bound() {
                        // A new phone: what the previous one sent is not pasted
                        recordings.flush();
                        bound_resends = BOUND_RESENDS;
                        resend.reset();
                    }
                    step
                }
                Some(Command::Stop) | None => break RoomOutcome::Stop,
            },
            item = incoming.recv() => match item {
                Some(Ok(frames)) => handle_frames(&mut pairing, frames, recordings),
                Some(Err(ClientError::RoomGone)) => break RoomOutcome::Gone,
                Some(Err(e)) => break RoomOutcome::Failed(e.to_string()),
                None => break RoomOutcome::Failed("relay poller stopped".to_string()),
            },
        };
        match step {
            Ok(step) => {
                post(client, room, &step.send).await;
                ui.show(client.origin(), room, &pairing);
            }
            Err(e) => break RoomOutcome::Failed(e.to_string()),
        }
    };
    poller.abort();
    outcome
}

/// Feeds a batch to the protocol, queues the recordings, merges the replies.
fn handle_frames(
    pairing: &mut Pairing,
    frames: Vec<Frame>,
    recordings: &RecordingQueue,
) -> Result<Step, super::crypto::CryptoError> {
    let mut send = Vec::new();
    for frame in frames {
        let step = pairing.on_frame(frame)?;
        send.extend(step.send);
        if let Some(Event::Recording { pcm, sample_rate }) = step.event {
            if !recordings.push(pcm, sample_rate) {
                warn!("Remote mic: transcription queue full, recording dropped");
            }
        }
    }
    Ok(Step { send, event: None })
}

struct Queued {
    pcm: Vec<i16>,
    sample_rate: u32,
    at: Instant,
    generation: u64,
}

/// Bounded queue of recordings to transcribe. `flush` invalidates everything
/// queued so far (stop, new phone) without waiting for the transcriber.
struct RecordingQueue {
    tx: mpsc::Sender<Queued>,
    generation: Arc<AtomicU64>,
}

impl RecordingQueue {
    fn new() -> (Self, mpsc::Receiver<Queued>, Arc<AtomicU64>) {
        let (tx, rx) = mpsc::channel(QUEUE_LEN);
        let generation = Arc::new(AtomicU64::new(0));
        let queue = Self {
            tx,
            generation: generation.clone(),
        };
        (queue, rx, generation)
    }

    /// `false` when the queue is full and the recording was dropped.
    fn push(&self, pcm: Vec<i16>, sample_rate: u32) -> bool {
        let item = Queued {
            pcm,
            sample_rate,
            at: Instant::now(),
            generation: self.generation.load(Ordering::SeqCst),
        };
        self.tx.try_send(item).is_ok()
    }

    fn flush(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
}

impl Queued {
    fn is_current(&self, generation: u64, now: Instant) -> bool {
        self.generation == generation && now.duration_since(self.at) <= MAX_QUEUE_AGE
    }
}

/// Transcribes and pastes recordings one at a time, in arrival order.
fn spawn_transcriber(app: AppHandle) -> RecordingQueue {
    let (queue, mut rx, generation) = RecordingQueue::new();
    tauri::async_runtime::spawn(async move {
        while let Some(item) = rx.recv().await {
            if !item.is_current(generation.load(Ordering::SeqCst), Instant::now()) {
                info!("Remote mic: dropped a recording that waited too long or was flushed");
                continue;
            }
            let app = app.clone();
            let Queued {
                pcm, sample_rate, ..
            } = item;
            let _ = tauri::async_runtime::spawn_blocking(move || {
                match transcribe_pcm(&app, pcm, sample_rate) {
                    Ok(text) if !text.is_empty() => {
                        if let Err(e) = crate::clipboard::paste(&text, &app) {
                            warn!("Remote mic: paste failed: {}", e);
                        }
                    }
                    Ok(_) => {}
                    // The transcript is never logged, only the failure
                    Err(e) => warn!("Remote mic: {:?}", e),
                }
            })
            .await;
        }
    });
    queue
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_queue_is_bounded() {
        let (queue, _rx, _) = RecordingQueue::new();
        for _ in 0..QUEUE_LEN {
            assert!(queue.push(vec![1], 16000));
        }
        assert!(!queue.push(vec![1], 16000));
    }

    #[test]
    fn flushed_or_old_recordings_are_not_current() {
        let (queue, mut rx, generation) = RecordingQueue::new();
        queue.push(vec![1], 16000);
        queue.flush();
        queue.push(vec![2], 16000);
        let now = Instant::now();
        let current = generation.load(Ordering::SeqCst);
        let flushed = rx.try_recv().unwrap();
        assert!(!flushed.is_current(current, now));
        let fresh = rx.try_recv().unwrap();
        assert!(fresh.is_current(current, now));
        let later = now + MAX_QUEUE_AGE + Duration::from_secs(1);
        assert!(!fresh.is_current(current, later));
    }
}
