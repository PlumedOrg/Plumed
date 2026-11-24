//! Wire format of the Plumed remote microphone, shared by the relay server
//! (`PhoneMic_server/server`) and the Plumed desktop app (`src-tauri`).
//! The phone page (`server/static/wire.js`) mirrors it in JavaScript; both sides
//! are pinned by `test-vectors.json`.
//!
//! Keep `PhoneMic_server/protocol_details.md` (the human-readable spec) in sync
//! with any change here.
//!
//! # Version
//! [`PROTOCOL_VERSION`] is served at [`VERSION_PATH`] and mixed into the HKDF
//! salt ([`HKDF_SALT`]). The desktop refuses a relay with any other version:
//! no backward compatibility, on purpose, to keep the attack surface small.
//! Bump it (and regenerate the crypto test vectors) on any wire change.
//!
//! # Transport (what the relay sees)
//! A room has one secret per [`Role`] (header [`SECRET_HEADER`]) and one inbox
//! for the desktop plus one per phone. The QR code only carries the phone
//! secret, so whoever sees it cannot read the desktop inbox nor close the room.
//! - `POST /api/rooms` creates a room: `{"room": <id>, "desktop": <secret>, "phone": <secret>}`,
//! - `GET <inbox>` returns everything queued in it (one envelope), waiting up
//!   to [`POLL_WAIT_SECS`], `204` when nothing came; needs the owner's secret,
//! - `POST <inbox>` appends an [envelope](encode); needs the *other* role's secret,
//! - `DELETE /api/rooms/<id>` closes the room (desktop secret).
//!
//! Inboxes: [`desktop_inbox_path`], and [`phone_inbox_path`] where the tag is
//! chosen at random by each phone ([`valid_inbox_tag`]), so a phone never
//! takes frames meant for another one.
//!
//! # Application messages (what the two ends exchange)
//! - phone -> desktop, plain text: `{"type":"hello","key":<phone pub>,"to":<desktop pub>,"inbox":<tag>}`,
//! - desktop -> phone, plain text: `{"type":"stale","to":<phone pub>}` (the
//!   desktop has no key for that phone: only desktop-secret holders can post it),
//! - everything else is a binary frame `IV(12) || AES-GCM(inner) || tag(16)`,
//!   where `inner` is [`seal_inner`]: a sequence number then an [`op`].

use std::fmt;

/// The speech model repositories and the model relay's URL scheme.
pub mod page;
pub mod models;
pub mod releases;

macro_rules! protocol_version {
    () => {
        "5.0.0"
    };
}
/// Version of the whole PhoneMic protocol (wire format, crypto, pairing).
/// Any mismatch between the desktop and the relay is refused.
pub const PROTOCOL_VERSION: &str = protocol_version!();
/// HKDF salt of the end-to-end keys: a peer on another version can never decrypt.
pub const HKDF_SALT: &str = concat!("Plumed-RemoteMic-", protocol_version!());
/// `GET` answers `{"protocol": PROTOCOL_VERSION}`.
pub const VERSION_PATH: &str = "/api/version";

/// Header carrying the role's secret on every room request.
pub const SECRET_HEADER: &str = "x-room-secret";
/// Path prefix of the rooms API.
pub const ROOMS_PATH: &str = "/api/rooms";
/// The server answers a long-poll after this long without traffic.
pub const POLL_WAIT_SECS: u64 = 25;
/// Largest single frame (an encrypted audio chunk is about 16 KiB).
pub const MAX_FRAME: usize = 256 * 1024;
/// Largest request body (one envelope).
pub const MAX_BODY: usize = 1024 * 1024;
/// Bytes a mailbox may hold before senders get `429`.
pub const MAILBOX_CAP: usize = MAX_BODY;
/// Phone inboxes a room may hold at once (phones that scanned the QR code).
pub const MAX_PHONE_INBOXES: usize = 8;
/// Sample rates accepted in [`op::START`].
pub const SAMPLE_RATES: std::ops::RangeInclusive<u32> = 8000..=96000;

/// The two ends of a room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Desktop,
    Phone,
}

impl Role {
    pub const fn as_str(self) -> &'static str {
        match self {
            Role::Desktop => "desktop",
            Role::Phone => "phone",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "desktop" => Some(Role::Desktop),
            "phone" => Some(Role::Phone),
            _ => None,
        }
    }

    pub const fn peer(self) -> Self {
        match self {
            Role::Desktop => Role::Phone,
            Role::Phone => Role::Desktop,
        }
    }
}

/// `/api/rooms/<id>/desktop`
pub fn desktop_inbox_path(room: &str) -> String {
    format!("{}/{}/desktop", ROOMS_PATH, room)
}

/// `/api/rooms/<id>/phone/<tag>`
pub fn phone_inbox_path(room: &str, tag: &str) -> String {
    format!("{}/{}/phone/{}", ROOMS_PATH, room, tag)
}

/// A phone inbox tag: 16 to 43 base64url characters (the phone uses 16
/// random bytes, 22 characters).
pub fn valid_inbox_tag(tag: &str) -> bool {
    (16..=43).contains(&tag.len())
        && tag
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Frame {
    /// UTF-8 JSON (only the unauthenticated handshake messages).
    Text(String),
    /// An encrypted packet.
    Binary(Vec<u8>),
}

const KIND_TEXT: u8 = 0;
const KIND_BINARY: u8 = 1;
const HEADER_LEN: usize = 5;

#[derive(Debug, PartialEq, Eq)]
pub enum EnvelopeError {
    Truncated,
    UnknownKind(u8),
    FrameTooLarge(usize),
    NotUtf8,
}

impl fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvelopeError::Truncated => write!(f, "truncated envelope"),
            EnvelopeError::UnknownKind(k) => write!(f, "unknown frame kind {k}"),
            EnvelopeError::FrameTooLarge(n) => write!(f, "frame of {n} bytes is too large"),
            EnvelopeError::NotUtf8 => write!(f, "text frame is not UTF-8"),
        }
    }
}

impl std::error::Error for EnvelopeError {}

/// Envelope: frames back to back, each `kind(u8) || len(u32 BE) || bytes`.
/// Two envelopes concatenated are one valid envelope, so the relay can queue
/// request bodies as they come.
pub fn encode(frames: &[Frame]) -> Vec<u8> {
    let mut out = Vec::new();
    for frame in frames {
        let (kind, bytes) = match frame {
            Frame::Text(t) => (KIND_TEXT, t.as_bytes()),
            Frame::Binary(b) => (KIND_BINARY, b.as_slice()),
        };
        out.push(kind);
        out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        out.extend_from_slice(bytes);
    }
    out
}

/// Walks an envelope without copying: `(is_text, bytes)` per frame.
fn walk(mut bytes: &[u8]) -> impl Iterator<Item = Result<(bool, &[u8]), EnvelopeError>> {
    std::iter::from_fn(move || {
        if bytes.is_empty() {
            return None;
        }
        let item = parse_one(bytes);
        match item {
            Ok((is_text, body, rest)) => {
                bytes = rest;
                Some(Ok((is_text, body)))
            }
            Err(e) => {
                bytes = &[];
                Some(Err(e))
            }
        }
    })
}

fn parse_one(bytes: &[u8]) -> Result<(bool, &[u8], &[u8]), EnvelopeError> {
    if bytes.len() < HEADER_LEN {
        return Err(EnvelopeError::Truncated);
    }
    let is_text = match bytes[0] {
        KIND_TEXT => true,
        KIND_BINARY => false,
        other => return Err(EnvelopeError::UnknownKind(other)),
    };
    let len = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;
    if len > MAX_FRAME {
        return Err(EnvelopeError::FrameTooLarge(len));
    }
    let rest = &bytes[HEADER_LEN..];
    match rest.len() >= len {
        true => Ok((is_text, &rest[..len], &rest[len..])),
        false => Err(EnvelopeError::Truncated),
    }
}

/// Checks an envelope (what the relay does before queueing it).
pub fn validate(bytes: &[u8]) -> Result<(), EnvelopeError> {
    for item in walk(bytes) {
        let (is_text, body) = item?;
        if is_text && std::str::from_utf8(body).is_err() {
            return Err(EnvelopeError::NotUtf8);
        }
    }
    Ok(())
}

pub fn decode(bytes: &[u8]) -> Result<Vec<Frame>, EnvelopeError> {
    walk(bytes)
        .map(|item| {
            let (is_text, body) = item?;
            match is_text {
                true => std::str::from_utf8(body)
                    .map(|s| Frame::Text(s.to_string()))
                    .map_err(|_| EnvelopeError::NotUtf8),
                false => Ok(Frame::Binary(body.to_vec())),
            }
        })
        .collect()
}

/// Operations inside an encrypted frame.
pub mod op {
    /// phone -> desktop: a recording starts, payload `sample_rate(u32 BE)`.
    pub const START: u8 = 1;
    /// phone -> desktop: mono pcm-s16le samples of the current recording.
    pub const AUDIO: u8 = 2;
    /// phone -> desktop: the recording is complete, transcribe it.
    pub const END: u8 = 3;
    /// phone -> desktop: drop the current recording.
    pub const CANCEL: u8 = 4;
    /// desktop -> phone: the user accepted the phone, pairing bound. Payload:
    /// `<room id>.<phone secret>` (ASCII) of the private room the phone moves to.
    pub const BOUND: u8 = 0x10;
    /// desktop -> phone: a recording was received and is being transcribed.
    pub const RECEIVED: u8 = 0x11;
    /// desktop -> phone: the user refused this pairing (or let it expire).
    pub const DENIED: u8 = 0x12;
    /// desktop -> phone: the user paired another phone instead of this one.
    pub const REPLACED: u8 = 0x13;
    /// desktop -> phone: payload `seq(u64 BE)`, the last phone packet received
    /// with no gap before it. The phone forgets the packets up to it and
    /// resends the others (a lower value than what it sent means a gap).
    pub const ACK: u8 = 0x14;
}

const SEQ_LEN: usize = 8;

/// Plaintext of an encrypted frame: `seq(u64 BE) || op(u8) || payload`.
/// Each side numbers its frames from 1 and the receiver refuses any `seq` not
/// above the last one, so the relay cannot replay or reorder a recording.
pub fn seal_inner(seq: u64, op: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(SEQ_LEN + 1 + payload.len());
    out.extend_from_slice(&seq.to_be_bytes());
    out.push(op);
    out.extend_from_slice(payload);
    out
}

/// `(seq, op, payload)`, `None` when too short.
pub fn open_inner(plain: &[u8]) -> Option<(u64, u8, &[u8])> {
    if plain.len() < SEQ_LEN + 1 {
        return None;
    }
    let (seq, rest) = plain.split_at(SEQ_LEN);
    let seq = u64::from_be_bytes(seq.try_into().ok()?);
    Some((seq, rest[0], &rest[1..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    fn vectors() -> serde_json::Value {
        serde_json::from_str(include_str!("../test-vectors.json")).unwrap()
    }

    #[test]
    fn envelope_matches_the_shared_vector() {
        let v = &vectors()["envelope"];
        let frames = vec![
            Frame::Text(v["text"].as_str().unwrap().to_string()),
            Frame::Binary(unhex(v["binary"].as_str().unwrap())),
        ];
        let bytes = unhex(v["encoded"].as_str().unwrap());
        assert_eq!(encode(&frames), bytes);
        assert_eq!(decode(&bytes).unwrap(), frames);
    }

    #[test]
    fn inner_matches_the_shared_vector() {
        let v = &vectors()["inner"];
        let payload = unhex(v["payload"].as_str().unwrap());
        let seq = v["seq"].as_u64().unwrap();
        let op = v["op"].as_u64().unwrap() as u8;
        let bytes = unhex(v["encoded"].as_str().unwrap());
        assert_eq!(seal_inner(seq, op, &payload), bytes);
        assert_eq!(open_inner(&bytes), Some((seq, op, payload.as_slice())));
        assert_eq!(open_inner(&bytes[..8]), None);
    }

    #[test]
    fn envelopes_concatenate() {
        let a = encode(&[Frame::Text("a".into())]);
        let b = encode(&[Frame::Binary(vec![1])]);
        let both = [a, b].concat();
        assert_eq!(
            decode(&both).unwrap(),
            vec![Frame::Text("a".into()), Frame::Binary(vec![1])]
        );
        assert_eq!(validate(&[]), Ok(()));
    }

    #[test]
    fn malformed_envelopes_are_refused() {
        assert_eq!(validate(&[1, 0, 0]), Err(EnvelopeError::Truncated));
        assert_eq!(validate(&[1, 0, 0, 0, 2, 9]), Err(EnvelopeError::Truncated));
        assert_eq!(
            validate(&[7, 0, 0, 0, 0]),
            Err(EnvelopeError::UnknownKind(7))
        );
        let huge = (MAX_FRAME as u32 + 1).to_be_bytes();
        assert_eq!(
            validate(&[1, huge[0], huge[1], huge[2], huge[3]]),
            Err(EnvelopeError::FrameTooLarge(MAX_FRAME + 1))
        );
        assert_eq!(
            validate(&[0, 0, 0, 0, 1, 0xff]),
            Err(EnvelopeError::NotUtf8)
        );
    }

    #[test]
    fn version_matches_the_shared_vector() {
        let v = vectors();
        assert_eq!(v["protocol_version"].as_str().unwrap(), PROTOCOL_VERSION);
        assert_eq!(HKDF_SALT, format!("Plumed-RemoteMic-{PROTOCOL_VERSION}"));
    }

    #[test]
    fn roles() {
        assert_eq!(Role::parse("phone"), Some(Role::Phone));
        assert_eq!(Role::parse("x"), None);
        assert_eq!(Role::Phone.peer(), Role::Desktop);
        assert_eq!(desktop_inbox_path("abc"), "/api/rooms/abc/desktop");
        assert_eq!(phone_inbox_path("abc", "t"), "/api/rooms/abc/phone/t");
    }

    #[test]
    fn inbox_tags() {
        assert!(valid_inbox_tag("AAAAAAAAAAAAAAAAAAAAAA"));
        assert!(valid_inbox_tag("abc-_9abc-_9abc-"));
        assert!(!valid_inbox_tag("short"));
        assert!(!valid_inbox_tag(&"a".repeat(44)));
        assert!(!valid_inbox_tag("AAAAAAAAAAAAAAAAAAAAA/"));
    }
}
