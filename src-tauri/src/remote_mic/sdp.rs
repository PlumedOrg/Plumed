//! The ParakeetWeb phone page only joins a room that holds a WebRTC offer, and
//! feeds it to `setRemoteDescription`. Plumed never speaks WebRTC (it only uses
//! the HTTPS relay), so it posts a syntactically valid data-channel offer with
//! no ICE candidates: the phone's WebRTC attempt can never connect and it falls
//! back to the relay after its grace period.

use ring::rand::{SecureRandom, SystemRandom};

/// ICE ufrag/pwd alphabet (RFC 8839 ice-char, minus '+' and '/' for simplicity).
const ICE_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    SystemRandom::new()
        .fill(&mut buf)
        .expect("system RNG is available");
    buf
}

fn random_ice_string<const N: usize>() -> String {
    random_bytes::<N>()
        .iter()
        .map(|b| ICE_CHARS[*b as usize % ICE_CHARS.len()] as char)
        .collect()
}

/// A fresh dummy offer (random ufrag, pwd, DTLS fingerprint and session id).
pub fn dummy_offer() -> String {
    let session_id = u64::from_be_bytes(random_bytes::<8>()) >> 1;
    let fingerprint = random_bytes::<32>()
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(":");
    let lines = [
        "v=0".to_string(),
        format!("o=- {} 2 IN IP4 127.0.0.1", session_id),
        "s=-".to_string(),
        "t=0 0".to_string(),
        "a=group:BUNDLE 0".to_string(),
        "a=msid-semantic: WMS".to_string(),
        "m=application 9 UDP/DTLS/SCTP webrtc-datachannel".to_string(),
        "c=IN IP4 0.0.0.0".to_string(),
        format!("a=ice-ufrag:{}", random_ice_string::<8>()),
        format!("a=ice-pwd:{}", random_ice_string::<24>()),
        "a=ice-options:trickle".to_string(),
        format!("a=fingerprint:sha-256 {}", fingerprint),
        "a=setup:actpass".to_string(),
        "a=mid:0".to_string(),
        "a=sctp-port:5000".to_string(),
        "a=max-message-size:262144".to_string(),
    ];
    lines.join("\r\n") + "\r\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirrors `validateSdp` in ParakeetWeb's signaling/server.js.
    #[test]
    fn passes_the_signaling_server_validator() {
        let sdp = dummy_offer();
        assert!(sdp.len() <= 16 * 1024 && !sdp.contains('\0'));
        let body = sdp.strip_suffix("\r\n").unwrap();
        let lines: Vec<&str> = body.split("\r\n").collect();
        assert!(lines.len() <= 200);
        for line in &lines {
            assert!(line.len() <= 1024);
            let mut chars = line.chars();
            let (kind, eq) = (chars.next().unwrap(), chars.next().unwrap());
            assert!("abceikmoprstuvz".contains(kind) && eq == '=', "{line}");
            assert!(!line.contains('\n'));
        }
        assert!(
            lines[0].starts_with("v=") && lines[1].starts_with("o=") && lines[2].starts_with("s=")
        );
    }

    #[test]
    fn offers_are_random() {
        assert_ne!(dummy_offer(), dummy_offer());
    }
}
