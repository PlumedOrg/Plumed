//! HTTPS client for the Plumed relay (`PhoneMic_server/`), long-poll only.
//! Requests go through `reqwest`, which honors the system proxy: hospital
//! networks often force an HTTP proxy that would break a WebSocket.

use plumed_relay_protocol::page::PAGE;
use plumed_relay_protocol::{
    decode, desktop_inbox_path, encode, phone_inbox_path, Frame, MAX_BODY, PROTOCOL_VERSION,
    ROOMS_PATH, SECRET_HEADER, VERSION_PATH,
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
    /// The relay speaks another protocol version (`None`: too old to say).
    #[error("{}", protocol_mismatch_message(.0.as_deref().unwrap_or("?")))]
    ProtocolMismatch(Option<String>),
    /// The relay served a phone page that is not the one embedded in Plumed.
    #[error("{}", PAGE_TAMPERED)]
    PageTampered,
}

/// Translation key of a [`ClientError::ProtocolMismatch`] when
/// [`relay_is_newer`]: not suspicious, Plumed just needs an update.
pub const APP_OUTDATED: &str = "The relay uses a newer PhoneMic protocol (v{{relay}}, ours v{{ours}}): update Plumed to use it.";

/// A relay on a newer protocol version than ours means Plumed is out of date.
/// The page is pinned to the protocol version (see `page.rs`), so a newer
/// page always comes with a newer version and never looks like tampering.
pub fn relay_is_newer(relay: &str) -> bool {
    match (
        semver::Version::parse(relay),
        semver::Version::parse(PROTOCOL_VERSION),
    ) {
        (Ok(relay), Ok(ours)) => relay > ours,
        _ => false,
    }
}

/// Translation key of [`ClientError::PageTampered`].
pub const PAGE_TAMPERED: &str = "The relay served a phone page that differs from the one of its protocol version: it may be compromised. PhoneMic was stopped.";

/// What a phone browser sends, so the page check looks like a phone. Only the
/// headers: the TLS handshake of reqwest still differs from a browser's (see
/// the threat model in protocol_details.md).
const PHONE_USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36";

/// `Ok` only when the relay served `path` exactly as embedded in Plumed.
pub fn check_page_answer(path: &str, status: StatusCode, body: &[u8]) -> Result<(), ClientError> {
    let expected = PAGE.iter().find(|a| a.path == path).map(|a| a.body);
    match status.is_success() && expected == Some(body) {
        true => Ok(()),
        false => Err(ClientError::PageTampered),
    }
}

/// Translation key of [`ClientError::ProtocolMismatch`], `{{relay}}` and
/// `{{ours}}` filled by the interface (`Snapshot::error_params`).
pub const PROTOCOL_MISMATCH: &str = "The relay is serving encryption protocol v{{relay}} and our version is v{{ours}}, so we can't use it. Backward compatibility of the protocol is not supported on purpose, to reduce the attack surface.";

fn protocol_mismatch_message(relay: &str) -> String {
    PROTOCOL_MISMATCH
        .replace("{{relay}}", relay)
        .replace("{{ours}}", PROTOCOL_VERSION)
}

/// `Ok` only when the relay's version answer is exactly ours.
pub fn check_version_answer(status: StatusCode, body: &[u8]) -> Result<(), ClientError> {
    if status == StatusCode::NOT_FOUND {
        // Relays before versioning have no such route
        return Err(ClientError::ProtocolMismatch(None));
    }
    if !status.is_success() {
        return Err(ClientError::Status(status));
    }
    #[derive(Deserialize)]
    struct Answer {
        protocol: String,
    }
    let answer: Answer =
        serde_json::from_slice(body).map_err(|e| ClientError::Invalid(e.to_string()))?;
    match answer.protocol == PROTOCOL_VERSION {
        true => Ok(()),
        // Truncated: the relay is not trusted, its text ends up on screen
        false => Err(ClientError::ProtocolMismatch(Some(
            answer.protocol.chars().take(32).collect(),
        ))),
    }
}

impl From<reqwest::Error> for ClientError {
    /// Without the URL: it names the room and the phone inbox, and this text
    /// ends up in logs and on screen.
    fn from(e: reqwest::Error) -> Self {
        ClientError::Network(e.without_url().to_string())
    }
}

/// The body of `resp`, refused past `MAX_BODY`: an honest relay never queues
/// more for one inbox, a malicious one must not exhaust our memory.
async fn bounded_body(mut resp: Response) -> Result<Vec<u8>, ClientError> {
    let mut body = Vec::new();
    while let Some(chunk) = resp.chunk().await? {
        if body.len() + chunk.len() > MAX_BODY {
            return Err(ClientError::Invalid("answer too large".to_string()));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
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

/// The link the phone scans:
/// `<origin>/#<room>.<phone secret>.<desktop public key>.<pairing secret>`,
/// `pairing` being `Pairing::link_key`.
pub fn phone_link(origin: &str, room: &Room, pairing: &str) -> String {
    format!("{}/#{}.{}.{}", origin, room.id, room.phone_secret, pairing)
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

    /// Fetches every file of the phone page like a phone would (fresh
    /// connection, phone headers, no room secret) and compares it with the
    /// page embedded in Plumed.
    pub async fn check_page(&self) -> Result<(), ClientError> {
        let http = Self::builder(&self.origin)
            .user_agent(PHONE_USER_AGENT)
            .build()?;
        for asset in PAGE {
            let resp = http
                .get(format!("{}{}", self.origin, asset.path))
                .header(header::ACCEPT_LANGUAGE, "fr-FR,fr;q=0.9")
                .send()
                .await?;
            let status = resp.status();
            check_page_answer(asset.path, status, &bounded_body(resp).await?)?;
        }
        Ok(())
    }

    /// Refuses a relay on another protocol version (see [`PROTOCOL_VERSION`]).
    pub async fn check_version(&self) -> Result<(), ClientError> {
        let url = format!("{}{}", self.origin, VERSION_PATH);
        let resp = self.http.get(url).send().await?;
        let status = resp.status();
        let body = match status.is_success() {
            true => bounded_body(resp).await?,
            false => Vec::new(),
        };
        check_version_answer(status, &body)
    }

    pub async fn create_room(&self) -> Result<Room, ClientError> {
        let url = format!("{}{}", self.origin, ROOMS_PATH);
        let resp = check(self.http.post(url).send().await?).await?;
        serde_json::from_slice(&bounded_body(resp).await?)
            .map_err(|e| ClientError::Invalid(e.to_string()))
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
        decode(&bounded_body(resp).await?).map_err(|e| ClientError::Invalid(e.to_string()))
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
    fn only_a_newer_relay_means_plumed_is_out_of_date() {
        assert!(relay_is_newer("999.0.0"));
        assert!(!relay_is_newer("0.0.1"));
        assert!(!relay_is_newer(PROTOCOL_VERSION));
        assert!(!relay_is_newer("garbage"));
    }

    #[test]
    fn only_the_embedded_page_passes_the_check() {
        let index = &PAGE[0];
        assert!(check_page_answer(index.path, StatusCode::OK, index.body).is_ok());
        let mut tampered = index.body.to_vec();
        tampered.extend_from_slice(b"<script src=//evil.example></script>");
        for (path, status, body) in [
            (index.path, StatusCode::OK, &tampered[..]),
            (index.path, StatusCode::NOT_FOUND, index.body),
            ("/unknown.js", StatusCode::OK, index.body),
        ] {
            assert!(matches!(
                check_page_answer(path, status, body),
                Err(ClientError::PageTampered)
            ));
        }
    }

    #[test]
    fn only_the_exact_protocol_version_is_accepted() {
        let ok = format!(r#"{{"protocol":"{PROTOCOL_VERSION}"}}"#);
        assert!(check_version_answer(StatusCode::OK, ok.as_bytes()).is_ok());
        let other = check_version_answer(StatusCode::OK, br#"{"protocol":"99.0.0"}"#);
        assert!(matches!(other, Err(ClientError::ProtocolMismatch(Some(v))) if v == "99.0.0"));
        let message = check_version_answer(StatusCode::OK, br#"{"protocol":"99.0.0"}"#)
            .unwrap_err()
            .to_string();
        assert!(message.contains("v99.0.0") && message.contains(&format!("v{PROTOCOL_VERSION}")));
        assert!(matches!(
            check_version_answer(StatusCode::NOT_FOUND, b""),
            Err(ClientError::ProtocolMismatch(None))
        ));
        assert!(matches!(
            check_version_answer(StatusCode::OK, b"not json"),
            Err(ClientError::Invalid(_))
        ));
        let long = format!(r#"{{"protocol":"{}"}}"#, "9".repeat(500));
        assert!(matches!(
            check_version_answer(StatusCode::OK, long.as_bytes()),
            Err(ClientError::ProtocolMismatch(Some(v))) if v.len() == 32
        ));
    }

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
        let answer = "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:1/api/rooms\r\nContent-Length: 0\r\n\r\n";
        let addr = one_shot_server(answer.as_bytes().to_vec()).await;
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = RelayClient::new(format!("http://{addr}")).unwrap();
        // Followed, the redirect would fail with a network error on port 1
        assert!(matches!(
            client.create_room().await,
            Err(ClientError::Status(StatusCode::TEMPORARY_REDIRECT))
        ));
    }

    /// Serves `answer` to the first request on a local port.
    async fn one_shot_server(answer: Vec<u8>) -> std::net::SocketAddr {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 2048];
            let _ = sock.read(&mut buf).await;
            let _ = sock.write_all(&answer).await;
        });
        addr
    }

    #[tokio::test]
    async fn oversized_poll_answers_are_refused() {
        let body = vec![0u8; MAX_BODY + 1];
        let mut answer =
            format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len()).into_bytes();
        answer.extend(body);
        let addr = one_shot_server(answer).await;
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = RelayClient::new(format!("http://{addr}")).unwrap();
        assert!(matches!(
            client.poll_from("/api/rooms/r/desktop", "s").await,
            Err(ClientError::Invalid(_))
        ));
    }

    #[tokio::test]
    async fn oversized_room_answers_are_refused() {
        let body = format!(
            r#"{{"room":"{}","desktop":"d","phone":"p"}}"#,
            "a".repeat(MAX_BODY)
        );
        let mut answer = format!(
            "HTTP/1.1 201 Created\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .into_bytes();
        answer.extend(body.into_bytes());
        let addr = one_shot_server(answer).await;
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = RelayClient::new(format!("http://{addr}")).unwrap();
        assert!(matches!(
            client.create_room().await,
            Err(ClientError::Invalid(_))
        ));
    }

    #[tokio::test]
    async fn network_errors_do_not_name_the_room() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        // Nothing listens on port 1
        let client = RelayClient::new("http://127.0.0.1:1".to_string()).unwrap();
        let Err(ClientError::Network(text)) =
            client.poll_from("/api/rooms/R00M/desktop", "s").await
        else {
            panic!("expected a network error")
        };
        assert!(!text.contains("R00M"), "{text}");
    }

    #[test]
    fn phone_link_layout() {
        let room = Room {
            id: "AbC-_9".into(),
            desktop_secret: "d3sk".into(),
            phone_secret: "s3cr_et".into(),
        };
        assert_eq!(
            phone_link("https://turn.plumed.fr", &room, "BKEY.PSEC"),
            "https://turn.plumed.fr/#AbC-_9.s3cr_et.BKEY.PSEC"
        );
    }
}
