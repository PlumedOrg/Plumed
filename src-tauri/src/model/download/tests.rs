//! Downloads against a minimal local HTTP server whose answers each test
//! scripts (hub and relay on the same server, told apart by their paths).

use super::*;
use crate::model::store::tests::{install, temp_root, tiny_variant};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

enum Reply {
    /// The file, honouring a Range header.
    File,
    /// The file whole, ignoring any Range header.
    IgnoreRange,
    /// The file with its last byte changed.
    Corrupt,
    /// Announces the whole rest, sends `n` bytes, closes.
    CutAfter(usize),
    Status(u16),
    /// Never answers.
    Hang,
}

/// One request the server saw: path with query, and the Range header.
#[derive(Debug, Clone, PartialEq)]
struct Seen {
    path: String,
    range: Option<String>,
}

type Script = Arc<dyn Fn(&Seen, usize) -> Reply + Send + Sync>;

fn body_of(path: &str) -> &'static [u8] {
    match path.contains("a.bin") {
        true => b"alpha",
        false => b"beta!",
    }
}

fn is_relay(seen: &Seen) -> bool {
    seen.path.starts_with("/models/")
}

/// Starts the server, returns its endpoints and the requests log.
async fn serve(
    script: impl Fn(&Seen, usize) -> Reply + Send + Sync + 'static,
) -> (Endpoints, Arc<Mutex<Vec<Seen>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let log = Arc::new(Mutex::new(Vec::new()));
    let script: Script = Arc::new(script);
    let server_log = log.clone();
    tokio::spawn(async move {
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let (log, script) = (server_log.clone(), script.clone());
            tokio::spawn(answer(socket, log, script));
        }
    });
    let endpoints = Endpoints {
        hub: base.clone(),
        relay: base,
        hub_redirect: |_| false,
        backoff: Duration::from_millis(1),
    };
    (endpoints, log)
}

async fn answer(mut socket: tokio::net::TcpStream, log: Arc<Mutex<Vec<Seen>>>, script: Script) {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match socket.read(&mut byte).await {
            Ok(1) => head.push(byte[0]),
            _ => return,
        }
    }
    let head = String::from_utf8_lossy(&head).into_owned();
    let path = head.split_whitespace().nth(1).unwrap_or("").to_owned();
    let range = head.lines().find_map(|l| {
        l.to_ascii_lowercase()
            .strip_prefix("range: ")
            .map(str::to_owned)
    });
    let seen = Seen { path, range };
    let index = {
        let mut log = log.lock().unwrap();
        log.push(seen.clone());
        log.len() - 1
    };
    let body = body_of(&seen.path);
    let start: usize = seen
        .range
        .as_deref()
        .and_then(|r| r.strip_prefix("bytes=")?.strip_suffix('-')?.parse().ok())
        .unwrap_or(0);
    let reply = script(&seen, index);
    let response = match reply {
        Reply::Hang => {
            tokio::time::sleep(Duration::from_secs(3600)).await;
            return;
        }
        Reply::Status(code) => {
            format!("HTTP/1.1 {code} X\r\ncontent-length: 0\r\n\r\n").into_bytes()
        }
        Reply::IgnoreRange => ok(200, "", body),
        Reply::Corrupt => {
            let mut bad = body.to_vec();
            *bad.last_mut().unwrap() ^= 1;
            ok(200, "", &bad)
        }
        Reply::File | Reply::CutAfter(_) if start > 0 => {
            let range = format!(
                "content-range: bytes {start}-{}/{}\r\n",
                body.len() - 1,
                body.len()
            );
            ok(206, &range, &body[start..])
        }
        Reply::File | Reply::CutAfter(_) => ok(200, "", body),
    };
    let response = match reply {
        Reply::CutAfter(n) => {
            let header_end = response.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
            response[..header_end + n].to_vec()
        }
        _ => response,
    };
    let _ = socket.write_all(&response).await;
    let _ = socket.shutdown().await;
}

fn ok(status: u16, extra: &str, body: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "HTTP/1.1 {status} X\r\ncontent-length: {}\r\nconnection: close\r\n{extra}\r\n",
        body.len()
    )
    .into_bytes();
    out.extend_from_slice(body);
    out
}

fn not_cancelled() -> watch::Receiver<bool> {
    let (sender, receiver) = watch::channel(false);
    std::mem::forget(sender);
    receiver
}

async fn run(root: &Path, endpoints: &Endpoints) -> Result<Source, DownloadError> {
    download(root, tiny_variant(), endpoints, not_cancelled(), |_| {}).await
}

#[tokio::test]
async fn downloads_from_the_hub_and_marks_the_model_installed() {
    let (endpoints, log) = serve(|_, _| Reply::File).await;
    let root = temp_root();
    let variant = tiny_variant();
    assert_eq!(run(&root, &endpoints).await, Ok(Source::HuggingFace));
    assert_eq!(
        store::installed_source(&root, variant),
        Some(Source::HuggingFace)
    );
    assert!(store::verify(&root, variant).unwrap());
    let paths: Vec<String> = log.lock().unwrap().iter().map(|s| s.path.clone()).collect();
    assert_eq!(
        paths,
        [
            "/owner/tiny/resolve/0123456789abcdef0123456789abcdef01234567/x/a.bin",
            "/owner/tiny/resolve/0123456789abcdef0123456789abcdef01234567/b.bin",
        ]
    );
    let dir = store::variant_dir(&root, variant);
    assert!(!store::partial_path(&dir, variant.files[0].local_name).exists());
}

#[tokio::test]
async fn falls_back_to_the_relay_and_stays_there() {
    let (endpoints, log) = serve(|seen, _| match is_relay(seen) {
        true => Reply::File,
        false => Reply::Status(502),
    })
    .await;
    let root = temp_root();
    assert_eq!(run(&root, &endpoints).await, Ok(Source::Relay));
    let log = log.lock().unwrap();
    // Three hub tries for the first file, then only the relay.
    assert_eq!(
        log.iter().filter(|s| !is_relay(s)).count(),
        ATTEMPTS as usize
    );
    assert_eq!(
        log.last().unwrap().path,
        "/models/owner/tiny/b.bin?revision=0123456789abcdef0123456789abcdef01234567"
    );
}

#[tokio::test]
async fn resumes_a_partial_file_with_a_range_request() {
    let (endpoints, log) = serve(|_, _| Reply::File).await;
    let root = temp_root();
    let variant = tiny_variant();
    let dir = store::variant_dir(&root, variant);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        store::partial_path(&dir, variant.files[0].local_name),
        b"al",
    )
    .unwrap();
    assert!(run(&root, &endpoints).await.is_ok());
    assert_eq!(log.lock().unwrap()[0].range.as_deref(), Some("bytes=2-"));
    assert!(store::verify(&root, variant).unwrap());
}

#[tokio::test]
async fn a_cut_connection_is_resumed_where_it_stopped() {
    let (endpoints, log) = serve(|_, index| match index {
        0 => Reply::CutAfter(3),
        _ => Reply::File,
    })
    .await;
    let root = temp_root();
    assert!(run(&root, &endpoints).await.is_ok());
    let log = log.lock().unwrap();
    assert_eq!(log[0].range, None);
    assert_eq!(log[1].range.as_deref(), Some("bytes=3-"));
    assert!(store::verify(&root, tiny_variant()).unwrap());
}

#[tokio::test]
async fn a_server_ignoring_the_range_restarts_the_file() {
    let (endpoints, _) = serve(|_, _| Reply::IgnoreRange).await;
    let root = temp_root();
    let variant = tiny_variant();
    let dir = store::variant_dir(&root, variant);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        store::partial_path(&dir, variant.files[0].local_name),
        b"al",
    )
    .unwrap();
    assert!(run(&root, &endpoints).await.is_ok());
    assert!(store::verify(&root, variant).unwrap());
}

#[tokio::test]
async fn a_wrong_hash_is_never_installed() {
    let (endpoints, _) = serve(|seen, _| match is_relay(seen) {
        true => Reply::File,
        false => Reply::Corrupt,
    })
    .await;
    let root = temp_root();
    assert_eq!(
        run(&root, &endpoints).await,
        Ok(Source::Relay),
        "the relay has it right"
    );

    let (endpoints, _) = serve(|_, _| Reply::Corrupt).await;
    let root = temp_root();
    assert_eq!(
        run(&root, &endpoints).await,
        Err(DownloadError::Checksum {
            file: "x/a.bin".to_owned()
        })
    );
    assert_eq!(store::installed_source(&root, tiny_variant()), None);
    let dir = store::variant_dir(&root, tiny_variant());
    assert!(!dir.join(tiny_variant().files[0].local_name).exists());
}

#[tokio::test]
async fn installed_files_are_not_downloaded_again() {
    let (endpoints, log) = serve(|_, _| Reply::File).await;
    let root = temp_root();
    install(&root, tiny_variant());
    assert!(run(&root, &endpoints).await.is_ok());
    assert!(log.lock().unwrap().is_empty());
}

#[tokio::test]
async fn cancelling_stops_a_stalled_download() {
    let (endpoints, _) = serve(|_, _| Reply::Hang).await;
    let root = temp_root();
    let (cancel, receiver) = watch::channel(false);
    let job = download(&root, tiny_variant(), &endpoints, receiver, |_| {});
    let cancel_soon = async {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let _ = cancel.send(true);
    };
    let (result, ()) = tokio::join!(job, cancel_soon);
    assert_eq!(result, Err(DownloadError::Cancelled));
}

#[tokio::test]
async fn a_variant_that_cannot_run_is_refused() {
    let redux = crate::model::catalog::find("redux-w2a8").unwrap();
    let (endpoints, log) = serve(|_, _| Reply::File).await;
    let result = download(&temp_root(), redux, &endpoints, not_cancelled(), |_| {}).await;
    assert_eq!(result, Err(DownloadError::Unavailable));
    assert!(log.lock().unwrap().is_empty());
}

#[test]
fn the_relay_override_applies_unless_empty() {
    assert_eq!(relay_url(None), DEFAULT_RELAY_URL);
    assert_eq!(relay_url(Some("  ")), DEFAULT_RELAY_URL);
    assert_eq!(relay_url(Some("https://relay.test")), "https://relay.test");
}

#[test]
fn the_meter_throttles_and_smooths() {
    let start = Instant::now();
    let mut meter = Meter::new(start, 0);
    assert_eq!(meter.tick(start + Duration::from_millis(100), 100), None);
    let first = meter.tick(start + Duration::from_secs(1), 1000).unwrap();
    assert!((first - 1000.0).abs() < 1e-6);
    // 3000 B/s now: smoothed to 0.7 * 1000 + 0.3 * 3000
    let second = meter.tick(start + Duration::from_secs(2), 4000).unwrap();
    assert!((second - 1600.0).abs() < 1e-6);
}

#[test]
fn free_space_is_known_on_this_system() {
    assert!(available_space(&std::env::temp_dir()).is_some_and(|free| free > 0));
}
