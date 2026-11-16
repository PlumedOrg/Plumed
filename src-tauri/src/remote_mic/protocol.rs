//! Receiver side of the ParakeetWeb remote-mic application protocol, without I/O.
//!
//! Sequence on one relay slot (see ParakeetWeb `useRemoteMic.js`):
//! 1. we send `public-key` (repeatedly: the relay drops frames until the phone is there),
//! 2. the phone answers `sender-public-key`, both sides show the pair fingerprint,
//! 3. both users confirm (`verify-ok`, either order), the session is bound,
//! 4. each recording is `audio-config`, encrypted pcm-s16 frames, `audio-end`.
//!
//! Nothing is accepted from the phone before both confirmations.

use super::crypto::{decode_public_key, pair_fingerprint, KeyPair, SessionKey};
use crate::smartmic::audio_bridge::accumulate_pcm;
use serde::Deserialize;

/// Same caps as ParakeetWeb's desktop receiver.
const MAX_TEXT_FRAME: usize = 4096;
const MAX_BINARY_FRAME: usize = 256 * 1024;
const SAMPLE_RATES: std::ops::RangeInclusive<u32> = 8000..=96000;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum PhoneMessage {
    SenderPublicKey {
        key: String,
    },
    VerifyOk,
    VerifyDeny,
    AudioConfig {
        #[serde(rename = "sampleRate")]
        sample_rate: u32,
        format: Option<String>,
    },
    AudioEnd,
    #[serde(other)]
    Other,
}

#[derive(Debug, PartialEq)]
pub enum Event {
    /// Show this code to the user and ask whether the phone shows the same.
    Fingerprint(String),
    /// Both sides confirmed: audio is now accepted.
    Bound,
    /// The phone user refused the pairing.
    Denied,
    /// A complete recording, mono pcm-s16 at `sample_rate`.
    Recording { pcm: Vec<i16>, sample_rate: u32 },
    /// The phone broke the protocol: drop the session.
    ProtocolError(&'static str),
}

struct Recording {
    sample_rate: u32,
    pcm: Vec<i16>,
}

pub struct Pairing {
    keypair: Option<KeyPair>,
    our_public_b64: String,
    our_public_raw: Vec<u8>,
    key: Option<SessionKey>,
    local_ok: bool,
    peer_ok: bool,
    bound: bool,
    recording: Option<Recording>,
}

impl Pairing {
    pub fn new(keypair: KeyPair) -> Self {
        Self {
            our_public_b64: keypair.public_b64(),
            our_public_raw: keypair.public_raw().to_vec(),
            keypair: Some(keypair),
            key: None,
            local_ok: false,
            peer_ok: false,
            bound: false,
            recording: None,
        }
    }

    /// The `public-key` message, to send until `has_peer_key()`.
    pub fn public_key_message(&self) -> String {
        serde_json::json!({ "type": "public-key", "key": self.our_public_b64 }).to_string()
    }

    pub fn has_peer_key(&self) -> bool {
        self.key.is_some()
    }

    pub fn is_bound(&self) -> bool {
        self.bound
    }

    /// The local user confirmed the fingerprint. The caller must send `verify-ok`.
    pub fn confirm(&mut self) -> Option<Event> {
        if self.key.is_none() || self.local_ok {
            return None;
        }
        self.local_ok = true;
        self.try_bind()
    }

    fn try_bind(&mut self) -> Option<Event> {
        match self.local_ok && self.peer_ok && !self.bound {
            true => {
                self.bound = true;
                Some(Event::Bound)
            }
            false => None,
        }
    }

    pub fn on_text(&mut self, text: &str) -> Option<Event> {
        if text.len() > MAX_TEXT_FRAME {
            return None;
        }
        let msg: PhoneMessage = serde_json::from_str(text).ok()?;
        match msg {
            PhoneMessage::SenderPublicKey { key } => self.on_peer_key(&key),
            PhoneMessage::VerifyOk if !self.bound && self.key.is_some() => {
                self.peer_ok = true;
                self.try_bind()
            }
            PhoneMessage::VerifyDeny if !self.bound => Some(Event::Denied),
            PhoneMessage::AudioConfig {
                sample_rate,
                format,
            } if self.bound => self.on_audio_config(sample_rate, format),
            PhoneMessage::AudioEnd if self.bound => self.recording.take().and_then(|r| {
                (!r.pcm.is_empty()).then_some(Event::Recording {
                    pcm: r.pcm,
                    sample_rate: r.sample_rate,
                })
            }),
            _ => None,
        }
    }

    fn on_peer_key(&mut self, b64: &str) -> Option<Event> {
        // Duplicates after the first key are ignored, like the phone does
        let keypair = self.keypair.take()?;
        let derived = decode_public_key(b64).and_then(|peer| {
            let fingerprint = pair_fingerprint(&self.our_public_raw, &peer);
            keypair.derive(&peer).map(|key| (key, fingerprint))
        });
        match derived {
            Ok((key, fingerprint)) => {
                self.key = Some(key);
                Some(Event::Fingerprint(fingerprint))
            }
            Err(_) => Some(Event::ProtocolError("invalid phone public key")),
        }
    }

    fn on_audio_config(&mut self, sample_rate: u32, format: Option<String>) -> Option<Event> {
        if self.recording.is_some() {
            return Some(Event::ProtocolError("audio-config inside a recording"));
        }
        if !SAMPLE_RATES.contains(&sample_rate) || format.as_deref() != Some("pcm-s16") {
            return Some(Event::ProtocolError("unsupported audio format"));
        }
        self.recording = Some(Recording {
            sample_rate,
            pcm: Vec::new(),
        });
        None
    }

    /// Encrypted audio frame. Dropped unless bound and inside a recording.
    pub fn on_binary(&mut self, packet: &[u8]) -> Option<Event> {
        let (Some(key), Some(rec)) = (&self.key, &mut self.recording) else {
            return None;
        };
        if !self.bound || packet.len() > MAX_BINARY_FRAME {
            return None;
        }
        match key.decrypt(packet) {
            // A full buffer stops growing: the recording is cut at the cap
            Ok(pcm) => {
                accumulate_pcm(&mut rec.pcm, &pcm);
                None
            }
            Err(_) => Some(Event::ProtocolError("audio frame failed authentication")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// A fake phone: its own key pair and the derived session key.
    struct Phone {
        public_b64: String,
        key: SessionKey,
    }

    impl Phone {
        /// Pairs with the receiver's `public-key` message, returns the phone and its reply.
        fn join(receiver_msg: &str) -> (Self, String) {
            let v: serde_json::Value = serde_json::from_str(receiver_msg).unwrap();
            let peer = decode_public_key(v["key"].as_str().unwrap()).unwrap();
            let kp = KeyPair::generate().unwrap();
            let public_b64 = kp.public_b64();
            let key = kp.derive(&peer).unwrap();
            let reply = serde_json::json!({"type": "sender-public-key", "key": public_b64});
            (Phone { public_b64, key }, reply.to_string())
        }

        fn seal(&self, plain: &[u8]) -> Vec<u8> {
            self.key.seal(plain)
        }
    }

    fn paired() -> (Pairing, Phone) {
        let mut p = Pairing::new(KeyPair::generate().unwrap());
        let (phone, reply) = Phone::join(&p.public_key_message());
        assert!(matches!(p.on_text(&reply), Some(Event::Fingerprint(_))));
        (p, phone)
    }

    fn bound() -> (Pairing, Phone) {
        let (mut p, phone) = paired();
        assert_eq!(p.confirm(), None);
        assert_eq!(p.on_text(r#"{"type":"verify-ok"}"#), Some(Event::Bound));
        (p, phone)
    }

    const CONFIG: &str = r#"{"type":"audio-config","sampleRate":16000,"format":"pcm-s16"}"#;

    #[test]
    fn fingerprint_matches_what_the_phone_computes() {
        let mut p = Pairing::new(KeyPair::generate().unwrap());
        let receiver_raw = p.our_public_raw.clone();
        let (phone, reply) = Phone::join(&p.public_key_message());
        let sender_raw = decode_public_key(&phone.public_b64).unwrap();
        assert_eq!(
            p.on_text(&reply),
            Some(Event::Fingerprint(pair_fingerprint(
                &receiver_raw,
                &sender_raw
            )))
        );
    }

    #[test]
    fn binds_in_either_confirmation_order() {
        let (mut p, _) = paired();
        assert_eq!(p.on_text(r#"{"type":"verify-ok"}"#), None);
        assert_eq!(p.confirm(), Some(Event::Bound));
        assert!(p.is_bound());
        bound();
    }

    #[test]
    fn duplicate_peer_keys_are_ignored() {
        let (mut p, _) = paired();
        let (_, other) = Phone::join(&p.public_key_message());
        assert_eq!(p.on_text(&other), None);
    }

    #[test]
    fn verify_before_any_key_does_not_count() {
        let mut p = Pairing::new(KeyPair::generate().unwrap());
        assert_eq!(p.on_text(r#"{"type":"verify-ok"}"#), None);
        assert_eq!(p.confirm(), None);
        let (_, reply) = Phone::join(&p.public_key_message());
        p.on_text(&reply);
        // The early verify-ok was not buffered: the phone still has to confirm
        assert_eq!(p.confirm(), None);
        assert!(!p.is_bound());
    }

    #[test]
    fn audio_is_ignored_until_bound() {
        let (mut p, phone) = paired();
        assert_eq!(p.on_text(CONFIG), None);
        assert_eq!(p.on_binary(&phone.seal(&[1, 0])), None);
        assert_eq!(p.on_text(r#"{"type":"audio-end"}"#), None);
    }

    #[test]
    fn a_recording_is_decrypted_and_returned_on_audio_end() {
        let (mut p, phone) = bound();
        assert_eq!(p.on_text(CONFIG), None);
        p.on_binary(&phone.seal(&[0x00, 0x00, 0xff, 0x7f]));
        p.on_binary(&phone.seal(&[0x00, 0x80]));
        assert_eq!(
            p.on_text(r#"{"type":"audio-end"}"#),
            Some(Event::Recording {
                pcm: vec![0, 32767, -32768],
                sample_rate: 16000
            })
        );
        // Next recording starts clean
        assert_eq!(p.on_text(CONFIG), None);
        assert_eq!(p.on_text(r#"{"type":"audio-end"}"#), None);
    }

    #[test]
    fn protocol_violations_are_reported() {
        let (mut p, _) = bound();
        p.on_text(CONFIG);
        assert!(matches!(p.on_text(CONFIG), Some(Event::ProtocolError(_))));
        let (mut p, _) = bound();
        let bad = r#"{"type":"audio-config","sampleRate":200000,"format":"pcm-s16"}"#;
        assert!(matches!(p.on_text(bad), Some(Event::ProtocolError(_))));
        let f32 = r#"{"type":"audio-config","sampleRate":16000}"#;
        assert!(matches!(p.on_text(f32), Some(Event::ProtocolError(_))));
        let (mut p, _) = bound();
        p.on_text(CONFIG);
        let mut forged = vec![0u8; 40];
        forged[0] = 1;
        assert!(matches!(
            p.on_binary(&forged),
            Some(Event::ProtocolError(_))
        ));
    }

    #[test]
    fn deny_and_garbage() {
        let (mut p, _) = paired();
        assert_eq!(p.on_text(r#"{"type":"verify-deny"}"#), Some(Event::Denied));
        assert_eq!(p.on_text("not json"), None);
        assert_eq!(p.on_text(&"x".repeat(5000)), None);
        assert_eq!(p.on_text(r#"{"type":"paused"}"#), None);
        let mut fresh = Pairing::new(KeyPair::generate().unwrap());
        let bad_key = r#"{"type":"sender-public-key","key":"AAAA"}"#;
        assert!(matches!(
            fresh.on_text(bad_key),
            Some(Event::ProtocolError(_))
        ));
    }
}
