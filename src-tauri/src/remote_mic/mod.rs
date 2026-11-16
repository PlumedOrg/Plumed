//! Plumed fork: phone microphone through the ParakeetWeb relay (HTTPS only, no WebRTC).

pub mod client;
pub mod crypto;
pub mod protocol;
pub mod sdp;
pub mod session;

/// End-to-end check against a real ParakeetWeb server, with a simulated phone:
/// `PLUMED_RELAY_E2E_URL=https://turn.plumed.fr cargo test --lib relay_e2e -- --ignored`
#[cfg(test)]
mod e2e {
    use super::client::{origin_of, Frame, RelayClient};
    use super::crypto::{decode_public_key, KeyPair};
    use super::protocol::{Event, Pairing};

    async fn next_text(c: &RelayClient, room: &super::client::Room, token: &str) -> String {
        loop {
            if let Some(Frame::Text(t)) = c.receive(room, token).await.unwrap() {
                return t;
            }
        }
    }

    #[tokio::test]
    #[ignore = "needs network and PLUMED_RELAY_E2E_URL"]
    async fn relay_e2e() {
        let url = std::env::var("PLUMED_RELAY_E2E_URL").expect("PLUMED_RELAY_E2E_URL");
        // The app installs it in `run()`
        let _ = rustls::crypto::ring::default_provider().install_default();
        let desk = RelayClient::new(origin_of(&url).unwrap()).unwrap();
        let room = desk.create_room().await.unwrap();
        desk.post_offer(&room, &super::sdp::dummy_offer())
            .await
            .unwrap();
        let desk_token = desk.claim_slot(&room).await.unwrap();
        let mut pairing = Pairing::new(KeyPair::generate().unwrap());

        // The phone joins, then our public-key reaches it
        let phone = desk.clone();
        let phone_token = phone.claim_slot(&room).await.unwrap();
        let send = |c: &RelayClient, tok: &str, t: String| {
            let (c, room, tok) = (c.clone(), room.clone(), tok.to_string());
            async move { c.send_text(&room, &tok, t).await.unwrap() }
        };
        send(&desk, &desk_token, pairing.public_key_message()).await;
        let msg: serde_json::Value =
            serde_json::from_str(&next_text(&phone, &room, &phone_token).await).unwrap();
        let desk_pub = decode_public_key(msg["key"].as_str().unwrap()).unwrap();
        let phone_kp = KeyPair::generate().unwrap();
        let phone_pub = phone_kp.public_b64();
        let phone_key = phone_kp.derive(&desk_pub).unwrap();
        let reply = serde_json::json!({"type": "sender-public-key", "key": phone_pub});
        send(&phone, &phone_token, reply.to_string()).await;
        let got = next_text(&desk, &room, &desk_token).await;
        assert!(matches!(pairing.on_text(&got), Some(Event::Fingerprint(_))));

        // Both confirm, then one recording
        pairing.confirm();
        send(&phone, &phone_token, r#"{"type":"verify-ok"}"#.into()).await;
        let got = next_text(&desk, &room, &desk_token).await;
        assert_eq!(pairing.on_text(&got), Some(Event::Bound));
        let config = r#"{"type":"audio-config","sampleRate":16000,"format":"pcm-s16"}"#;
        send(&phone, &phone_token, config.into()).await;
        assert_eq!(
            pairing.on_text(&next_text(&desk, &room, &desk_token).await),
            None
        );
        phone
            .send_binary_for_test(&room, &phone_token, phone_key.seal(&[1, 0, 2, 0]))
            .await;
        match desk.receive(&room, &desk_token).await.unwrap() {
            Some(Frame::Binary(b)) => assert_eq!(pairing.on_binary(&b), None),
            other => panic!("expected audio, got {other:?}"),
        }
        send(&phone, &phone_token, r#"{"type":"audio-end"}"#.into()).await;
        assert_eq!(
            pairing.on_text(&next_text(&desk, &room, &desk_token).await),
            Some(Event::Recording {
                pcm: vec![1, 2],
                sample_rate: 16000
            })
        );
        desk.close_slot(&room, &desk_token).await;
    }
}
