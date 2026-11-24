//! End-to-end crypto of the Plumed remote microphone (ECDH P-256, HKDF-SHA256,
//! AES-256-GCM), byte-compatible with the phone page
//! (`PhoneMic_server/server/static/crypto.js`).
//! Keep `PhoneMic_server/protocol_details.md` in sync with any change here.
//!
//! Each direction has its own key, bound to both public keys and to the QR
//! code's pairing secret:
//! `HKDF(salt, shared x || pairing secret, info = label || desktop pub || phone pub)`
//! with the labels `phone-to-desktop` and `desktop-to-phone`. A packet can then
//! never be reflected back to its sender, and the key only exists for that
//! exact pair. The label `hello-proof` gives the value the phone's `hello`
//! carries: only a phone that read the QR code can compute it, so the relay
//! (which never sees the URL fragment) cannot pose as a phone.
//!
//! The JS side duplicates this on purpose (a browser cannot run Rust). Both are
//! pinned by `PhoneMic_server/protocol/test-vectors.json`, generated with
//! Node `crypto`: if either side changes, these tests must fail.

use base64::engine::general_purpose::{STANDARD as B64, URL_SAFE_NO_PAD as B64URL};
use base64::Engine;
use plumed_relay_protocol::{Role, HKDF_SALT};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::agreement::{agree_ephemeral, EphemeralPrivateKey, UnparsedPublicKey, ECDH_P256};
use ring::rand::{SecureRandom, SystemRandom};
use ring::{digest, hkdf};

const PHONE_TO_DESKTOP: &[u8] = b"phone-to-desktop";
const DESKTOP_TO_PHONE: &[u8] = b"desktop-to-phone";
const HELLO_PROOF: &[u8] = b"hello-proof";
/// Random bytes of the pairing secret carried by the QR code.
pub const PAIRING_SECRET_LEN: usize = 32;
const PUBLIC_KEY_LEN: usize = 65;
const IV_LEN: usize = 12;
const TAG_LEN: usize = 16;
/// Hex characters shown for the pairing fingerprint (64 bits).
const FINGERPRINT_HEX: usize = 16;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum CryptoError {
    #[error("invalid peer public key")]
    BadPublicKey,
    #[error("key agreement failed")]
    Agreement,
    #[error("audio packet failed authentication")]
    Decrypt,
    #[error("encryption failed")]
    Seal,
}

/// Our ephemeral ECDH key for one pairing. Consumed by `derive`.
pub struct KeyPair {
    private: EphemeralPrivateKey,
    public: Vec<u8>,
}

impl KeyPair {
    pub fn generate() -> Result<Self, CryptoError> {
        let private = EphemeralPrivateKey::generate(&ECDH_P256, &SystemRandom::new())
            .map_err(|_| CryptoError::Agreement)?;
        let public = private
            .compute_public_key()
            .map_err(|_| CryptoError::Agreement)?
            .as_ref()
            .to_vec();
        Ok(Self { private, public })
    }

    pub fn public_raw(&self) -> &[u8] {
        &self.public
    }

    /// Standard padded base64 of the raw 65-byte point (88 chars), as in a `hello`.
    pub fn public_b64(&self) -> String {
        B64.encode(&self.public)
    }

    /// Unpadded base64url of the raw point, for the QR link fragment.
    pub fn public_b64url(&self) -> String {
        B64URL.encode(&self.public)
    }

    /// ECDH with the peer's raw public key, then HKDF (with the QR code's
    /// `pairing_secret`) into the two keys of the session, seen from `us`, and
    /// the hello proof.
    pub fn derive(
        self,
        peer_raw: &[u8],
        pairing_secret: &[u8],
        us: Role,
    ) -> Result<(SessionKeys, HelloProof), CryptoError> {
        let (desktop, phone) = match us {
            Role::Desktop => (self.public.as_slice(), peer_raw),
            Role::Phone => (peer_raw, self.public.as_slice()),
        };
        let peer = UnparsedPublicKey::new(&ECDH_P256, peer_raw);
        let [to_desktop, to_phone, proof] = agree_ephemeral(self.private, &peer, |shared| {
            [PHONE_TO_DESKTOP, DESKTOP_TO_PHONE, HELLO_PROOF]
                .map(|label| derive_key_bytes(shared, pairing_secret, label, desktop, phone))
        })
        .map_err(|_| CryptoError::Agreement)?;
        let (send, recv) = match us {
            Role::Desktop => (to_phone, to_desktop),
            Role::Phone => (to_desktop, to_phone),
        };
        let keys = SessionKeys {
            send: SessionKey::from_bytes(&send)?,
            recv: SessionKey::from_bytes(&recv)?,
        };
        Ok((keys, HelloProof(proof)))
    }
}

/// What a `hello` must carry (base64url in its `proof` field).
pub struct HelloProof([u8; 32]);

impl HelloProof {
    pub fn b64url(&self) -> String {
        B64URL.encode(self.0)
    }

    /// Constant-time comparison with the `proof` of a hello.
    pub fn matches(&self, claimed_b64url: &str) -> bool {
        match B64URL.decode(claimed_b64url) {
            Ok(claimed) if claimed.len() == self.0.len() => {
                claimed
                    .iter()
                    .zip(self.0)
                    .fold(0u8, |acc, (a, b)| acc | (a ^ b))
                    == 0
            }
            _ => false,
        }
    }
}

/// A fresh random pairing secret for a QR code.
pub fn pairing_secret() -> Result<[u8; PAIRING_SECRET_LEN], CryptoError> {
    let mut secret = [0u8; PAIRING_SECRET_LEN];
    SystemRandom::new()
        .fill(&mut secret)
        .map_err(|_| CryptoError::Agreement)?;
    Ok(secret)
}

/// Mirrors `importPublicKey`: length cap, base64 charset, 65 bytes, 0x04 prefix.
/// Curve membership is checked later by ring during the agreement.
pub fn decode_public_key(b64: &str) -> Result<Vec<u8>, CryptoError> {
    let charset_ok = b64.len() <= 200
        && !b64.is_empty()
        && b64.trim_end_matches('=').len() + 2 >= b64.len()
        && b64
            .trim_end_matches('=')
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'+' || c == b'/');
    if !charset_ok {
        return Err(CryptoError::BadPublicKey);
    }
    let raw = B64.decode(b64).map_err(|_| CryptoError::BadPublicKey)?;
    match raw.len() == PUBLIC_KEY_LEN && raw[0] == 0x04 {
        true => Ok(raw),
        false => Err(CryptoError::BadPublicKey),
    }
}

/// HKDF-SHA256(shared x || pairing secret, info = label || desktop || phone) -> 32 bytes.
fn derive_key_bytes(
    shared: &[u8],
    pairing_secret: &[u8],
    label: &[u8],
    desktop: &[u8],
    phone: &[u8],
) -> [u8; 32] {
    let mut out = [0u8; 32];
    hkdf::Salt::new(hkdf::HKDF_SHA256, HKDF_SALT.as_bytes())
        .extract(&[shared, pairing_secret].concat())
        .expand(&[label, desktop, phone], &AES_256_GCM)
        .and_then(|okm| okm.fill(&mut out))
        .expect("32 bytes is a valid HKDF-SHA256 output length");
    out
}

/// The two AES-256-GCM keys of a session: one to seal, one to open.
pub struct SessionKeys {
    send: SessionKey,
    recv: SessionKey,
}

impl SessionKeys {
    /// `IV(12) || ciphertext || tag(16)` for the peer, with a random IV.
    pub fn seal(&self, plain: &[u8]) -> Result<Vec<u8>, CryptoError> {
        self.send.seal(plain)
    }

    /// Opens a packet from the peer.
    pub fn decrypt(&self, packet: &[u8]) -> Result<Vec<u8>, CryptoError> {
        self.recv.decrypt(packet)
    }
}

/// One AES-256-GCM key.
struct SessionKey(LessSafeKey);

impl SessionKey {
    fn from_bytes(key: &[u8]) -> Result<Self, CryptoError> {
        let unbound = UnboundKey::new(&AES_256_GCM, key).map_err(|_| CryptoError::Agreement)?;
        Ok(Self(LessSafeKey::new(unbound)))
    }

    /// Decrypts `IV(12) || ciphertext || tag(16)` (no AAD).
    fn decrypt(&self, packet: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if packet.len() < IV_LEN + TAG_LEN {
            return Err(CryptoError::Decrypt);
        }
        let (iv, sealed) = packet.split_at(IV_LEN);
        let nonce = Nonce::try_assume_unique_for_key(iv).map_err(|_| CryptoError::Decrypt)?;
        let mut buf = sealed.to_vec();
        let plain_len = self
            .0
            .open_in_place(nonce, Aad::empty(), &mut buf)
            .map_err(|_| CryptoError::Decrypt)?
            .len();
        buf.truncate(plain_len);
        Ok(buf)
    }

    /// `IV(12) || ciphertext || tag(16)` with a random IV.
    fn seal(&self, plain: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut iv = [0u8; IV_LEN];
        SystemRandom::new()
            .fill(&mut iv)
            .map_err(|_| CryptoError::Seal)?;
        let mut buf = plain.to_vec();
        self.0
            .seal_in_place_append_tag(Nonce::assume_unique_for_key(iv), Aad::empty(), &mut buf)
            .map_err(|_| CryptoError::Seal)?;
        Ok([iv.as_slice(), &buf].concat())
    }
}

/// `SHA-256(desktop || phone)`, first 16 uppercase hex, grouped by 4: `524C-F8E8-D5A4-2D82`.
pub fn pair_fingerprint(desktop_raw: &[u8], phone_raw: &[u8]) -> String {
    let mut ctx = digest::Context::new(&digest::SHA256);
    ctx.update(desktop_raw);
    ctx.update(phone_raw);
    let hex: String = ctx
        .finish()
        .as_ref()
        .iter()
        .take(FINGERPRINT_HEX / 2)
        .map(|b| format!("{:02X}", b))
        .collect();
    hex.as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).expect("hex is ascii"))
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shared with the relay crate and the phone page tests.
    fn vector(name: &str) -> String {
        let all: serde_json::Value = serde_json::from_str(include_str!(
            "../../../PhoneMic_server/protocol/test-vectors.json"
        ))
        .unwrap();
        all["crypto"][name].as_str().unwrap().to_owned()
    }

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn hkdf_matches_the_js_vector() {
        let d = decode_public_key(&vector("desktop_public")).unwrap();
        let p = decode_public_key(&vector("phone_public")).unwrap();
        let shared = unhex(&vector("shared_x"));
        let secret = unhex(&vector("pairing_secret"));
        for (label, name) in [
            (PHONE_TO_DESKTOP, "key_phone_to_desktop"),
            (DESKTOP_TO_PHONE, "key_desktop_to_phone"),
            (HELLO_PROOF, "hello_proof"),
        ] {
            assert_eq!(
                derive_key_bytes(&shared, &secret, label, &d, &p).to_vec(),
                unhex(&vector(name)),
                "{name}"
            );
        }
    }

    #[test]
    fn fingerprint_matches_the_js_vector() {
        let d = decode_public_key(&vector("desktop_public")).unwrap();
        let p = decode_public_key(&vector("phone_public")).unwrap();
        assert_eq!(pair_fingerprint(&d, &p), vector("fingerprint"));
        // Order matters: desktop first
        assert_ne!(pair_fingerprint(&p, &d), vector("fingerprint"));
    }

    #[test]
    fn decrypts_the_js_packet() {
        let key = SessionKey::from_bytes(&unhex(&vector("key_phone_to_desktop"))).unwrap();
        assert_eq!(
            key.decrypt(&unhex(&vector("packet"))).unwrap(),
            unhex(&vector("packet_inner"))
        );
    }

    #[test]
    fn rejects_tampered_or_short_packets() {
        let key = SessionKey::from_bytes(&unhex(&vector("key_phone_to_desktop"))).unwrap();
        let mut packet = unhex(&vector("packet"));
        *packet.last_mut().unwrap() ^= 1;
        assert_eq!(key.decrypt(&packet), Err(CryptoError::Decrypt));
        assert_eq!(key.decrypt(&[0u8; 27]), Err(CryptoError::Decrypt));
    }

    #[test]
    fn two_generated_keys_agree_and_ivs_differ() {
        let (a, b) = (KeyPair::generate().unwrap(), KeyPair::generate().unwrap());
        assert_eq!(a.public_b64().len(), 88);
        assert_eq!(B64URL.decode(a.public_b64url()).unwrap(), a.public_raw());
        let (a_pub, b_pub) = (a.public_raw().to_vec(), b.public_raw().to_vec());
        let secret = pairing_secret().unwrap();
        let (desktop, desktop_proof) = a.derive(&b_pub, &secret, Role::Desktop).unwrap();
        let (phone, phone_proof) = b.derive(&a_pub, &secret, Role::Phone).unwrap();
        assert!(desktop_proof.matches(&phone_proof.b64url()));
        assert!(!desktop_proof.matches("short"));
        assert!(!desktop_proof.matches(&B64URL.encode([0u8; 32])));
        let (p1, p2) = (phone.seal(b"hello").unwrap(), phone.seal(b"hello").unwrap());
        assert_ne!(p1[..IV_LEN], p2[..IV_LEN]);
        assert_eq!(desktop.decrypt(&p1).unwrap(), b"hello");
        let back = desktop.seal(b"bound").unwrap();
        assert_eq!(phone.decrypt(&back).unwrap(), b"bound");
        // Reflected to its sender, a packet does not open
        assert_eq!(phone.decrypt(&p1), Err(CryptoError::Decrypt));
        assert_eq!(desktop.decrypt(&back), Err(CryptoError::Decrypt));
    }

    #[test]
    fn another_pairing_secret_gives_other_keys() {
        let (a, b) = (KeyPair::generate().unwrap(), KeyPair::generate().unwrap());
        let (a_pub, b_pub) = (a.public_raw().to_vec(), b.public_raw().to_vec());
        let (desktop, desktop_proof) = a.derive(&b_pub, &[1; 32], Role::Desktop).unwrap();
        let (phone, phone_proof) = b.derive(&a_pub, &[2; 32], Role::Phone).unwrap();
        assert!(!desktop_proof.matches(&phone_proof.b64url()));
        let packet = phone.seal(b"hello").unwrap();
        assert_eq!(desktop.decrypt(&packet), Err(CryptoError::Decrypt));
    }

    #[test]
    fn public_key_validation_mirrors_js() {
        assert!(decode_public_key(&vector("desktop_public")).is_ok());
        assert!(decode_public_key(&"A".repeat(201)).is_err());
        assert!(decode_public_key("not valid base64!!").is_err());
        assert!(decode_public_key(&B64.encode("too short")).is_err());
        // 65 bytes but not an uncompressed point
        assert!(decode_public_key(&B64.encode([0x02u8; 65])).is_err());
        // Right shape, not on the curve: refused at agreement time
        let off_curve = [[0x04u8].as_slice(), &[0x01u8; 64]].concat();
        let kp = KeyPair::generate().unwrap();
        assert!(kp.derive(&off_curve, &[0; 32], Role::Desktop).is_err());
    }
}
