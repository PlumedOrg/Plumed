//! Plumed fork: phone microphone through the Plumed relay
//! (`remote_microphone_server/`, HTTPS long-poll only, end-to-end encrypted).

pub mod client;
pub mod crypto;
pub mod protocol;
pub mod session;

/// End-to-end check against a running relay, with a simulated phone:
/// `cargo run -p plumed-relay` in `remote_microphone_server/`, then
/// `PLUMED_RELAY_E2E_URL=http://127.0.0.1:5174 cargo test --lib relay_e2e -- --ignored`
#[cfg(test)]
mod e2e {
    use super::client::{origin_of, phone_link, RelayClient};
    use super::crypto::KeyPair;
    use super::protocol::{Event, Pairing};
    use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
    use base64::Engine;
    use plumed_relay_protocol::{
        desktop_inbox_path, op, open_inner, phone_inbox_path, seal_inner, Frame,
    };

    const TAG: &str = "e2e-phone-inbox-tag-00";

    /// Polls `path` (as the desktop or, with the phone secret, as the phone).
    async fn next(c: &RelayClient, path: &str, secret: &str) -> Vec<Frame> {
        loop {
            let frames = c.poll_from(path, secret).await.unwrap();
            if !frames.is_empty() {
                return frames;
            }
        }
    }

    #[tokio::test]
    #[ignore = "needs a running relay and PLUMED_RELAY_E2E_URL"]
    async fn relay_e2e() {
        let url = std::env::var("PLUMED_RELAY_E2E_URL").expect("PLUMED_RELAY_E2E_URL");
        // The app installs it in `run()`
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = RelayClient::new(origin_of(&url).unwrap()).unwrap();
        let room = client.create_room().await.unwrap();
        let mut pairing = Pairing::new().unwrap();

        // The phone reads the QR link and says hello
        let link = phone_link(client.origin(), &room, &pairing.link_key());
        let desktop_raw = URL_SAFE_NO_PAD
            .decode(link.rsplit('.').next().unwrap())
            .unwrap();
        let phone_kp = KeyPair::generate().unwrap();
        let hello = serde_json::json!({
            "type": "hello",
            "key": phone_kp.public_b64(),
            "to": STANDARD.encode(&desktop_raw),
            "inbox": TAG,
        });
        let phone_key = phone_kp.derive(&desktop_raw).unwrap();
        let send_phone = |frame: Frame| {
            let (c, room) = (client.clone(), room.clone());
            let path = desktop_inbox_path(&room.id);
            async move {
                c.send_to(&path, &room.phone_secret, &[frame])
                    .await
                    .unwrap()
            }
        };
        let desktop_inbox = desktop_inbox_path(&room.id);
        let phone_inbox = phone_inbox_path(&room.id, TAG);
        send_phone(Frame::Text(hello.to_string())).await;
        let frames = next(&client, &desktop_inbox, &room.desktop_secret).await;
        let step = pairing.on_frame(frames[0].clone()).unwrap();
        let Some(Event::Fingerprint(code)) = step.event else {
            panic!("expected a fingerprint");
        };

        // The desktop user confirms: the phone gets BOUND
        let step = pairing.confirm(&code, true).unwrap();
        let bound = step.send.into_iter().next().unwrap();
        assert_eq!(bound.inbox, TAG);
        client.send(&room, TAG, &[bound.frame]).await.unwrap();
        let Frame::Binary(packet) = &next(&client, &phone_inbox, &room.phone_secret).await[0]
        else {
            panic!("expected BOUND");
        };
        let plain = phone_key.decrypt(packet).unwrap();
        assert_eq!(open_inner(&plain).map(|(_, code, _)| code), Some(op::BOUND));

        // One utterance
        let ops: [(u8, &[u8]); 3] = [
            (op::START, &16000u32.to_be_bytes()),
            (op::AUDIO, &[1, 0, 2, 0]),
            (op::END, &[]),
        ];
        for (seq, (code, payload)) in (1u64..).zip(ops) {
            let packet = phone_key.seal(&seal_inner(seq, code, payload)).unwrap();
            send_phone(Frame::Binary(packet)).await;
        }
        let mut recording = None;
        while recording.is_none() {
            for frame in next(&client, &desktop_inbox, &room.desktop_secret).await {
                recording = recording.or(pairing.on_frame(frame).unwrap().event);
            }
        }
        assert_eq!(
            recording,
            Some(Event::Recording {
                pcm: vec![1, 2],
                sample_rate: 16000
            })
        );
        client.close(&room).await;
        assert!(client.poll(&room).await.is_err());
    }
}
