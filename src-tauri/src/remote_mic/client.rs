//! HTTPS client for the ParakeetWeb signaling server, relay long-poll only.
//! Long-poll goes through `reqwest`, which honors the system proxy: hospital
//! networks often force an HTTP proxy that would break a raw WebSocket.

use reqwest::{header, Client, Response, StatusCode};
use serde::Deserialize;
use std::time::Duration;

/// The relay server holds a down-poll 25 s; leave room for slow proxies.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// Room expired or unknown (the server answers 401 for both).
    #[error("room expired")]
    RoomGone,
    /// Our relay slot was closed, usually because the phone left.
    #[error("relay slot closed")]
    SlotClosed,
    #[error("rate limited, retry in {0} s")]
    RateLimited(u64),
    #[error("server error {0}")]
    Status(StatusCode),
    #[error("network error: {0}")]
    Network(String),
}

impl From<reqwest::Error> for ClientError {
    fn from(e: reqwest::Error) -> Self {
        ClientError::Network(e.to_string())
    }
}

#[derive(Debug, PartialEq)]
pub enum Frame {
    Text(String),
    Binary(Vec<u8>),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RoomCreated {
    room_id: String,
    secret: String,
}

#[derive(Deserialize)]
struct SlotClaimed {
    token: String,
}

/// A room on the signaling server: id + secret, as encoded in the QR link.
#[derive(Clone)]
pub struct Room {
    pub id: String,
    pub secret: String,
}

#[derive(Clone)]
pub struct RelayClient {
    http: Client,
    origin: String,
}

/// `https://relay.plumed.fr/` -> `https://relay.plumed.fr` (the origin the server allows).
pub fn origin_of(server_url: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(server_url.trim()).map_err(|e| e.to_string())?;
    match (url.scheme(), url.host_str()) {
        ("https", Some(_)) => Ok(url.origin().ascii_serialization()),
        _ => Err("the relay URL must be https://".to_string()),
    }
}

/// The link the phone scans: `<origin>/remote-mic.html#<room>:<secret>`.
pub fn phone_link(origin: &str, room: &Room) -> String {
    format!("{}/remote-mic.html#{}:{}", origin, room.id, room.secret)
}

fn status_error(status: StatusCode, resp: &Response) -> ClientError {
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::NOT_FOUND => ClientError::RoomGone,
        StatusCode::GONE | StatusCode::CONFLICT => ClientError::SlotClosed,
        StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE => {
            let secs = resp
                .headers()
                .get(header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(5);
            ClientError::RateLimited(secs)
        }
        other => ClientError::Status(other),
    }
}

async fn check(resp: Response) -> Result<Response, ClientError> {
    let status = resp.status();
    match status.is_success() {
        true => Ok(resp),
        false => Err(status_error(status, &resp)),
    }
}

impl RelayClient {
    pub fn new(origin: String) -> Result<Self, ClientError> {
        let mut headers = header::HeaderMap::new();
        let origin_value = header::HeaderValue::from_str(&origin)
            .map_err(|e| ClientError::Network(e.to_string()))?;
        // The server refuses any request whose Origin is not in ALLOWED_ORIGINS
        headers.insert(header::ORIGIN, origin_value);
        let http = Client::builder()
            .default_headers(headers)
            .timeout(REQUEST_TIMEOUT)
            .https_only(true)
            .build()?;
        Ok(Self { http, origin })
    }

    pub fn origin(&self) -> &str {
        &self.origin
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/signal{}", self.origin, path)
    }

    pub async fn create_room(&self) -> Result<Room, ClientError> {
        let resp = check(self.http.post(self.url("/rooms")).send().await?).await?;
        let created: RoomCreated = resp.json().await?;
        Ok(Room {
            id: created.room_id,
            secret: created.secret,
        })
    }

    pub async fn post_offer(&self, room: &Room, sdp: &str) -> Result<(), ClientError> {
        let body = serde_json::json!({ "type": "offer", "sdp": sdp });
        let req = self
            .http
            .post(self.url(&format!("/rooms/{}/offer", room.id)))
            .header("X-Room-Secret", &room.secret)
            .json(&body);
        check(req.send().await?).await.map(|_| ())
    }

    /// Claims one of the room's two relay slots and returns its token.
    pub async fn claim_slot(&self, room: &Room) -> Result<String, ClientError> {
        let req = self
            .http
            .post(self.url(&format!("/rooms/{}/relay/handshake", room.id)))
            .header("X-Room-Secret", &room.secret);
        let claimed: SlotClaimed = check(req.send().await?).await?.json().await?;
        Ok(claimed.token)
    }

    pub async fn send_text(
        &self,
        room: &Room,
        token: &str,
        text: String,
    ) -> Result<(), ClientError> {
        let req = self
            .http
            .post(self.url(&format!("/rooms/{}/relay/up", room.id)))
            .header("X-Room-Secret", &room.secret)
            .header("X-Slot-Token", token)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(text);
        check(req.send().await?).await.map(|_| ())
    }

    /// Waits up to 25 s for one frame from the phone. `None` means nothing arrived.
    pub async fn receive(&self, room: &Room, token: &str) -> Result<Option<Frame>, ClientError> {
        let req = self
            .http
            .get(self.url(&format!("/rooms/{}/relay/down?wait=true", room.id)))
            .header("X-Room-Secret", &room.secret)
            .header("X-Slot-Token", token);
        let resp = check(req.send().await?).await?;
        if resp.status() == StatusCode::NO_CONTENT {
            return Ok(None);
        }
        let binary = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.starts_with("application/octet-stream"));
        let body = resp.bytes().await?.to_vec();
        match binary {
            true => Ok(Some(Frame::Binary(body))),
            false => Ok(Some(Frame::Text(
                String::from_utf8_lossy(&body).into_owned(),
            ))),
        }
    }

    /// What the phone sends: an encrypted audio packet.
    #[cfg(test)]
    pub async fn send_binary_for_test(&self, room: &Room, token: &str, packet: Vec<u8>) {
        let req = self
            .http
            .post(self.url(&format!("/rooms/{}/relay/up", room.id)))
            .header("X-Room-Secret", &room.secret)
            .header("X-Slot-Token", token)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .body(packet);
        check(req.send().await.unwrap()).await.unwrap();
    }

    /// Frees our slot (best effort: the server reaps idle slots after 60 s anyway).
    pub async fn close_slot(&self, room: &Room, token: &str) {
        let req = self
            .http
            .post(self.url(&format!("/rooms/{}/relay/close", room.id)))
            .header("X-Room-Secret", &room.secret)
            .header("X-Slot-Token", token);
        let _ = req.send().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_strips_path_and_requires_https() {
        assert_eq!(
            origin_of("https://relay.plumed.fr/").unwrap(),
            "https://relay.plumed.fr"
        );
        assert_eq!(
            origin_of(" https://Relay.Plumed.fr/x/y ").unwrap(),
            "https://relay.plumed.fr"
        );
        assert_eq!(
            origin_of("https://h.example:8443").unwrap(),
            "https://h.example:8443"
        );
        assert!(origin_of("http://relay.plumed.fr").is_err());
        assert!(origin_of("relay.plumed.fr").is_err());
    }

    #[test]
    fn phone_link_matches_parakeetweb_format() {
        let room = Room {
            id: "ABC234".into(),
            secret: "s3cr:et".into(),
        };
        assert_eq!(
            phone_link("https://relay.plumed.fr", &room),
            "https://relay.plumed.fr/remote-mic.html#ABC234:s3cr:et"
        );
    }
}
