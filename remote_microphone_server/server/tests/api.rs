//! HTTP API of the relay, driven through the router without a socket.

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use http_body_util::BodyExt;
use plumed_relay::{app, Config};
use plumed_relay_protocol::{encode, Frame, ROOMS_PATH, SECRET_HEADER};
use tower::ServiceExt;

fn router(max_rooms: usize, rooms_per_ip_per_hour: u32) -> Router {
    app(&Config {
        max_rooms,
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

/// -> (room, secret)
async fn create(router: &Router) -> (String, String) {
    let resp = call(router, Method::POST, ROOMS_PATH, "", vec![]).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json: serde_json::Value = serde_json::from_slice(&body(resp).await).expect("json");
    let field = |name: &str| json[name].as_str().expect("string").to_owned();
    (field("room"), field("secret"))
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
async fn phone_page_is_served() {
    let resp = call(&router(10, 10), Method::GET, "/", "", vec![]).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp.headers()[header::CONTENT_TYPE]
        .to_str()
        .unwrap()
        .starts_with("text/html"));
    let page = String::from_utf8(body(resp).await).unwrap();
    assert!(page.contains("https://plumed.fr"));
}

#[tokio::test]
async fn frames_go_to_the_peer_mailbox() {
    let r = router(10, 10);
    let (room, secret) = create(&r).await;
    let phone = format!("{ROOMS_PATH}/{room}/phone");
    let desktop = format!("{ROOMS_PATH}/{room}/desktop");
    let sent = call(&r, Method::POST, &phone, &secret, envelope()).await;
    assert_eq!(sent.status(), StatusCode::NO_CONTENT);
    let sent = call(&r, Method::POST, &phone, &secret, envelope()).await;
    assert_eq!(sent.status(), StatusCode::NO_CONTENT);
    let got = call(&r, Method::GET, &desktop, &secret, vec![]).await;
    assert_eq!(got.status(), StatusCode::OK);
    // Both envelopes, concatenated
    assert_eq!(body(got).await, [envelope(), envelope()].concat());
}

#[tokio::test(start_paused = true)]
async fn empty_poll_times_out_with_204() {
    let r = router(10, 10);
    let (room, secret) = create(&r).await;
    let got = call(
        &r,
        Method::GET,
        &format!("{ROOMS_PATH}/{room}/phone"),
        &secret,
        vec![],
    )
    .await;
    assert_eq!(got.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn waiting_poll_wakes_up_on_send() {
    let r = router(10, 10);
    let (room, secret) = create(&r).await;
    let poll = {
        let (r, uri, secret) = (
            r.clone(),
            format!("{ROOMS_PATH}/{room}/desktop"),
            secret.clone(),
        );
        tokio::spawn(async move { call(&r, Method::GET, &uri, &secret, vec![]).await })
    };
    tokio::task::yield_now().await;
    call(
        &r,
        Method::POST,
        &format!("{ROOMS_PATH}/{room}/phone"),
        &secret,
        envelope(),
    )
    .await;
    let got = tokio::time::timeout(std::time::Duration::from_secs(5), poll)
        .await
        .expect("woken before the long-poll timeout")
        .expect("task");
    assert_eq!(body(got).await, envelope());
}

#[tokio::test]
async fn wrong_secret_bad_role_and_bad_envelope() {
    let r = router(10, 10);
    let (room, secret) = create(&r).await;
    let phone = format!("{ROOMS_PATH}/{room}/phone");
    let wrong = call(&r, Method::POST, &phone, "nope", envelope()).await;
    assert_eq!(wrong.status(), StatusCode::NOT_FOUND);
    let unknown = call(
        &r,
        Method::POST,
        &format!("{ROOMS_PATH}/zzz/phone"),
        &secret,
        envelope(),
    )
    .await;
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
    let role = call(
        &r,
        Method::POST,
        &format!("{ROOMS_PATH}/{room}/tv"),
        &secret,
        envelope(),
    )
    .await;
    assert_eq!(role.status(), StatusCode::NOT_FOUND);
    let bad = call(&r, Method::POST, &phone, &secret, vec![9, 0, 0, 0, 0]).await;
    assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn closed_room_answers_404() {
    let r = router(10, 10);
    let (room, secret) = create(&r).await;
    let closed = call(
        &r,
        Method::DELETE,
        &format!("{ROOMS_PATH}/{room}"),
        &secret,
        vec![],
    )
    .await;
    assert_eq!(closed.status(), StatusCode::NO_CONTENT);
    let got = call(
        &r,
        Method::GET,
        &format!("{ROOMS_PATH}/{room}/phone"),
        &secret,
        vec![],
    )
    .await;
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
async fn full_server_answers_503() {
    let r = router(1, 10);
    create(&r).await;
    let resp = call(&r, Method::POST, ROOMS_PATH, "", vec![]).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}
