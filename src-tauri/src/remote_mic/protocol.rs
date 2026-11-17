//! Desktop side of the Plumed remote-mic protocol, without I/O.
//!
//! The QR code carries the public key of `pending`. A phone that scanned it posts
//! `{"type":"hello","key":<phone pub>,"to":<that key>,"inbox":<tag>}`: it becomes the
//! `candidate`, both screens show the same fingerprint, and only the desktop user
//! confirms. The confirmed phone becomes `active` and gets an encrypted `BOUND`
//! (a refused one gets an encrypted `DENIED`, a replaced one `REPLACED`).
//! From then on every frame is `IV || AES-GCM(seq || op || payload)`.
//! Every answer goes to the inbox the phone named in its hello.
//!
//! Each hello consumes the pending key (ring's ECDH keys are single use), so a
//! new one is generated and the QR code changes: the old code answers `stale`.
//! Scanning the new code while a phone is active replaces it after confirmation.
//!
//! Sequence numbers must be contiguous inside a recording: a gap (the relay
//! withheld a chunk or the END of the previous one) drops the recording.

use super::crypto::{decode_public_key, pair_fingerprint, CryptoError, KeyPair, SessionKeys};
use crate::smartmic::audio_bridge::accumulate_pcm;
use plumed_relay_protocol::{
    op, open_inner, seal_inner, valid_inbox_tag, Frame, Role, MAX_FRAME, SAMPLE_RATES,
};
use serde::Deserialize;
use std::time::{Duration, Instant};

const MAX_TEXT_FRAME: usize = 4096;
/// A phone not confirmed within this delay is refused: an unattended desktop
/// must not keep a stranger's pairing request one click away.
pub const CANDIDATE_TTL: Duration = Duration::from_secs(60);

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum PhoneMessage {
    Hello {
        key: String,
        to: String,
        inbox: String,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, PartialEq)]
pub enum Event {
    /// A phone said hello: ask the user whether it shows the same code.
    Fingerprint(String),
    /// A complete recording, mono pcm-s16 at `sample_rate`.
    Recording { pcm: Vec<i16>, sample_rate: u32 },
    /// The user's click bound a new phone.
    Paired,
}

/// A frame for the phone inbox tagged `inbox`.
#[derive(Debug, PartialEq)]
pub struct Outgoing {
    pub inbox: String,
    pub frame: Frame,
}

/// What one input produced: frames to post to the relay, and maybe an event.
#[derive(Debug, Default, PartialEq)]
pub struct Step {
    pub send: Vec<Outgoing>,
    pub event: Option<Event>,
}

impl Step {
    fn send(inbox: &str, frame: Frame) -> Self {
        Self {
            send: vec![Outgoing {
                inbox: inbox.to_string(),
                frame,
            }],
            event: None,
        }
    }
}

struct Candidate {
    peer: Peer,
    fingerprint: String,
    since: Instant,
}

/// A phone we share keys with.
struct Peer {
    key: SessionKeys,
    inbox: String,
    seq_in: u64,
    seq_out: u64,
    recording: Option<(Vec<i16>, u32)>,
}

impl Peer {
    fn seal(&mut self, op: u8, payload: &[u8]) -> Result<Outgoing, CryptoError> {
        self.seq_out += 1;
        let packet = self.key.seal(&seal_inner(self.seq_out, op, payload))?;
        Ok(Outgoing {
            inbox: self.inbox.clone(),
            frame: Frame::Binary(packet),
        })
    }
}

pub struct Pairing {
    pending: KeyPair,
    candidate: Option<Candidate>,
    active: Option<Peer>,
}

/// Unencrypted answer to a hello on an old QR code (no key to seal it with).
/// `to` names the key it answers: the phone ignores it otherwise.
fn stale(inbox: &str, phone_b64: &str) -> Step {
    let text = serde_json::json!({ "type": "stale", "to": phone_b64 }).to_string();
    Step::send(inbox, Frame::Text(text))
}

impl Pairing {
    pub fn new() -> Result<Self, CryptoError> {
        Ok(Self {
            pending: KeyPair::generate()?,
            candidate: None,
            active: None,
        })
    }

    /// Public key for the QR link fragment.
    pub fn link_key(&self) -> String {
        self.pending.public_b64url()
    }

    pub fn is_bound(&self) -> bool {
        self.active.is_some()
    }

    pub fn awaits_confirmation(&self) -> bool {
        self.candidate.is_some()
    }

    /// Errors only when the system cannot generate keys or randomness.
    pub fn on_frame(&mut self, frame: Frame) -> Result<Step, CryptoError> {
        match frame {
            Frame::Text(text) => self.on_text(&text),
            Frame::Binary(packet) => self.on_binary(&packet),
        }
    }

    fn on_text(&mut self, text: &str) -> Result<Step, CryptoError> {
        if text.len() > MAX_TEXT_FRAME {
            return Ok(Step::default());
        }
        match serde_json::from_str(text) {
            Ok(PhoneMessage::Hello { key, to, inbox }) if valid_inbox_tag(&inbox) => {
                self.on_hello(key, &to, inbox)
            }
            Ok(PhoneMessage::Hello { .. }) => Ok(Step::default()),
            Ok(PhoneMessage::Other) | Err(_) => Ok(Step::default()),
        }
    }

    fn on_hello(
        &mut self,
        phone_b64: String,
        to: &str,
        inbox: String,
    ) -> Result<Step, CryptoError> {
        let Ok(phone_raw) = decode_public_key(&phone_b64) else {
            return Ok(Step::default());
        };
        if to != self.pending.public_b64() {
            return Ok(stale(&inbox, &phone_b64));
        }
        let ours = std::mem::replace(&mut self.pending, KeyPair::generate()?);
        let fingerprint = pair_fingerprint(ours.public_raw(), &phone_raw);
        // Not on the curve: a forged hello, the QR code simply changes
        let Ok(key) = ours.derive(&phone_raw, Role::Desktop) else {
            return Ok(Step::default());
        };
        self.candidate = Some(Candidate {
            peer: Peer {
                key,
                inbox,
                seq_in: 0,
                seq_out: 0,
                recording: None,
            },
            fingerprint: fingerprint.clone(),
            since: Instant::now(),
        });
        Ok(Step {
            send: vec![],
            event: Some(Event::Fingerprint(fingerprint)),
        })
    }

    /// The local user compared `fingerprint`: `true` binds the candidate
    /// (replacing any active phone), `false` refuses it. Ignored when another
    /// phone said hello since the code was shown: the click was not about it.
    pub fn confirm(&mut self, fingerprint: &str, accepted: bool) -> Result<Step, CryptoError> {
        if self.fingerprint() != Some(fingerprint) {
            return Ok(Step::default());
        }
        let Some(Candidate { mut peer, .. }) = self.candidate.take() else {
            return Ok(Step::default());
        };
        let (send, event) = match accepted {
            true => {
                let replaced = match self.active.take() {
                    Some(mut old) => Some(old.seal(op::REPLACED, &[])?),
                    None => None,
                };
                let bound = peer.seal(op::BOUND, &[])?;
                self.active = Some(peer);
                let send = replaced.into_iter().chain([bound]).collect();
                (send, Some(Event::Paired))
            }
            false => (vec![peer.seal(op::DENIED, &[])?], None),
        };
        Ok(Step { send, event })
    }

    /// Refuses a candidate waiting since more than [`CANDIDATE_TTL`] at `now`.
    pub fn expire_candidate(&mut self, now: Instant) -> Result<Step, CryptoError> {
        match &self.candidate {
            Some(c) if now.duration_since(c.since) > CANDIDATE_TTL => {
                let fingerprint = c.fingerprint.clone();
                self.confirm(&fingerprint, false)
            }
            _ => Ok(Step::default()),
        }
    }

    /// The fingerprint being compared, if any.
    pub fn fingerprint(&self) -> Option<&str> {
        self.candidate.as_ref().map(|c| c.fingerprint.as_str())
    }

    /// Seals one op for the active phone (`None` when no phone is bound).
    pub fn seal_op(&mut self, op: u8) -> Result<Option<Outgoing>, CryptoError> {
        self.active.as_mut().map(|a| a.seal(op, &[])).transpose()
    }

    fn on_binary(&mut self, packet: &[u8]) -> Result<Step, CryptoError> {
        let Some(active) = self.active.as_mut() else {
            return Ok(Step::default());
        };
        if packet.len() > MAX_FRAME {
            return Ok(Step::default());
        }
        // Another phone of the room, a forgery, or a replay: dropped silently
        let Ok(plain) = active.key.decrypt(packet) else {
            return Ok(Step::default());
        };
        let Some((seq, code, payload)) = open_inner(&plain) else {
            return Ok(Step::default());
        };
        if seq <= active.seq_in {
            return Ok(Step::default());
        }
        // A gap means the relay withheld frames: never transcribe a recording
        // with a hole in it (a dropped "pas" flips the meaning of a sentence)
        if seq != active.seq_in + 1 {
            active.recording = None;
        }
        active.seq_in = seq;
        match code {
            op::START => {
                // A START without END (phone reconnected) drops the unfinished one
                active.recording = <[u8; 4]>::try_from(payload)
                    .ok()
                    .map(u32::from_be_bytes)
                    .filter(|rate| SAMPLE_RATES.contains(rate))
                    .map(|rate| (Vec::new(), rate));
            }
            op::AUDIO => {
                if let Some((pcm, _)) = active.recording.as_mut() {
                    // A full buffer stops growing: the recording is cut at the cap
                    accumulate_pcm(pcm, payload);
                }
            }
            op::END => {
                if let Some((pcm, sample_rate)) = active.recording.take() {
                    let received = active.seal(op::RECEIVED, &[])?;
                    let event = (!pcm.is_empty()).then_some(Event::Recording { pcm, sample_rate });
                    return Ok(Step {
                        send: vec![received],
                        event,
                    });
                }
            }
            op::CANCEL => active.recording = None,
            _ => {}
        }
        Ok(Step::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64URL;
    use base64::Engine;

    /// A fake phone that scanned the QR code.
    struct Phone {
        public_b64: String,
        desktop_b64: String,
        inbox: String,
        key: SessionKeys,
        seq: u64,
    }

    impl Phone {
        fn scan(p: &Pairing) -> Self {
            let desktop_raw = B64URL.decode(p.link_key()).unwrap();
            let kp = KeyPair::generate().unwrap();
            let public_b64 = kp.public_b64();
            let inbox = B64URL.encode(&kp.public_raw()[1..17]);
            let desktop_b64 = base64::engine::general_purpose::STANDARD.encode(&desktop_raw);
            let key = kp.derive(&desktop_raw, Role::Phone).unwrap();
            Phone {
                public_b64,
                desktop_b64,
                inbox,
                key,
                seq: 0,
            }
        }

        fn hello(&self) -> Frame {
            let hello = serde_json::json!({
                "type": "hello", "key": self.public_b64, "to": self.desktop_b64, "inbox": self.inbox,
            });
            Frame::Text(hello.to_string())
        }

        fn op(&mut self, code: u8, payload: &[u8]) -> Frame {
            self.seq += 1;
            Frame::Binary(self.key.seal(&seal_inner(self.seq, code, payload)).unwrap())
        }

        /// -> (seq, op) of a desktop frame, `None` if not for this phone.
        fn open(&self, out: &Outgoing) -> Option<(u64, u8)> {
            let Frame::Binary(packet) = &out.frame else {
                return None;
            };
            if out.inbox != self.inbox {
                return None;
            };
            let plain = self.key.decrypt(packet).ok()?;
            open_inner(&plain).map(|(seq, code, _)| (seq, code))
        }
    }

    fn text(out: &Outgoing) -> serde_json::Value {
        match &out.frame {
            Frame::Text(t) => serde_json::from_str(t).unwrap(),
            Frame::Binary(_) => panic!("expected text"),
        }
    }

    fn bound() -> (Pairing, Phone) {
        let mut p = Pairing::new().unwrap();
        let phone = Phone::scan(&p);
        let step = p.on_frame(phone.hello()).unwrap();
        let Some(Event::Fingerprint(code)) = step.event else {
            panic!("no fingerprint")
        };
        let step = p.confirm(&code, true).unwrap();
        assert_eq!(phone.open(&step.send[0]), Some((1, op::BOUND)));
        assert!(p.is_bound());
        (p, phone)
    }

    #[test]
    fn hello_shows_the_fingerprint_both_sides_compute() {
        let mut p = Pairing::new().unwrap();
        let qr_key = p.link_key();
        let phone = Phone::scan(&p);
        let step = p.on_frame(phone.hello()).unwrap();
        let desktop_raw = B64URL.decode(&qr_key).unwrap();
        let phone_raw = decode_public_key(&phone.public_b64).unwrap();
        let expected = pair_fingerprint(&desktop_raw, &phone_raw);
        assert_eq!(step.event, Some(Event::Fingerprint(expected.clone())));
        assert_eq!(p.fingerprint(), Some(expected.as_str()));
        // The key was consumed: the QR code changed
        assert_ne!(p.link_key(), qr_key);
        assert!(!p.is_bound());
    }

    #[test]
    fn an_old_qr_code_gets_an_addressed_stale() {
        let mut p = Pairing::new().unwrap();
        let first = Phone::scan(&p);
        p.on_frame(first.hello()).unwrap();
        // Same (now consumed) QR code scanned by a second phone
        let late = Phone {
            desktop_b64: first.desktop_b64.clone(),
            ..Phone::scan(&p)
        };
        let step = p.on_frame(late.hello()).unwrap();
        assert_eq!(step.send[0].inbox, late.inbox);
        let answer = text(&step.send[0]);
        assert_eq!(answer["type"], "stale");
        assert_eq!(answer["to"], late.public_b64.as_str());
        assert_eq!(step.event, None);
    }

    #[test]
    fn refusal_sends_an_authenticated_denied() {
        let mut p = Pairing::new().unwrap();
        let phone = Phone::scan(&p);
        p.on_frame(phone.hello()).unwrap();
        let code = p.fingerprint().unwrap().to_string();
        let step = p.confirm(&code, false).unwrap();
        assert_eq!(phone.open(&step.send[0]), Some((1, op::DENIED)));
        assert!(!p.is_bound() && !p.awaits_confirmation());
        // Nothing left to confirm
        assert_eq!(p.confirm(&code, true).unwrap(), Step::default());
    }

    #[test]
    fn a_click_about_a_replaced_candidate_is_ignored() {
        let mut p = Pairing::new().unwrap();
        let first = Phone::scan(&p);
        p.on_frame(first.hello()).unwrap();
        let shown = p.fingerprint().unwrap().to_string();
        // An attacker says hello on the new QR code right before the click
        let attacker = Phone::scan(&p);
        p.on_frame(attacker.hello()).unwrap();
        assert_eq!(p.confirm(&shown, true).unwrap(), Step::default());
        assert!(!p.is_bound());
        assert!(p.awaits_confirmation());
    }

    #[test]
    fn an_unconfirmed_candidate_expires() {
        let mut p = Pairing::new().unwrap();
        let phone = Phone::scan(&p);
        p.on_frame(phone.hello()).unwrap();
        let now = Instant::now();
        assert_eq!(p.expire_candidate(now).unwrap(), Step::default());
        assert!(p.awaits_confirmation());
        let later = now + CANDIDATE_TTL + Duration::from_secs(1);
        let step = p.expire_candidate(later).unwrap();
        assert_eq!(phone.open(&step.send[0]), Some((1, op::DENIED)));
        assert!(!p.awaits_confirmation());
    }

    #[test]
    fn a_recording_is_returned_on_end_and_acknowledged() {
        let (mut p, mut phone) = bound();
        let start = phone.op(op::START, &16000u32.to_be_bytes());
        assert_eq!(p.on_frame(start).unwrap(), Step::default());
        p.on_frame(phone.op(op::AUDIO, &[0x00, 0x00, 0xff, 0x7f]))
            .unwrap();
        p.on_frame(phone.op(op::AUDIO, &[0x00, 0x80])).unwrap();
        let step = p.on_frame(phone.op(op::END, &[])).unwrap();
        assert_eq!(
            step.event,
            Some(Event::Recording {
                pcm: vec![0, 32767, -32768],
                sample_rate: 16000
            })
        );
        assert_eq!(phone.open(&step.send[0]), Some((2, op::RECEIVED)));
    }

    #[test]
    fn replays_cancel_and_bad_rates_are_handled() {
        let (mut p, mut phone) = bound();
        p.on_frame(phone.op(op::START, &16000u32.to_be_bytes()))
            .unwrap();
        let audio = phone.op(op::AUDIO, &[1, 0]);
        p.on_frame(audio.clone()).unwrap();
        // The same packet again (a retried POST) is not counted twice
        p.on_frame(audio).unwrap();
        let step = p.on_frame(phone.op(op::END, &[])).unwrap();
        assert_eq!(
            step.event,
            Some(Event::Recording {
                pcm: vec![1],
                sample_rate: 16000
            })
        );
        // Cancelled utterance: no recording, no acknowledgement
        p.on_frame(phone.op(op::START, &16000u32.to_be_bytes()))
            .unwrap();
        p.on_frame(phone.op(op::AUDIO, &[1, 0])).unwrap();
        p.on_frame(phone.op(op::CANCEL, &[])).unwrap();
        assert_eq!(p.on_frame(phone.op(op::END, &[])).unwrap(), Step::default());
        // Unsupported rate: the audio after it is ignored
        p.on_frame(phone.op(op::START, &200_000u32.to_be_bytes()))
            .unwrap();
        p.on_frame(phone.op(op::AUDIO, &[1, 0])).unwrap();
        assert_eq!(p.on_frame(phone.op(op::END, &[])).unwrap(), Step::default());
    }

    #[test]
    fn a_withheld_frame_drops_the_recording() {
        let (mut p, mut phone) = bound();
        p.on_frame(phone.op(op::START, &16000u32.to_be_bytes()))
            .unwrap();
        p.on_frame(phone.op(op::AUDIO, &[1, 0])).unwrap();
        // The relay swallows this chunk
        let _withheld = phone.op(op::AUDIO, &[2, 0]);
        p.on_frame(phone.op(op::AUDIO, &[3, 0])).unwrap();
        assert_eq!(p.on_frame(phone.op(op::END, &[])).unwrap(), Step::default());
        // A withheld END: the next recording starts clean, gaps between
        // recordings are harmless
        p.on_frame(phone.op(op::START, &16000u32.to_be_bytes()))
            .unwrap();
        p.on_frame(phone.op(op::AUDIO, &[4, 0])).unwrap();
        let _withheld_end = phone.op(op::END, &[]);
        p.on_frame(phone.op(op::START, &16000u32.to_be_bytes()))
            .unwrap();
        p.on_frame(phone.op(op::AUDIO, &[5, 0])).unwrap();
        let step = p.on_frame(phone.op(op::END, &[])).unwrap();
        assert_eq!(
            step.event,
            Some(Event::Recording {
                pcm: vec![5],
                sample_rate: 16000
            })
        );
    }

    #[test]
    fn audio_before_binding_or_from_strangers_is_ignored() {
        let mut p = Pairing::new().unwrap();
        let mut phone = Phone::scan(&p);
        p.on_frame(phone.hello()).unwrap();
        assert_eq!(
            p.on_frame(phone.op(op::START, &16000u32.to_be_bytes()))
                .unwrap(),
            Step::default()
        );
        let code = p.fingerprint().unwrap().to_string();
        p.confirm(&code, true).unwrap();
        // A frame sealed with an unrelated key
        let mut stranger = Phone::scan(&Pairing::new().unwrap());
        assert_eq!(
            p.on_frame(stranger.op(op::END, &[])).unwrap(),
            Step::default()
        );
        assert_eq!(
            p.on_frame(Frame::Text("x".repeat(5000))).unwrap(),
            Step::default()
        );
        assert_eq!(
            p.on_frame(Frame::Text("not json".into())).unwrap(),
            Step::default()
        );
        // A hello without a valid inbox cannot be answered: ignored
        let qr_key = p.link_key();
        let mut nameless = Phone::scan(&p);
        nameless.inbox = "a/b".into();
        assert_eq!(p.on_frame(nameless.hello()).unwrap(), Step::default());
        assert_eq!(p.link_key(), qr_key);
    }

    #[test]
    fn scanning_the_new_code_replaces_the_active_phone() {
        let (mut p, old) = bound();
        let mut new = Phone::scan(&p);
        let Some(Event::Fingerprint(code)) = p.on_frame(new.hello()).unwrap().event else {
            panic!("no fingerprint")
        };
        let step = p.confirm(&code, true).unwrap();
        assert_eq!(step.event, Some(Event::Paired));
        // The old phone learns it was replaced, the new one is bound
        assert_eq!(old.open(&step.send[0]), Some((2, op::REPLACED)));
        assert_eq!(new.open(&step.send[1]), Some((1, op::BOUND)));
        p.on_frame(new.op(op::START, &16000u32.to_be_bytes()))
            .unwrap();
        p.on_frame(new.op(op::AUDIO, &[2, 0])).unwrap();
        assert!(p.on_frame(new.op(op::END, &[])).unwrap().event.is_some());
    }

    #[test]
    fn an_ignored_click_does_not_report_a_new_phone() {
        // The session flushes queued recordings on `Paired`: a stale click
        // while a phone is bound must not throw away that phone's audio
        let (mut p, _active) = bound();
        p.on_frame(Phone::scan(&p).hello()).unwrap();
        let shown = p.fingerprint().unwrap().to_string();
        p.on_frame(Phone::scan(&p).hello()).unwrap();
        assert!(p.is_bound());
        assert_eq!(p.confirm(&shown, true).unwrap().event, None);
    }

    #[test]
    fn seal_op_needs_a_bound_phone() {
        assert_eq!(Pairing::new().unwrap().seal_op(op::BOUND).unwrap(), None);
        let (mut p, phone) = bound();
        let frame = p.seal_op(op::BOUND).unwrap().unwrap();
        assert_eq!(phone.open(&frame), Some((2, op::BOUND)));
    }
}
