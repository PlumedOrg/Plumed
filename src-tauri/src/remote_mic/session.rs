//! One remote-mic session: room, QR code, pairing, then transcription of each
//! recording sent by the phone. Everything stays in RAM and dies with the session.
//!
//! Lifecycle (`run`):
//! - create a room, post the dummy offer, show the QR code,
//! - claim a relay slot and pair (`run_slot`); when the phone leaves, pair again
//!   on the same room with a fresh key (the same QR code keeps working),
//! - the server deletes a room 10 min after its creation: if a phone was paired in
//!   it we open a new room (new QR code), otherwise the session stops.

use super::client::{phone_link, ClientError, Frame, RelayClient, Room};
use super::crypto::KeyPair;
use super::protocol::{Event, Pairing};
use super::sdp::dummy_offer;
use crate::smartmic::audio_bridge::transcribe_pcm;
use log::{info, warn};
use parking_lot::Mutex;
use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

/// Our `public-key` is resent at this pace until the phone answers.
const KEY_RESEND: Duration = Duration::from_secs(2);
/// Pause before retrying after a network error or a full room.
const RETRY_DELAY: Duration = Duration::from_secs(3);
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

    fn waiting(&self, origin: &str, room: &Room) {
        let link = phone_link(origin, room);
        let qr = crate::smartmic::qr::encode_svg_data_uri(&link).ok();
        self.update(|s| {
            *s = Snapshot {
                status: Status::Waiting,
                qr,
                link: Some(link),
                ..Snapshot::default()
            }
        });
    }

    fn status(&self, status: Status) {
        self.update(|s| s.status = status);
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

/// How one pairing on a relay slot ended.
enum SlotEnd {
    /// Phone left, refused, or broke the protocol: pair again on the same room.
    PhoneLeft {
        was_bound: bool,
    },
    RoomGone {
        was_bound: bool,
    },
    Stop,
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
        let room = match open_room(&client).await {
            Ok(room) => room,
            Err(e) => break Some(e.to_string()),
        };
        ui.waiting(client.origin(), &room);
        match serve_room(&client, &room, &mut commands, &ui, &recordings).await {
            RoomOutcome::Reopen => {
                info!("Remote mic: room expired after a session, opening a new one")
            }
            RoomOutcome::Expired => break Some("QR code expired: no phone joined".to_string()),
            RoomOutcome::Stop => break None,
            RoomOutcome::Failed(e) => break Some(e),
        }
    };
    // Close the channel so a new `start` is accepted before the UI hears "stopped"
    commands.close();
    ui.stopped(error);
}

enum RoomOutcome {
    Reopen,
    Expired,
    Stop,
    Failed(String),
}

async fn serve_room(
    client: &RelayClient,
    room: &Room,
    commands: &mut mpsc::UnboundedReceiver<Command>,
    ui: &Ui<'_>,
    recordings: &mpsc::UnboundedSender<(Vec<i16>, u32)>,
) -> RoomOutcome {
    let mut paired_once = false;
    loop {
        let token = match claim_slot(client, room, commands).await {
            Ok(Some(token)) => token,
            Ok(None) => return RoomOutcome::Stop,
            Err(ClientError::RoomGone) => {
                return reopen_or_expire(paired_once);
            }
            Err(e) => return RoomOutcome::Failed(e.to_string()),
        };
        let end = run_slot(client, room, &token, commands, ui, recordings).await;
        client.close_slot(room, &token).await;
        match end {
            SlotEnd::PhoneLeft { was_bound } => {
                paired_once |= was_bound;
                ui.waiting(client.origin(), room);
            }
            SlotEnd::RoomGone { was_bound } => return reopen_or_expire(paired_once || was_bound),
            SlotEnd::Stop => return RoomOutcome::Stop,
        }
    }
}

fn reopen_or_expire(paired_once: bool) -> RoomOutcome {
    match paired_once {
        true => RoomOutcome::Reopen,
        false => RoomOutcome::Expired,
    }
}

async fn open_room(client: &RelayClient) -> Result<Room, ClientError> {
    let room = loop {
        match client.create_room().await {
            Err(ClientError::RateLimited(secs)) => {
                tokio::time::sleep(Duration::from_secs(secs.min(60))).await
            }
            other => break other?,
        }
    };
    client.post_offer(&room, &dummy_offer()).await?;
    Ok(room)
}

/// Claims a slot, waiting while the room is full (the old phone slot is reaped
/// by the server after 60 s idle). `Ok(None)` means the user stopped meanwhile.
async fn claim_slot(
    client: &RelayClient,
    room: &Room,
    commands: &mut mpsc::UnboundedReceiver<Command>,
) -> Result<Option<String>, ClientError> {
    loop {
        let delay = match client.claim_slot(room).await {
            Ok(token) => return Ok(Some(token)),
            Err(ClientError::SlotClosed) | Err(ClientError::Network(_)) => RETRY_DELAY,
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
/// connection does not drop the pairing (the server keeps the slot 60 s).
fn spawn_poller(
    client: RelayClient,
    room: Room,
    token: String,
) -> (
    mpsc::Receiver<Result<Frame, ClientError>>,
    tauri::async_runtime::JoinHandle<()>,
) {
    let (tx, rx) = mpsc::channel(64);
    let task = tauri::async_runtime::spawn(async move {
        loop {
            let item = match client.receive(&room, &token).await {
                Ok(Some(frame)) => Ok(frame),
                Ok(None) => continue,
                Err(ClientError::RateLimited(secs)) => {
                    tokio::time::sleep(Duration::from_secs(secs.min(60))).await;
                    continue;
                }
                Err(ClientError::Network(e)) => {
                    warn!("Remote mic: relay poll failed, retrying: {}", e);
                    tokio::time::sleep(RETRY_DELAY).await;
                    continue;
                }
                Err(e) => Err(e),
            };
            let fatal = item.is_err();
            if tx.send(item).await.is_err() || fatal {
                break;
            }
        }
    });
    (rx, task)
}

async fn run_slot(
    client: &RelayClient,
    room: &Room,
    token: &str,
    commands: &mut mpsc::UnboundedReceiver<Command>,
    ui: &Ui<'_>,
    recordings: &mpsc::UnboundedSender<(Vec<i16>, u32)>,
) -> SlotEnd {
    let keypair = match KeyPair::generate() {
        Ok(kp) => kp,
        Err(_) => return SlotEnd::Stop,
    };
    let mut pairing = Pairing::new(keypair);
    let (mut frames, poller) = spawn_poller(client.clone(), room.clone(), token.to_string());
    let mut resend = tokio::time::interval(KEY_RESEND);
    let send = |text: String| client.send_text(room, token, text);

    let end = loop {
        let was_bound = pairing.is_bound();
        tokio::select! {
            _ = resend.tick(), if !pairing.has_peer_key() => {
                match send(pairing.public_key_message()).await {
                    Err(ClientError::RoomGone) => break SlotEnd::RoomGone { was_bound },
                    Err(ClientError::SlotClosed) => break SlotEnd::PhoneLeft { was_bound },
                    // Network hiccup or rate limit: the next tick retries
                    _ => {}
                }
            }
            cmd = commands.recv() => match cmd {
                Some(Command::Confirm(true)) if pairing.has_peer_key() => {
                    let _ = send(r#"{"type":"verify-ok"}"#.to_string()).await;
                    if pairing.confirm() == Some(Event::Bound) {
                        ui.status(Status::Connected);
                    }
                }
                Some(Command::Confirm(false)) if pairing.has_peer_key() => {
                    let _ = send(r#"{"type":"verify-deny"}"#.to_string()).await;
                    break SlotEnd::PhoneLeft { was_bound };
                }
                Some(Command::Confirm(_)) => {}
                Some(Command::Stop) | None => break SlotEnd::Stop,
            },
            frame = frames.recv() => {
                let event = match frame {
                    Some(Ok(Frame::Text(text))) => pairing.on_text(&text),
                    Some(Ok(Frame::Binary(packet))) => pairing.on_binary(&packet),
                    Some(Err(ClientError::RoomGone)) => break SlotEnd::RoomGone { was_bound },
                    Some(Err(_)) | None => break SlotEnd::PhoneLeft { was_bound },
                };
                match event {
                    Some(Event::Fingerprint(fingerprint)) => ui.update(|s| {
                        s.status = Status::Verify;
                        s.fingerprint = Some(fingerprint);
                    }),
                    Some(Event::Bound) => ui.status(Status::Connected),
                    Some(Event::Recording { pcm, sample_rate }) => {
                        let _ = recordings.send((pcm, sample_rate));
                    }
                    Some(Event::Denied) => break SlotEnd::PhoneLeft { was_bound },
                    Some(Event::ProtocolError(why)) => {
                        warn!("Remote mic: dropping the phone: {}", why);
                        break SlotEnd::PhoneLeft { was_bound };
                    }
                    None => {}
                }
            }
        }
    };
    poller.abort();
    end
}

/// Transcribes and pastes recordings one at a time, in arrival order.
fn spawn_transcriber(app: AppHandle) -> mpsc::UnboundedSender<(Vec<i16>, u32)> {
    let (tx, mut rx) = mpsc::unbounded_channel::<(Vec<i16>, u32)>();
    tauri::async_runtime::spawn(async move {
        while let Some((pcm, sample_rate)) = rx.recv().await {
            let app = app.clone();
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
    tx
}
