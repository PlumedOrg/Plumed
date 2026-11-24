//! HTTP API of the relay, driven through the router without a socket.

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use http_body_util::BodyExt;
use plumed_relay::{app, Config};
use plumed_relay_protocol::page::{page_path, PAGE_VERSION};
use plumed_relay_protocol::{
    desktop_inbox_path, encode, phone_inbox_path, Frame, PROTOCOL_VERSION, ROOMS_PATH,
    SECRET_HEADER, VERSION_PATH,
};
use tower::ServiceExt;

fn router(max_rooms: usize, rooms_per_ip_per_hour: u32) -> Router {
    app(&Config {
        max_rooms,
        rooms_per_ip: max_rooms,
        rooms_per_ip_per_hour,
        trust_proxy: false,
    })
    .0
}

async fn call(router: &Router, method: Method, uri: &str, secret: &str, body: Vec<u8>) -> Response {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header(SECRET_HEADER, secret)
        .body(Body::from(body))
        .expect("valid request");
    router.clone().oneshot(req).await.expect("infallible")
}

async fn body(resp: Response) -> Vec<u8> {
    resp.into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes()
        .to_vec()
}

const TAG: &str = "AAAAAAAAAAAAAAAAAAAAAA";
const OTHER_TAG: &str = "BBBBBBBBBBBBBBBBBBBBBB";

struct Created {
    room: String,
    desktop: String,
    phone: String,
}

impl Created {
    fn desktop_inbox(&self) -> String {
        desktop_inbox_path(&self.room)
    }

    fn phone_inbox(&self, tag: &str) -> String {
        phone_inbox_path(&self.room, tag)
    }
}

async fn create(router: &Router) -> Created {
    let resp = call(router, Method::POST, ROOMS_PATH, "", vec![]).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json: serde_json::Value = serde_json::from_slice(&body(resp).await).expect("json");
    let field = |name: &str| json[name].as_str().expect("string").to_owned();
    Created {
        room: field("room"),
        desktop: field("desktop"),
        phone: field("phone"),
    }
}

fn envelope() -> Vec<u8> {
    encode(&[
        Frame::Text("{\"type\":\"hello\"}".into()),
        Frame::Binary(vec![1, 2, 3]),
    ])
}

#[tokio::test]
async fn health_and_security_headers() {
    let resp = call(&router(10, 10), Method::GET, "/healthz", "", vec![]).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let h = resp.headers();
    assert!(h[header::CONTENT_SECURITY_POLICY]
        .to_str()
        .unwrap()
        .contains("default-src 'none'"));
    assert_eq!(h[header::CACHE_CONTROL], "no-store");
    assert!(h.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
}

#[tokio::test]
async fn version_is_served() {
    let router = router(10, 100);
    let resp = call(&router, Method::GET, VERSION_PATH, "", vec![]).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json: serde_json::Value = serde_json::from_slice(&body(resp).await).unwrap();
    assert_eq!(json["protocol"], PROTOCOL_VERSION);
    let pages = json["pages"].as_array().unwrap();
    assert_eq!(pages.last().unwrap(), &serde_json::json!(PAGE_VERSION));
}

#[tokio::test]
async fn phone_page_is_served() {
    let root = call(&router(10, 10), Method::GET, "/", "", vec![]).await;
    let current = page_path(PAGE_VERSION, "/");
    assert_eq!(root.headers()[header::LOCATION], current.as_str());
    let resp = call(&router(10, 10), Method::GET, &current, "", vec![]).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp.headers()[header::CONTENT_TYPE]
        .to_str()
        .unwrap()
        .starts_with("text/html"));
    let page = String::from_utf8(body(resp).await).unwrap();
    assert!(page.contains("https://plumed.fr"));
}

#[tokio::test]
async fn frames_go_to_the_peer_inbox() {
    let r = router(10, 10);
    let c = create(&r).await;
    for _ in 0..2 {
        let sent = call(&r, Method::POST, &c.desktop_inbox(), &c.phone, envelope()).await;
        assert_eq!(sent.status(), StatusCode::NO_CONTENT);
    }
    let got = call(&r, Method::GET, &c.desktop_inbox(), &c.desktop, vec![]).await;
    assert_eq!(got.status(), StatusCode::OK);
    // Both envelopes, concatenated
    assert_eq!(body(got).await, [envelope(), envelope()].concat());

    let sent = call(
        &r,
        Method::POST,
        &c.phone_inbox(TAG),
        &c.desktop,
        envelope(),
    )
    .await;
    assert_eq!(sent.status(), StatusCode::NO_CONTENT);
    let got = call(&r, Method::GET, &c.phone_inbox(TAG), &c.phone, vec![]).await;
    assert_eq!(body(got).await, envelope());
}

#[tokio::test(start_paused = true)]
async fn phones_do_not_share_an_inbox() {
    let r = router(10, 10);
    let c = create(&r).await;
    call(
        &r,
        Method::POST,
        &c.phone_inbox(TAG),
        &c.desktop,
        envelope(),
    )
    .await;
    let other = call(&r, Method::GET, &c.phone_inbox(OTHER_TAG), &c.phone, vec![]).await;
    assert_eq!(other.status(), StatusCode::NO_CONTENT);
    let mine = call(&r, Method::GET, &c.phone_inbox(TAG), &c.phone, vec![]).await;
    assert_eq!(body(mine).await, envelope());
}

#[tokio::test(start_paused = true)]
async fn empty_poll_times_out_with_204() {
    let r = router(10, 10);
    let c = create(&r).await;
    let got = call(&r, Method::GET, &c.phone_inbox(TAG), &c.phone, vec![]).await;
    assert_eq!(got.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn waiting_poll_wakes_up_on_send() {
    let r = router(10, 10);
    let c = create(&r).await;
    let poll = {
        let (r, uri, secret) = (r.clone(), c.desktop_inbox(), c.desktop.clone());
        tokio::spawn(async move { call(&r, Method::GET, &uri, &secret, vec![]).await })
    };
    tokio::task::yield_now().await;
    call(&r, Method::POST, &c.desktop_inbox(), &c.phone, envelope()).await;
    let got = tokio::time::timeout(std::time::Duration::from_secs(5), poll)
        .await
        .expect("woken before the long-poll timeout")
        .expect("task");
    assert_eq!(body(got).await, envelope());
}

#[tokio::test]
async fn each_role_only_does_its_part() {
    let r = router(10, 10);
    let c = create(&r).await;
    let status = |method: Method, uri: String, secret: String, body: Vec<u8>| {
        let r = r.clone();
        async move { call(&r, method, &uri, &secret, body).await.status() }
    };
    let not_found = StatusCode::NOT_FOUND;
    // The QR (phone secret) can neither read the desktop inbox nor close the room
    assert_eq!(
        status(Method::GET, c.desktop_inbox(), c.phone.clone(), vec![]).await,
        not_found
    );
    let close = format!("{ROOMS_PATH}/{}", c.room);
    assert_eq!(
        status(Method::DELETE, close.clone(), c.phone.clone(), vec![]).await,
        not_found
    );
    // Nobody writes to their own inbox
    assert_eq!(
        status(
            Method::POST,
            c.desktop_inbox(),
            c.desktop.clone(),
            envelope()
        )
        .await,
        not_found
    );
    assert_eq!(
        status(
            Method::POST,
            c.phone_inbox(TAG),
            c.phone.clone(),
            envelope()
        )
        .await,
        not_found
    );
    assert_eq!(
        status(Method::GET, c.phone_inbox(TAG), c.desktop.clone(), vec![]).await,
        not_found
    );
    assert_eq!(
        status(Method::DELETE, close, c.desktop.clone(), vec![]).await,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn wrong_secret_bad_path_and_bad_envelope() {
    let r = router(10, 10);
    let c = create(&r).await;
    let inbox = c.desktop_inbox();
    let wrong = call(&r, Method::POST, &inbox, "nope", envelope()).await;
    assert_eq!(wrong.status(), StatusCode::NOT_FOUND);
    let unknown = call(
        &r,
        Method::POST,
        &desktop_inbox_path("zzz"),
        &c.phone,
        envelope(),
    )
    .await;
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
    let role = format!("{ROOMS_PATH}/{}/tv", c.room);
    let role = call(&r, Method::POST, &role, &c.phone, envelope()).await;
    assert_eq!(role.status(), StatusCode::NOT_FOUND);
    let short_tag = call(
        &r,
        Method::POST,
        &c.phone_inbox("abc"),
        &c.desktop,
        envelope(),
    )
    .await;
    assert_eq!(short_tag.status(), StatusCode::NOT_FOUND);
    let bad = call(&r, Method::POST, &inbox, &c.phone, vec![9, 0, 0, 0, 0]).await;
    assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn closed_room_answers_404() {
    let r = router(10, 10);
    let c = create(&r).await;
    let close = format!("{ROOMS_PATH}/{}", c.room);
    let closed = call(&r, Method::DELETE, &close, &c.desktop, vec![]).await;
    assert_eq!(closed.status(), StatusCode::NO_CONTENT);
    let got = call(&r, Method::GET, &c.phone_inbox(TAG), &c.phone, vec![]).await;
    assert_eq!(got.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn room_creation_is_rate_limited() {
    let r = router(10, 2);
    create(&r).await;
    create(&r).await;
    let resp = call(&r, Method::POST, ROOMS_PATH, "", vec![]).await;
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(resp.headers().contains_key(header::RETRY_AFTER));
}

#[tokio::test]
async fn browsers_cannot_create_rooms() {
    let r = router(10, 10);
    let req = Request::builder()
        .method(Method::POST)
        .uri(ROOMS_PATH)
        .header(header::ORIGIN, "https://evil.example")
        .body(Body::empty())
        .unwrap();
    let resp = r.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn full_server_answers_503() {
    let r = router(1, 10);
    create(&r).await;
    let resp = call(&r, Method::POST, ROOMS_PATH, "", vec![]).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn a_declared_oversized_body_is_refused_unread() {
    let router = router(10, 10);
    let room = create(&router).await;
    let req = Request::builder()
        .method(Method::POST)
        .uri(room.desktop_inbox())
        .header(SECRET_HEADER, &room.phone)
        .header(header::CONTENT_LENGTH, plumed_relay_protocol::MAX_BODY + 1)
        .body(Body::empty())
        .expect("valid request");
    let resp = router.clone().oneshot(req).await.expect("infallible");
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn one_client_cannot_hold_every_request_slot() {
    // One room allowed per client: four requests in flight at most
    let router = router(1, 10);
    let room = create(&router).await;
    let polls: Vec<_> = (0..4)
        .map(|_| {
            let (router, path, secret) =
                (router.clone(), room.desktop_inbox(), room.desktop.clone());
            tokio::spawn(async move { call(&router, Method::GET, &path, &secret, vec![]).await })
        })
        .collect();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let resp = call(&router, Method::GET, "/healthz", "", vec![]).await;
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    for poll in polls {
        poll.abort();
    }
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let resp = call(&router, Method::GET, "/healthz", "", vec![]).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
