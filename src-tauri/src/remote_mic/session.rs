//! One remote-mic session: rooms, QR code, pairing, then transcription of each
//! recording sent by the phone. Everything stays in RAM and dies with the session.
//!
//! Lifecycle (`run`): open the lobby room, show its QR code, then serve it
//! (`serve`) until the user stops. A confirmed phone gets a private room of its
//! own (see `protocol`), and the lobby is then replaced, so the QR code shown
//! changes and a photo of an old one opens nothing. The QR code stays visible:
//! scanning it pairs a new phone, which replaces the bound one. If the relay
//! drops a room (restart, 24 h limit), a new lobby is opened, or the phone
//! must scan again.

use super::audio::transcribe_pcm;
use super::client::{phone_link, ClientError, RelayClient, Room};
use super::protocol::{Event, Outgoing, Pairing, Step, Via, CANDIDATE_TTL};
use log::{info, warn};
use parking_lot::Mutex;
use plumed_relay_protocol::Frame;
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
/// A transcript ready more than this after the phone's `END` reached the
/// desktop is not pasted: the user has probably moved on to another window
/// or field, and the text would land there. Measured from the reception of
/// `END` (`Queued::at`, the only timestamp we have: the phone sends none),
/// so it covers the queue wait and the transcription, not the relay delay.
/// 20 s leaves room for transcribing a long recording (about 0.2 s per
/// second of audio) while staying well under the 60 s queue limit.
const MAX_PASTE_DELAY: Duration = Duration::from_secs(20);
/// Event telling the interface a transcript was kept out of the paste.
const LATE_EVENT: &str = "remote-mic-late-transcript";
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
    /// The user compared `fingerprint`: `accepted` binds, otherwise refuses.
    Confirm {
        fingerprint: String,
        accepted: bool,
    },
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
            return Err("PhoneMic is already running".to_string());
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
                s.qr = super::qr::encode_svg_data_uri(&link).ok();
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

async fn run(app: AppHandle, client: RelayClient, mut commands: mpsc::UnboundedReceiver<Command>) {
    let ui = Ui { app: &app };
    ui.update(|s| {
        *s = Snapshot {
            status: Status::Starting,
            ..Snapshot::default()
        }
    });
    let recordings = spawn_transcriber(app.clone());
    let error = serve(&client, &mut commands, &ui, &recordings).await.err();
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

/// What a poller reports, tagged with its room id: a room just replaced may
/// still report, and is then ignored.
type Polled = (String, Result<Vec<Frame>, ClientError>);

/// A room whose desktop inbox is being long-polled.
struct Watched {
    room: Room,
    poller: tauri::async_runtime::JoinHandle<()>,
}

impl Watched {
    /// Network errors are retried so a flaky connection does not drop the
    /// session; only a closed room is reported (`RoomGone`), then it stops.
    fn new(client: &RelayClient, room: Room, tx: &mpsc::Sender<Polled>) -> Self {
        let (client, tx, polled) = (client.clone(), tx.clone(), room.clone());
        let poller = tauri::async_runtime::spawn(async move {
            loop {
                let delay = match client.poll(&polled).await {
                    Ok(frames) if frames.is_empty() => continue,
                    Ok(frames) => match tx.send((polled.id.clone(), Ok(frames))).await {
                        Ok(()) => continue,
                        Err(_) => break,
                    },
                    Err(ClientError::RoomGone) => {
                        let _ = tx
                            .send((polled.id.clone(), Err(ClientError::RoomGone)))
                            .await;
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
        Self { room, poller }
    }

    fn is(&self, id: &str) -> bool {
        self.room.id == id
    }

    async fn close(self, client: &RelayClient) {
        self.poller.abort();
        client.close(&self.room).await;
    }
}

/// `open_room`, watched.
async fn open_watched(
    client: &RelayClient,
    commands: &mut mpsc::UnboundedReceiver<Command>,
    tx: &mpsc::Sender<Polled>,
) -> Result<Option<Watched>, String> {
    match open_room(client, commands).await {
        Ok(room) => Ok(room.map(|room| Watched::new(client, room, tx))),
        Err(e) => Err(e.to_string()),
    }
}

/// Posts what the protocol produced. A lost reply is not fatal: `BOUND` is
/// resent, and `RECEIVED`, `stale` or `denied` only update the phone screen.
async fn post(client: &RelayClient, lobby: &Room, private: Option<&Room>, outgoing: Vec<Outgoing>) {
    for ((via, inbox), frames) in by_inbox(outgoing) {
        let room = match via {
            Via::Lobby => lobby,
            Via::Private => match private {
                Some(room) => room,
                None => continue,
            },
        };
        if let Err(e) = client.send(room, &inbox, &frames).await {
            warn!("Remote mic: could not reach the phone: {}", e);
        }
    }
}

/// Runs of consecutive frames for the same inbox, in order: one request each.
fn by_inbox(outgoing: Vec<Outgoing>) -> Vec<((Via, String), Vec<Frame>)> {
    let mut runs: Vec<((Via, String), Vec<Frame>)> = Vec::new();
    for Outgoing { via, inbox, frame } in outgoing {
        match runs.last_mut() {
            Some(((v, last), frames)) if *v == via && *last == inbox => frames.push(frame),
            _ => runs.push(((via, inbox), vec![frame])),
        }
    }
    runs
}

/// What woke the session up.
enum Wake {
    Resend,
    Expiry,
    Command(Option<Command>),
    Polled(Option<Polled>),
}

/// Serves the lobby (the room of the QR code) and the private room of the
/// bound phone until the user stops. After a pairing, once the phone had
/// the time to read `BOUND` and move, the lobby is replaced: a photo of the
/// QR code is then worth nothing. Returns why it stopped, if not asked to.
async fn serve(
    client: &RelayClient,
    commands: &mut mpsc::UnboundedReceiver<Command>,
    ui: &Ui<'_>,
    recordings: &RecordingQueue,
) -> Result<(), String> {
    let mut pairing = Pairing::new().map_err(|e| e.to_string())?;
    let (tx, mut incoming) = mpsc::channel(16);
    let Some(mut lobby) = open_watched(client, commands, &tx).await? else {
        return Ok(());
    };
    let mut private: Option<Watched> = None;
    let mut resend = tokio::time::interval(BOUND_RESEND);
    // `BOUND` resends left, plus one tick that replaces the lobby
    let mut bound_ticks = 0;
    let mut expiry = tokio::time::interval(CANDIDATE_TTL / 6);

    let outcome = loop {
        ui.show(client.origin(), &lobby.room, &pairing);
        let wake = tokio::select! {
            _ = resend.tick(), if bound_ticks > 0 => Wake::Resend,
            _ = expiry.tick() => Wake::Expiry,
            cmd = commands.recv() => Wake::Command(cmd),
            polled = incoming.recv() => Wake::Polled(polled),
        };
        let step = match wake {
            Wake::Resend => {
                bound_ticks -= 1;
                match bound_ticks {
                    0 => {
                        let Some(new) = open_watched(client, commands, &tx).await? else {
                            break Ok(());
                        };
                        std::mem::replace(&mut lobby, new).close(client).await;
                        Ok(Step::default())
                    }
                    _ => pairing.resend_bound().map(|out| Step {
                        send: out.into_iter().collect(),
                        event: None,
                    }),
                }
            }
            Wake::Expiry => {
                if pairing.expire_active(Instant::now()) {
                    info!("Remote mic: the phone was bound too long ago, it must scan again");
                    if let Some(old) = private.take() {
                        old.close(client).await;
                    }
                }
                pairing.expire_candidate(Instant::now())
            }
            Wake::Command(Some(Command::Confirm {
                fingerprint,
                accepted: true,
            })) => {
                // A click about a phone that is no longer the candidate opens nothing
                if pairing.fingerprint() != Some(fingerprint.as_str()) {
                    continue;
                }
                let room = match client.create_room().await {
                    Ok(room) => room,
                    Err(e) => {
                        // The candidate stays: clicking again retries
                        warn!("Remote mic: could not open a room for the phone: {}", e);
                        continue;
                    }
                };
                let link = format!("{}.{}", room.id, room.phone_secret);
                let step = match pairing.confirm(&fingerprint, Some(&link)) {
                    Ok(step) => step,
                    Err(e) => break Err(e.to_string()),
                };
                // REPLACED goes through the private room of the replaced phone
                post(
                    client,
                    &lobby.room,
                    private.as_ref().map(|p| &p.room),
                    step.send,
                )
                .await;
                if let Some(old) = private.replace(Watched::new(client, room, &tx)) {
                    old.close(client).await;
                }
                // A new phone: what the previous one sent is not pasted
                recordings.flush();
                bound_ticks = BOUND_RESENDS + 1;
                resend.reset();
                continue;
            }
            Wake::Command(Some(Command::Confirm {
                fingerprint,
                accepted: false,
            })) => pairing.confirm(&fingerprint, None),
            Wake::Command(Some(Command::Stop) | None) => break Ok(()),
            Wake::Polled(None) => break Err("relay poller stopped".to_string()),
            Wake::Polled(Some((id, result))) => {
                let from_lobby = lobby.is(&id);
                let from_private = private.as_ref().is_some_and(|p| p.is(&id));
                match (result, from_lobby, from_private) {
                    (Ok(frames), true, _) | (Ok(frames), _, true) => {
                        handle_frames(&mut pairing, frames, recordings)
                    }
                    (Err(ClientError::RoomGone), true, _) => {
                        info!("Remote mic: the relay dropped the room, opening a new one");
                        let Some(new) = open_watched(client, commands, &tx).await? else {
                            break Ok(());
                        };
                        lobby = new;
                        Ok(Step::default())
                    }
                    (Err(ClientError::RoomGone), _, true) => {
                        info!("Remote mic: the relay dropped the room of the phone");
                        pairing.unbind();
                        private = None;
                        Ok(Step::default())
                    }
                    (Err(e), true, _) | (Err(e), _, true) => break Err(e.to_string()),
                    // A room replaced meanwhile
                    (_, false, false) => continue,
                }
            }
        };
        match step {
            Ok(step) => {
                let private_room = private.as_ref().map(|p| &p.room);
                post(client, &lobby.room, private_room, step.send).await;
            }
            Err(e) => break Err(e.to_string()),
        }
    };
    lobby.close(client).await;
    if let Some(private) = private {
        private.close(client).await;
    }
    outcome
}

/// Feeds a batch to the protocol, queues the recordings, merges the replies.
fn handle_frames(
    pairing: &mut Pairing,
    frames: Vec<Frame>,
    recordings: &RecordingQueue,
) -> Result<Step, super::crypto::CryptoError> {
    let mut send: Vec<Outgoing> = Vec::new();
    for frame in frames {
        let step = pairing.on_frame(frame)?;
        // The only plain-text reply is `stale`, which anyone holding an old
        // QR code can trigger: one per batch, or a 1 MiB envelope of hellos
        // would make us post thousands of answers in a row
        let answered = send.iter().any(|o| matches!(o.frame, Frame::Text(_)));
        send.extend(
            step.send
                .into_iter()
                .filter(|o| !(answered && matches!(o.frame, Frame::Text(_)))),
        );
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

/// Whether a transcript is still worth pasting, `delay` after the end of the
/// recording reached the desktop.
fn paste_is_timely(delay: Duration) -> bool {
    delay <= MAX_PASTE_DELAY
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
                pcm,
                sample_rate,
                at,
                ..
            } = item;
            let _ = tauri::async_runtime::spawn_blocking(move || {
                match transcribe_pcm(&app, pcm, sample_rate) {
                    Ok(text) if !text.is_empty() && !paste_is_timely(at.elapsed()) => {
                        // Already in the history (`transcribe_pcm`), just not pasted
                        info!("Remote mic: transcript too late, not pasted");
                        let _ = app.emit(LATE_EVENT, ());
                    }
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
    fn replies_are_grouped_per_inbox_in_order() {
        let out = |via: Via, inbox: &str, n: u8| Outgoing {
            via,
            inbox: inbox.into(),
            frame: Frame::Binary(vec![n]),
        };
        let (l, p) = (Via::Lobby, Via::Private);
        let runs = by_inbox(vec![
            out(l, "a", 1),
            out(l, "a", 2),
            out(l, "b", 3),
            out(l, "a", 4),
            // Same tag, other room: another request
            out(p, "a", 5),
        ]);
        let bin = |n: u8| Frame::Binary(vec![n]);
        let key = |via: Via, inbox: &str| (via, inbox.to_string());
        assert_eq!(
            runs,
            vec![
                (key(l, "a"), vec![bin(1), bin(2)]),
                (key(l, "b"), vec![bin(3)]),
                (key(l, "a"), vec![bin(4)]),
                (key(p, "a"), vec![bin(5)]),
            ]
        );
    }

    #[test]
    fn one_stale_answer_per_batch() {
        use base64::Engine;
        // A well-formed point that is not the key on the QR code
        let point: Vec<u8> = std::iter::once(4).chain([1; 64]).collect();
        let key = base64::engine::general_purpose::STANDARD.encode(point);
        let hellos = ["A", "B", "A", "B"]
            .iter()
            .map(|t| {
                let hello = serde_json::json!({
                    "type": "hello", "key": key, "to": "x", "inbox": t.repeat(22),
                });
                Frame::Text(hello.to_string())
            })
            .collect();
        let mut pairing = Pairing::new().unwrap();
        let (queue, _rx, _) = RecordingQueue::new();
        let step = handle_frames(&mut pairing, hellos, &queue).unwrap();
        assert_eq!(step.send.len(), 1);
    }

    #[test]
    fn a_late_transcript_is_not_pasted() {
        assert!(paste_is_timely(Duration::ZERO));
        assert!(paste_is_timely(MAX_PASTE_DELAY));
        assert!(!paste_is_timely(MAX_PASTE_DELAY + Duration::from_millis(1)));
        assert!(!paste_is_timely(MAX_QUEUE_AGE));
        assert!(MAX_PASTE_DELAY < MAX_QUEUE_AGE);
    }

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
