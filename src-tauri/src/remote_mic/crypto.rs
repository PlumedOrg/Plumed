//! End-to-end crypto of the ParakeetWeb remote microphone, byte-compatible with
//! `app/ui/src/lib/remote-crypto.js` (ECDH P-256, HKDF-SHA256, AES-256-GCM).
//!
//! This duplicates the JS protocol on purpose (the phone page is ParakeetWeb's).
//! The test vectors below were generated with Node `crypto` and cross-checked with
//! Python `cryptography`: if either side changes, these tests must fail.

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::agreement::{agree_ephemeral, EphemeralPrivateKey, UnparsedPublicKey, ECDH_P256};
use ring::{digest, hkdf, rand::SystemRandom};

const HKDF_SALT: &[u8] = b"ParakeetWeb-RemoteMic-v1";
const HKDF_INFO: &[u8] = b"AES-GCM-256-key";
const PUBLIC_KEY_LEN: usize = 65;
const IV_LEN: usize = 12;
const TAG_LEN: usize = 16;
/// Hex characters shown for the pairing fingerprint (64 bits, PW's floor).
const FINGERPRINT_HEX: usize = 16;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum CryptoError {
    #[error("invalid peer public key")]
    BadPublicKey,
    #[error("key agreement failed")]
    Agreement,
    #[error("audio packet failed authentication")]
    Decrypt,
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

    /// Standard padded base64 of the raw 65-byte point (88 chars), as PW sends it.
    pub fn public_b64(&self) -> String {
        B64.encode(&self.public)
    }

    /// ECDH with the peer's raw public key, then HKDF into the session key.
    pub fn derive(self, peer_raw: &[u8]) -> Result<SessionKey, CryptoError> {
        let peer = UnparsedPublicKey::new(&ECDH_P256, peer_raw);
        let key = agree_ephemeral(self.private, &peer, derive_key_bytes)
            .map_err(|_| CryptoError::Agreement)?;
        SessionKey::from_bytes(&key)
    }
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

/// HKDF-SHA256(shared secret) -> 32-byte AES key.
fn derive_key_bytes(shared: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    hkdf::Salt::new(hkdf::HKDF_SHA256, HKDF_SALT)
        .extract(shared)
        .expand(&[HKDF_INFO], &AES_256_GCM)
        .and_then(|okm| okm.fill(&mut out))
        .expect("32 bytes is a valid HKDF-SHA256 output length");
    out
}

/// The AES-256-GCM key shared with the phone (same key both directions).
pub struct SessionKey(LessSafeKey);

impl SessionKey {
    fn from_bytes(key: &[u8]) -> Result<Self, CryptoError> {
        let unbound = UnboundKey::new(&AES_256_GCM, key).map_err(|_| CryptoError::Agreement)?;
        Ok(Self(LessSafeKey::new(unbound)))
    }

    /// Decrypts `IV(12) || ciphertext || tag(16)` (no AAD).
    pub fn decrypt(&self, packet: &[u8]) -> Result<Vec<u8>, CryptoError> {
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
}

/// `SHA-256(receiver || sender)`, first 16 uppercase hex, grouped by 4: `524C-F8E8-D5A4-2D82`.
pub fn pair_fingerprint(receiver_raw: &[u8], sender_raw: &[u8]) -> String {
    let mut ctx = digest::Context::new(&digest::SHA256);
    ctx.update(receiver_raw);
    ctx.update(sender_raw);
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

    // Section 6.2 of the protocol spec (receiver scalar 0x01.., sender scalar 0x02..).
    const RECEIVER_PUB: &str =
        "BG/wO5SSQc4drdQ1GeaWDgqFtBppoFwygQOqK84VlMoWPE91OlW/AdxT9sCwx+7ni0DG/30lqW4igrmJzvccFEo=";
    const SENDER_PUB: &str =
        "BFUPRxAD89+Xw99QaseX9nIfsaH7e49vg9IkSYplyI4kE2CT1wEuUJpzcVy9CwCjzA/0tcAbP/oZarH7MnA2uOY=";
    const SHARED_X: &str = "eb459a6b30d70eb4c0ddcd34ea96ed59e8f8d1843c22dd895ab603aa13d9a81d";
    const AES_KEY: &str = "1d25d99afad8b6841cf1a308707af01be945ee32ab9b6f38de1d9236a66fb97b";
    const PACKET: &str = "0a0a0a0a0a0a0a0a0a0a0a0a7e2407b137661362dde247091ee245e64899bf475033";

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn hkdf_matches_the_js_vector() {
        assert_eq!(derive_key_bytes(&unhex(SHARED_X)).to_vec(), unhex(AES_KEY));
    }

    #[test]
    fn fingerprint_matches_the_js_vector() {
        let r = decode_public_key(RECEIVER_PUB).unwrap();
        let s = decode_public_key(SENDER_PUB).unwrap();
        assert_eq!(pair_fingerprint(&r, &s), "524C-F8E8-D5A4-2D82");
        // Order matters: receiver first
        assert_ne!(pair_fingerprint(&s, &r), "524C-F8E8-D5A4-2D82");
    }

    #[test]
    fn decrypts_the_js_packet() {
        let key = SessionKey::from_bytes(&unhex(AES_KEY)).unwrap();
        assert_eq!(key.decrypt(&unhex(PACKET)).unwrap(), unhex("0000ff7f0080"));
    }

    #[test]
    fn rejects_tampered_or_short_packets() {
        let key = SessionKey::from_bytes(&unhex(AES_KEY)).unwrap();
        let mut packet = unhex(PACKET);
        *packet.last_mut().unwrap() ^= 1;
        assert_eq!(key.decrypt(&packet), Err(CryptoError::Decrypt));
        assert_eq!(key.decrypt(&[0u8; 27]), Err(CryptoError::Decrypt));
    }

    #[test]
    fn two_generated_keys_agree() {
        let (a, b) = (KeyPair::generate().unwrap(), KeyPair::generate().unwrap());
        assert_eq!(a.public_b64().len(), 88);
        let (a_pub, b_pub) = (a.public_raw().to_vec(), b.public_raw().to_vec());
        let ka = a.derive(&b_pub).unwrap();
        let kb = b.derive(&a_pub).unwrap();
        // Seal with b's key, open with a's
        let mut buf = b"hello".to_vec();
        let iv = [7u8; IV_LEN];
        kb.0.seal_in_place_append_tag(Nonce::assume_unique_for_key(iv), Aad::empty(), &mut buf)
            .unwrap();
        let packet = [iv.as_slice(), &buf].concat();
        assert_eq!(ka.decrypt(&packet).unwrap(), b"hello");
    }

    #[test]
    fn public_key_validation_mirrors_js() {
        assert!(decode_public_key(RECEIVER_PUB).is_ok());
        assert!(decode_public_key(&"A".repeat(201)).is_err());
        assert!(decode_public_key("not valid base64!!").is_err());
        assert!(decode_public_key(&B64.encode("too short")).is_err());
        // 65 bytes but not an uncompressed point
        assert!(decode_public_key(&B64.encode([0x02u8; 65])).is_err());
        // Right shape, not on the curve: refused at agreement time
        let off_curve = [[0x04u8].as_slice(), &[0x01u8; 64]].concat();
        let kp = KeyPair::generate().unwrap();
        assert!(kp.derive(&off_curve).is_err());
    }
}
