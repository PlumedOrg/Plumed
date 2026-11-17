//! Desktop side of the Plumed remote-mic protocol, without I/O.
//!
//! The QR code carries the public key of `pending`. A phone that scanned it posts
//! `{"type":"hello","key":<phone pub>,"to":<that key>}`: it becomes the
//! `candidate`, both screens show the same fingerprint, and only the desktop user
//! confirms. The confirmed phone becomes `active` and gets an encrypted `BOUND`.
//! From then on every frame is `IV || AES-GCM(seq || op || payload)`.
//!
//! Each hello consumes the pending key (ring's ECDH keys are single use), so a
//! new one is generated and the QR code changes: the old code answers `stale`.
//! Scanning the new code while a phone is active replaces it after confirmation.

use super::crypto::{decode_public_key, pair_fingerprint, CryptoError, KeyPair, SessionKey};
use crate::smartmic::audio_bridge::accumulate_pcm;
use plumed_relay_protocol::{op, open_inner, seal_inner, Frame, MAX_FRAME, SAMPLE_RATES};
use serde::Deserialize;

const MAX_TEXT_FRAME: usize = 4096;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum PhoneMessage {
    Hello {
        key: String,
        to: String,
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
}

/// What one input produced: frames to post to the relay, and maybe an event.
#[derive(Debug, Default, PartialEq)]
pub struct Step {
    pub send: Vec<Frame>,
    pub event: Option<Event>,
}

impl Step {
    fn send(frame: Frame) -> Self {
        Self {
            send: vec![frame],
            event: None,
        }
    }
}

struct Candidate {
    key: SessionKey,
    phone_b64: String,
    fingerprint: String,
}

struct Active {
    key: SessionKey,
    seq_in: u64,
    seq_out: u64,
    recording: Option<(Vec<i16>, u32)>,
}

impl Active {
    fn seal(&mut self, op: u8, payload: &[u8]) -> Result<Frame, CryptoError> {
        self.seq_out += 1;
        let packet = self.key.seal(&seal_inner(self.seq_out, op, payload))?;
        Ok(Frame::Binary(packet))
    }
}

pub struct Pairing {
    pending: KeyPair,
    candidate: Option<Candidate>,
    active: Option<Active>,
}

/// Unencrypted answer to one phone's hello. `to` lets the other phones of the
/// room (they all poll the same mailbox) ignore it.
fn reply(kind: &str, phone_b64: &str) -> Frame {
    Frame::Text(serde_json::json!({ "type": kind, "to": phone_b64 }).to_string())
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
            Ok(PhoneMessage::Hello { key, to }) => self.on_hello(key, &to),
            Ok(PhoneMessage::Other) | Err(_) => Ok(Step::default()),
        }
    }

    fn on_hello(&mut self, phone_b64: String, to: &str) -> Result<Step, CryptoError> {
        let Ok(phone_raw) = decode_public_key(&phone_b64) else {
            return Ok(Step::default());
        };
        if to != self.pending.public_b64() {
            return Ok(Step::send(reply("stale", &phone_b64)));
        }
        let ours = std::mem::replace(&mut self.pending, KeyPair::generate()?);
        let fingerprint = pair_fingerprint(ours.public_raw(), &phone_raw);
        // Not on the curve: a forged hello, the QR code simply changes
        let Ok(key) = ours.derive(&phone_raw) else {
            return Ok(Step::default());
        };
        self.candidate = Some(Candidate {
            key,
            phone_b64,
            fingerprint: fingerprint.clone(),
        });
        Ok(Step {
            send: vec![],
            event: Some(Event::Fingerprint(fingerprint)),
        })
    }

    /// The local user compared the fingerprint: `true` binds the candidate
    /// (replacing any active phone), `false` refuses it.
    pub fn confirm(&mut self, accepted: bool) -> Result<Step, CryptoError> {
        let Some(candidate) = self.candidate.take() else {
            return Ok(Step::default());
        };
        match accepted {
            true => {
                let mut active = Active {
                    key: candidate.key,
                    seq_in: 0,
                    seq_out: 0,
                    recording: None,
                };
                let bound = active.seal(op::BOUND, &[])?;
                self.active = Some(active);
                Ok(Step::send(bound))
            }
            false => Ok(Step::send(reply("denied", &candidate.phone_b64))),
        }
    }

    /// The fingerprint being compared, if any.
    pub fn fingerprint(&self) -> Option<&str> {
        self.candidate.as_ref().map(|c| c.fingerprint.as_str())
    }

    /// Seals one op for the active phone (`None` when no phone is bound).
    pub fn seal_op(&mut self, op: u8) -> Result<Option<Frame>, CryptoError> {
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
        key: SessionKey,
        seq: u64,
    }

    impl Phone {
        fn scan(p: &Pairing) -> Self {
            let desktop_raw = B64URL.decode(p.link_key()).unwrap();
            let kp = KeyPair::generate().unwrap();
            let public_b64 = kp.public_b64();
            let desktop_b64 = base64::engine::general_purpose::STANDARD.encode(&desktop_raw);
            let key = kp.derive(&desktop_raw).unwrap();
            Phone {
                public_b64,
                desktop_b64,
                key,
                seq: 0,
            }
        }

        fn hello(&self) -> Frame {
            let hello = serde_json::json!({"type": "hello", "key": self.public_b64, "to": self.desktop_b64});
            Frame::Text(hello.to_string())
        }

        fn op(&mut self, code: u8, payload: &[u8]) -> Frame {
            self.seq += 1;
            Frame::Binary(self.key.seal(&seal_inner(self.seq, code, payload)).unwrap())
        }

        /// -> (seq, op) of a desktop frame, `None` if not for this phone.
        fn open(&self, frame: &Frame) -> Option<(u64, u8)> {
            let Frame::Binary(packet) = frame else {
                return None;
            };
            let plain = self.key.decrypt(packet).ok()?;
            open_inner(&plain).map(|(seq, code, _)| (seq, code))
        }
    }

    fn text(frame: &Frame) -> serde_json::Value {
        match frame {
            Frame::Text(t) => serde_json::from_str(t).unwrap(),
            Frame::Binary(_) => panic!("expected text"),
        }
    }

    fn bound() -> (Pairing, Phone) {
        let mut p = Pairing::new().unwrap();
        let phone = Phone::scan(&p);
        let step = p.on_frame(phone.hello()).unwrap();
        assert!(matches!(step.event, Some(Event::Fingerprint(_))));
        let step = p.confirm(true).unwrap();
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
        let answer = text(&step.send[0]);
        assert_eq!(answer["type"], "stale");
        assert_eq!(answer["to"], late.public_b64.as_str());
        assert_eq!(step.event, None);
    }

    #[test]
    fn refusal_sends_an_addressed_denied() {
        let mut p = Pairing::new().unwrap();
        let phone = Phone::scan(&p);
        p.on_frame(phone.hello()).unwrap();
        let step = p.confirm(false).unwrap();
        let answer = text(&step.send[0]);
        assert_eq!(answer["type"], "denied");
        assert_eq!(answer["to"], phone.public_b64.as_str());
        assert!(!p.is_bound() && !p.awaits_confirmation());
        // Nothing left to confirm
        assert_eq!(p.confirm(true).unwrap(), Step::default());
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
    fn audio_before_binding_or_from_strangers_is_ignored() {
        let mut p = Pairing::new().unwrap();
        let mut phone = Phone::scan(&p);
        p.on_frame(phone.hello()).unwrap();
        assert_eq!(
            p.on_frame(phone.op(op::START, &16000u32.to_be_bytes()))
                .unwrap(),
            Step::default()
        );
        p.confirm(true).unwrap();
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
    }

    #[test]
    fn scanning_the_new_code_replaces_the_active_phone() {
        let (mut p, _old) = bound();
        let mut new = Phone::scan(&p);
        assert!(matches!(
            p.on_frame(new.hello()).unwrap().event,
            Some(Event::Fingerprint(_))
        ));
        let step = p.confirm(true).unwrap();
        assert_eq!(new.open(&step.send[0]), Some((1, op::BOUND)));
        p.on_frame(new.op(op::START, &16000u32.to_be_bytes()))
            .unwrap();
        p.on_frame(new.op(op::AUDIO, &[2, 0])).unwrap();
        assert!(p.on_frame(new.op(op::END, &[])).unwrap().event.is_some());
    }

    #[test]
    fn seal_op_needs_a_bound_phone() {
        assert_eq!(Pairing::new().unwrap().seal_op(op::BOUND).unwrap(), None);
        let (mut p, phone) = bound();
        let frame = p.seal_op(op::BOUND).unwrap().unwrap();
        assert_eq!(phone.open(&frame), Some((2, op::BOUND)));
    }
}
