//! The model relay over real sockets, against a fake Hugging Face (hub that
//! redirects to a CDN, like the real one), as ParakeetWeb's tier-2 tests do.

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use plumed_models::{app, Allowlist, Config};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

const ETAG: &str = "\"v1\"";
const REPO: &str = "o/r";
const REVISION: &str = "rev1";

fn data() -> Vec<u8> {
    (0..1000u32).map(|i| (i % 251) as u8).collect()
}

#[derive(Default)]
struct Seen {
    hub: AtomicUsize,
    cdn: AtomicUsize,
    hub_auth: AtomicUsize,
    cdn_auth: AtomicUsize,
    github: AtomicUsize,
    api: AtomicUsize,
}

struct Fake {
    seen: Arc<Seen>,
    addr: SocketAddr,
    /// A fake github.com and api.github.com, sharing the CDN.
    github: SocketAddr,
}

async fn listen(router: Router) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let _ = axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await;
    });
    addr
}

fn has_auth(headers: &HeaderMap) -> bool {
    headers.contains_key(header::AUTHORIZATION)
}

async fn hub(
    State((seen, port)): State<(Arc<Seen>, u16)>,
    Path((_owner, _name, _rev, file)): Path<(String, String, String, String)>,
    headers: HeaderMap,
) -> Response {
    seen.hub.fetch_add(1, Ordering::SeqCst);
    if has_auth(&headers) {
        seen.hub_auth.fetch_add(1, Ordering::SeqCst);
    }
    match file.as_str() {
        "missing.bin" => StatusCode::NOT_FOUND.into_response(),
        "small.txt" => "hello".into_response(),
        "evil.bin" => (
            StatusCode::FOUND,
            [(header::LOCATION, "https://evil.example/x")],
        )
            .into_response(),
        _ => (
            StatusCode::FOUND,
            [
                (
                    header::LOCATION,
                    format!("http://127.0.0.1:{port}/cdn/{file}"),
                ),
                (
                    header::HeaderName::from_static("x-linked-size"),
                    "1000".to_owned(),
                ),
                (
                    header::HeaderName::from_static("x-linked-etag"),
                    ETAG.to_owned(),
                ),
            ],
        )
            .into_response(),
    }
}

async fn cdn(
    State(seen): State<Arc<Seen>>,
    Path(file): Path<String>,
    headers: HeaderMap,
) -> Response {
    seen.cdn.fetch_add(1, Ordering::SeqCst);
    if has_auth(&headers) {
        seen.cdn_auth.fetch_add(1, Ordering::SeqCst);
    }
    if file == "slow.bin" {
        // One byte, then nothing for a while: the stream stays open
        let body = futures_util::stream::unfold(0u8, |step| async move {
            match step {
                0 => Some((Ok::<_, std::io::Error>(vec![1u8]), 1)),
                _ => {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    None
                }
            }
        });
        return Body::from_stream(body).into_response();
    }
    let get = |name: header::HeaderName| headers.get(name).and_then(|v| v.to_str().ok());
    if get(header::IF_NONE_MATCH) == Some(ETAG) {
        return (StatusCode::NOT_MODIFIED, [(header::ETAG, ETAG)]).into_response();
    }
    let data = data();
    let fresh = get(header::IF_RANGE).is_none_or(|v| v == ETAG);
    let start = get(header::RANGE)
        .and_then(|r| r.strip_prefix("bytes="))
        .and_then(|r| r.strip_suffix('-'))
        .and_then(|r| r.parse::<usize>().ok());
    match (start, fresh) {
        (Some(start), true) => (
            StatusCode::PARTIAL_CONTENT,
            [
                (header::ETAG, ETAG.to_owned()),
                (header::ACCEPT_RANGES, "bytes".to_owned()),
                (
                    header::CONTENT_RANGE,
                    format!("bytes {start}-{}/{}", data.len() - 1, data.len()),
                ),
            ],
            data[start..].to_vec(),
        )
            .into_response(),
        _ => (
            [
                (header::ETAG, ETAG.to_owned()),
                (header::ACCEPT_RANGES, "bytes".to_owned()),
            ],
            data,
        )
            .into_response(),
    }
}

const LISTING: &str = r#"[{"tag_name":"1.1.0-beta.1","prerelease":true}]"#;

/// github.com: `latest` redirects to a tag, assets redirect to the CDN.
async fn github(
    State((seen, port)): State<(Arc<Seen>, u16)>,
    Path((tag, asset)): Path<(String, String)>,
) -> Response {
    seen.github.fetch_add(1, Ordering::SeqCst);
    let location = match (tag.as_str(), asset.as_str()) {
        ("latest", "latest.json") => "/PlumedOrg/Plumed/releases/download/1.0.0/latest.json".into(),
        (_, "evil.bin") => "https://evil.example/x".to_owned(),
        (_, "missing.bin") => return StatusCode::NOT_FOUND.into_response(),
        _ => format!("http://127.0.0.1:{port}/cdn/{asset}"),
    };
    (StatusCode::FOUND, [(header::LOCATION, location)]).into_response()
}

async fn api(State(seen): State<Arc<Seen>>, uri: axum::http::Uri) -> Response {
    seen.api.fetch_add(1, Ordering::SeqCst);
    match uri.query() {
        Some("per_page=10") => {
            ([(header::CONTENT_TYPE, "application/json")], LISTING).into_response()
        }
        _ => StatusCode::BAD_REQUEST.into_response(),
    }
}

async fn fake_hub() -> Fake {
    let seen = Arc::new(Seen::default());
    // The CDN first, so the hub knows where to redirect
    let cdn_router = Router::new()
        .route("/cdn/{*file}", get(cdn))
        .with_state(seen.clone());
    let cdn_addr = listen(cdn_router).await;
    let hub_router = Router::new()
        .route("/{owner}/{name}/resolve/{rev}/{*file}", get(hub))
        .with_state((seen.clone(), cdn_addr.port()));
    let addr = listen(hub_router).await;
    let github_router = Router::new()
        .route(
            "/PlumedOrg/Plumed/releases/download/{tag}/{asset}",
            get(github).with_state((seen.clone(), cdn_addr.port())),
        )
        .route(
            "/PlumedOrg/Plumed/releases/{tag}/download/{asset}",
            get(github).with_state((seen.clone(), cdn_addr.port())),
        )
        .route(
            "/repos/PlumedOrg/Plumed/releases",
            get(api).with_state(seen.clone()),
        );
    let github = listen(github_router).await;
    Fake { seen, addr, github }
}

/// Test redirects stay on this machine (the real relay: hub hosts only).
fn local_only(url: &reqwest::Url) -> bool {
    url.host_str() == Some("127.0.0.1")
}

async fn relay(fake: &Fake, token: Option<&str>, streams_per_ip: usize) -> String {
    let router = app(Config {
        allowlist: Allowlist::parse(&format!("{REPO}@{REVISION}")).expect("allowlist"),
        token: token.map(str::to_owned),
        upstream: format!("http://{}", fake.addr),
        redirect_allowed: local_only,
        github: format!("http://{}", fake.github),
        github_api: format!("http://{}", fake.github),
        github_redirect: local_only,
        max_streams: 16,
        streams_per_ip,
        trust_proxy: false,
    })
    .expect("relay");
    format!("http://{}", listen(router).await)
}

fn file_url(relay: &str, file: &str) -> String {
    format!("{relay}/models/{REPO}/{file}?revision={REVISION}")
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("client")
}

fn header_of(resp: &reqwest::Response, name: header::HeaderName) -> Option<&str> {
    resp.headers().get(name).and_then(|v| v.to_str().ok())
}

#[tokio::test]
async fn streams_a_redirected_file_with_length_etag_and_ranges() {
    let fake = fake_hub().await;
    let relay = relay(&fake, None, 8).await;
    let resp = client()
        .get(file_url(&relay, "int8/encoder.onnx"))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 200);
    assert_eq!(header_of(&resp, header::CONTENT_LENGTH), Some("1000"));
    assert_eq!(header_of(&resp, header::ETAG), Some(ETAG));
    assert_eq!(header_of(&resp, header::ACCEPT_RANGES), Some("bytes"));
    assert_eq!(resp.bytes().await.expect("body").to_vec(), data());
}

#[tokio::test]
async fn forwards_range_and_if_range_and_returns_the_slice() {
    let fake = fake_hub().await;
    let relay = relay(&fake, None, 8).await;
    let resp = client()
        .get(file_url(&relay, "int8/encoder.onnx"))
        .header(header::RANGE, "bytes=600-")
        .header(header::IF_RANGE, ETAG)
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 206);
    assert_eq!(
        header_of(&resp, header::CONTENT_RANGE),
        Some("bytes 600-999/1000")
    );
    assert_eq!(header_of(&resp, header::CONTENT_LENGTH), Some("400"));
    assert_eq!(resp.bytes().await.expect("body").to_vec(), data()[600..]);
}

#[tokio::test]
async fn a_stale_if_range_gets_the_whole_file() {
    let fake = fake_hub().await;
    let relay = relay(&fake, None, 8).await;
    let resp = client()
        .get(file_url(&relay, "int8/encoder.onnx"))
        .header(header::RANGE, "bytes=600-")
        .header(header::IF_RANGE, "\"old\"")
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.bytes().await.expect("body").len(), 1000);
}

#[tokio::test]
async fn if_none_match_round_trips_a_304() {
    let fake = fake_hub().await;
    let relay = relay(&fake, None, 8).await;
    let resp = client()
        .get(file_url(&relay, "int8/encoder.onnx"))
        .header(header::IF_NONE_MATCH, ETAG)
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 304);
}

#[tokio::test]
async fn head_answers_from_the_hub_without_touching_the_cdn() {
    let fake = fake_hub().await;
    let relay = relay(&fake, None, 8).await;
    let resp = client()
        .head(file_url(&relay, "int8/encoder.onnx"))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 200);
    assert_eq!(header_of(&resp, header::CONTENT_LENGTH), Some("1000"));
    assert_eq!(header_of(&resp, header::ETAG), Some(ETAG));
    assert_eq!(fake.seen.cdn.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_small_file_served_by_the_hub_itself_passes_through() {
    let fake = fake_hub().await;
    let relay = relay(&fake, None, 8).await;
    let resp = client()
        .get(file_url(&relay, "small.txt"))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.expect("body"), "hello");
}

#[tokio::test]
async fn the_token_reaches_the_hub_but_never_the_cdn() {
    let fake = fake_hub().await;
    let relay = relay(&fake, Some("hf_secret"), 8).await;
    let resp = client()
        .get(file_url(&relay, "int8/encoder.onnx"))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 200);
    assert_eq!(fake.seen.hub_auth.load(Ordering::SeqCst), 1);
    assert_eq!(fake.seen.cdn.load(Ordering::SeqCst), 1);
    assert_eq!(fake.seen.cdn_auth.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn anything_outside_the_allowlist_is_a_404_without_contacting_the_hub() {
    let fake = fake_hub().await;
    let relay = relay(&fake, None, 8).await;
    for url in [
        format!("{relay}/models/other/repo/x.onnx?revision={REVISION}"),
        format!("{relay}/models/{REPO}/x.onnx?revision=main"),
        format!("{relay}/models/{REPO}/x.onnx"),
        format!("{relay}/models/{REPO}/%2e%2e/x.onnx?revision={REVISION}"),
        format!("{relay}/other"),
    ] {
        let resp = client().get(&url).send().await.expect("send");
        assert_eq!(resp.status(), 404, "{url}");
    }
    assert_eq!(fake.seen.hub.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_missing_file_is_a_404_and_other_methods_are_refused() {
    let fake = fake_hub().await;
    let relay = relay(&fake, None, 8).await;
    let missing = client()
        .get(file_url(&relay, "missing.bin"))
        .send()
        .await
        .expect("send");
    assert_eq!(missing.status(), 404);
    let post = client()
        .request(Method::POST, file_url(&relay, "x.onnx"))
        .send()
        .await
        .expect("send");
    assert_eq!(post.status(), 405);
}

#[tokio::test]
async fn a_redirect_off_the_hub_is_a_502() {
    let fake = fake_hub().await;
    let relay = relay(&fake, None, 8).await;
    let resp = client()
        .get(file_url(&relay, "evil.bin"))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 502);
}

#[tokio::test]
async fn a_stream_beyond_the_per_client_cap_gets_503_until_one_ends() {
    let fake = fake_hub().await;
    let relay = relay(&fake, None, 1).await;
    let first = client()
        .get(file_url(&relay, "slow.bin"))
        .send()
        .await
        .expect("send");
    assert_eq!(first.status(), 200);
    let second = client()
        .get(file_url(&relay, "int8/encoder.onnx"))
        .send()
        .await
        .expect("send");
    assert_eq!(second.status(), 503);
    assert_eq!(header_of(&second, header::RETRY_AFTER), Some("5"));
    drop(first);
    // The slot is released once the relay notices the client left
    let mut status = 0;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let third = client()
            .get(file_url(&relay, "int8/encoder.onnx"))
            .send()
            .await
            .expect("send");
        status = third.status().as_u16();
        if status == 200 {
            break;
        }
    }
    assert_eq!(status, 200);
}

#[tokio::test]
async fn healthz_answers_ok() {
    let fake = fake_hub().await;
    let relay = relay(&fake, None, 8).await;
    let resp = client()
        .get(format!("{relay}/healthz"))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.text().await.expect("body"), "ok");
}

#[tokio::test]
async fn latest_json_follows_github_redirects_to_the_cdn() {
    let fake = fake_hub().await;
    let relay = relay(&fake, Some("hf-secret"), 8).await;
    let resp = client()
        .get(format!("{relay}/releases/latest/download/latest.json"))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.bytes().await.expect("body").to_vec(), data());
    // latest -> tag -> CDN, and the Hugging Face token never reaches GitHub
    assert_eq!(fake.seen.github.load(Ordering::SeqCst), 2);
    assert_eq!(fake.seen.cdn_auth.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_release_asset_streams_with_range_passthrough() {
    let fake = fake_hub().await;
    let relay = relay(&fake, None, 8).await;
    let resp = client()
        .get(format!(
            "{relay}/releases/download/1.0.0/Plumed_x64-setup.exe"
        ))
        .header(header::RANGE, "bytes=990-")
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 206);
    assert_eq!(
        header_of(&resp, header::CONTENT_RANGE),
        Some("bytes 990-999/1000")
    );
    assert_eq!(
        resp.bytes().await.expect("body").to_vec(),
        data()[990..].to_vec()
    );
}

#[tokio::test]
async fn release_junk_is_a_404_without_contacting_github() {
    let fake = fake_hub().await;
    let relay = relay(&fake, None, 8).await;
    for path in [
        "/releases/latest/download/other.json",
        "/releases/download/1.0.0/a/b",
        "/releases/download/1.0.0/%2e%2e",
        "/releases/download/1.0.0/x.exe?y=1",
        "/releases/list?per_page=100",
        "/releases/PlumedOrg/Plumed/x",
    ] {
        let resp = client()
            .get(format!("{relay}{path}"))
            .send()
            .await
            .expect("send");
        assert_eq!(resp.status(), 404, "{path}");
    }
    assert_eq!(fake.seen.github.load(Ordering::SeqCst), 0);
    assert_eq!(fake.seen.api.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn release_redirects_off_github_and_missing_assets_are_refused() {
    let fake = fake_hub().await;
    let relay = relay(&fake, None, 8).await;
    for (asset, status) in [("evil.bin", 502), ("missing.bin", 404)] {
        let resp = client()
            .get(format!("{relay}/releases/download/1.0.0/{asset}"))
            .send()
            .await
            .expect("send");
        assert_eq!(resp.status(), status, "{asset}");
    }
}

#[tokio::test]
async fn the_release_listing_is_fetched_once_then_cached() {
    let fake = fake_hub().await;
    let relay = relay(&fake, None, 8).await;
    for _ in 0..3 {
        let resp = client()
            .get(format!("{relay}/releases/list"))
            .send()
            .await
            .expect("send");
        assert_eq!(resp.status(), 200);
        assert_eq!(
            header_of(&resp, header::CONTENT_TYPE),
            Some("application/json")
        );
        assert_eq!(resp.text().await.expect("body"), LISTING);
    }
    assert_eq!(fake.seen.api.load(Ordering::SeqCst), 1);
}
