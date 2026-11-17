//! HTTPS client for the Plumed relay (`remote_microphone_server/`), long-poll only.
//! Requests go through `reqwest`, which honors the system proxy: hospital
//! networks often force an HTTP proxy that would break a WebSocket.

use plumed_relay_protocol::{
    decode, desktop_inbox_path, encode, phone_inbox_path, Frame, ROOMS_PATH, SECRET_HEADER,
};
use reqwest::{header, Client, Response, StatusCode};
use serde::Deserialize;
use std::time::Duration;

/// The relay holds a poll `POLL_WAIT_SECS` (25 s): leave room for slow proxies.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// Room closed or unknown (the relay answers 404 for both).
    #[error("room expired")]
    RoomGone,
    #[error("rate limited, retry in {0} s")]
    RateLimited(u64),
    #[error("relay error {0}")]
    Status(StatusCode),
    #[error("invalid answer from the relay: {0}")]
    Invalid(String),
    #[error("network error: {0}")]
    Network(String),
}

impl From<reqwest::Error> for ClientError {
    fn from(e: reqwest::Error) -> Self {
        ClientError::Network(e.to_string())
    }
}

/// A room on the relay: its id and the secret of each side. Only the phone
/// secret goes into the QR link.
#[derive(Clone, Deserialize)]
pub struct Room {
    #[serde(rename = "room")]
    pub id: String,
    #[serde(rename = "desktop")]
    pub desktop_secret: String,
    #[serde(rename = "phone")]
    pub phone_secret: String,
}

#[derive(Clone)]
pub struct RelayClient {
    http: Client,
    origin: String,
}

/// `https://turn.plumed.fr/` -> `https://turn.plumed.fr`. Plain http is only
/// accepted for a relay on this machine (development and tests).
pub fn origin_of(server_url: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(server_url.trim()).map_err(|e| e.to_string())?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    match (url.scheme(), url.host_str()) {
        ("https", Some(_)) => Ok(url.origin().ascii_serialization()),
        ("http", Some(_)) if local => Ok(url.origin().ascii_serialization()),
        _ => Err("the relay URL must be https://".to_string()),
    }
}

/// The link the phone scans: `<origin>/#<room>.<phone secret>.<desktop public key>`.
pub fn phone_link(origin: &str, room: &Room, desktop_key_b64url: &str) -> String {
    format!(
        "{}/#{}.{}.{}",
        origin, room.id, room.phone_secret, desktop_key_b64url
    )
}

fn status_error(resp: &Response) -> ClientError {
    match resp.status() {
        StatusCode::NOT_FOUND => ClientError::RoomGone,
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
    match resp.status().is_success() {
        true => Ok(resp),
        false => Err(status_error(&resp)),
    }
}

impl RelayClient {
    pub fn new(origin: String) -> Result<Self, ClientError> {
        let http = Self::builder(&origin).build()?;
        Ok(Self { http, origin })
    }

    /// Redirects are refused: the room secret header would follow them to
    /// whatever host the relay (or a captive portal) points at.
    fn builder(origin: &str) -> reqwest::ClientBuilder {
        Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .https_only(origin.starts_with("https://"))
            .redirect(reqwest::redirect::Policy::none())
    }

    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub async fn create_room(&self) -> Result<Room, ClientError> {
        let url = format!("{}{}", self.origin, ROOMS_PATH);
        let resp = check(self.http.post(url).send().await?).await?;
        Ok(resp.json().await?)
    }

    /// Posts frames to the inbox at `path`, proving `secret`.
    pub async fn send_to(
        &self,
        path: &str,
        secret: &str,
        frames: &[Frame],
    ) -> Result<(), ClientError> {
        let resp = self
            .http
            .post(format!("{}{}", self.origin, path))
            .header(SECRET_HEADER, secret)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .body(encode(frames))
            .send()
            .await?;
        check(resp).await.map(|_| ())
    }

    /// Waits up to 25 s for frames in the inbox at `path` (empty: nothing came).
    pub async fn poll_from(&self, path: &str, secret: &str) -> Result<Vec<Frame>, ClientError> {
        let req = self
            .http
            .get(format!("{}{}", self.origin, path))
            .header(SECRET_HEADER, secret);
        let resp = check(req.send().await?).await?;
        if resp.status() == StatusCode::NO_CONTENT {
            return Ok(Vec::new());
        }
        decode(&resp.bytes().await?).map_err(|e| ClientError::Invalid(e.to_string()))
    }

    /// Posts frames to the phone that polls the inbox `tag`.
    pub async fn send(&self, room: &Room, tag: &str, frames: &[Frame]) -> Result<(), ClientError> {
        let path = phone_inbox_path(&room.id, tag);
        self.send_to(&path, &room.desktop_secret, frames).await
    }

    /// Waits for frames from the phones.
    pub async fn poll(&self, room: &Room) -> Result<Vec<Frame>, ClientError> {
        self.poll_from(&desktop_inbox_path(&room.id), &room.desktop_secret)
            .await
    }

    /// Best effort: the relay drops idle rooms anyway.
    pub async fn close(&self, room: &Room) {
        let url = format!("{}{}/{}", self.origin, ROOMS_PATH, room.id);
        let _ = self
            .http
            .delete(url)
            .header(SECRET_HEADER, &room.desktop_secret)
            .send()
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_is_normalized_and_https_only_except_locally() {
        assert_eq!(
            origin_of("https://turn.plumed.fr/").unwrap(),
            "https://turn.plumed.fr"
        );
        assert_eq!(
            origin_of(" https://Turn.Plumed.fr/x/y ").unwrap(),
            "https://turn.plumed.fr"
        );
        assert_eq!(
            origin_of("https://h.example:8443").unwrap(),
            "https://h.example:8443"
        );
        assert_eq!(
            origin_of("http://127.0.0.1:5174").unwrap(),
            "http://127.0.0.1:5174"
        );
        assert!(origin_of("http://turn.plumed.fr").is_err());
        assert!(origin_of("turn.plumed.fr").is_err());
    }

    #[tokio::test]
    async fn redirects_are_not_followed() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 2048];
            let _ = sock.read(&mut buf).await;
            let answer = "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:1/api/rooms\r\nContent-Length: 0\r\n\r\n";
            sock.write_all(answer.as_bytes()).await.unwrap();
        });
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = RelayClient::new(format!("http://{addr}")).unwrap();
        // Followed, the redirect would fail with a network error on port 1
        assert!(matches!(
            client.create_room().await,
            Err(ClientError::Status(StatusCode::TEMPORARY_REDIRECT))
        ));
    }

    #[test]
    fn phone_link_layout() {
        let room = Room {
            id: "AbC-_9".into(),
            desktop_secret: "d3sk".into(),
            phone_secret: "s3cr_et".into(),
        };
        assert_eq!(
            phone_link("https://turn.plumed.fr", &room, "BKEY"),
            "https://turn.plumed.fr/#AbC-_9.s3cr_et.BKEY"
        );
    }
}
