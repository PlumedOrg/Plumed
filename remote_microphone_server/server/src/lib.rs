//! The Plumed remote microphone relay: serves the phone page and forwards
//! end-to-end encrypted frames between the phone and the Plumed desktop app.
//! The wire format is in the `plumed-relay-protocol` crate.
//!
//! Privacy: rooms live in RAM only, nothing is logged per request (no IP, no
//! room id), and the relay cannot read the audio (AES-GCM keys never leave the
//! two devices).

mod assets;
pub mod limiter;
pub mod rooms;

use axum::extract::{ConnectInfo, DefaultBodyLimit, Path, Request, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{middleware, Router};
use limiter::Limiter;
use plumed_relay_protocol::{
    valid_inbox_tag, validate, Role, MAX_BODY, POLL_WAIT_SECS, ROOMS_PATH, SECRET_HEADER,
};
use rooms::{Inbox, PushError, Room, Rooms, Taken};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A room nobody polled for this long is dropped (both ends poll every 25 s).
pub const ROOM_IDLE: Duration = Duration::from_secs(10 * 60);
/// Hard cap on a room's life, even in use: Plumed then opens a new one.
pub const ROOM_MAX_AGE: Duration = Duration::from_secs(24 * 3600);
const REAP_EVERY: Duration = Duration::from_secs(30);

pub struct Config {
    pub max_rooms: usize,
    pub rooms_per_ip_per_hour: u32,
    /// Take the client IP from the last `X-Forwarded-For` entry (behind Caddy).
    pub trust_proxy: bool,
}

struct Inner {
    rooms: Rooms,
    limiter: Limiter,
    trust_proxy: bool,
}

#[derive(Clone)]
pub struct AppState(Arc<Inner>);

pub fn app(config: &Config) -> (Router, AppState) {
    let state = AppState(Arc::new(Inner {
        rooms: Rooms::new(config.max_rooms),
        limiter: Limiter::new(config.rooms_per_ip_per_hour),
        trust_proxy: config.trust_proxy,
    }));
    let router = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route(ROOMS_PATH, post(create_room))
        .route(&format!("{ROOMS_PATH}/{{id}}"), delete(close_room))
        .route(
            &format!("{ROOMS_PATH}/{{id}}/desktop"),
            get(poll_desktop).post(send_desktop),
        )
        .route(
            &format!("{ROOMS_PATH}/{{id}}/phone/{{tag}}"),
            get(poll_phone).post(send_phone),
        )
        .merge(assets::router())
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .layer(middleware::map_response(security_headers))
        .with_state(state.clone());
    (router, state)
}

/// Drops idle rooms and old rate-limit windows, forever.
pub async fn reap_forever(state: AppState) {
    let mut tick = tokio::time::interval(REAP_EVERY);
    loop {
        tick.tick().await;
        let now = Instant::now();
        state.0.rooms.reap(now, ROOM_IDLE, ROOM_MAX_AGE);
        state.0.limiter.prune(now);
    }
}

const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; \
    connect-src 'self'; img-src 'self' data:; base-uri 'none'; form-action 'none'; \
    frame-ancestors 'none'";

async fn security_headers(mut resp: Response) -> Response {
    let h = resp.headers_mut();
    let set = |h: &mut HeaderMap, name: HeaderName, value: &'static str| {
        h.insert(name, HeaderValue::from_static(value));
    };
    set(h, header::CONTENT_SECURITY_POLICY, CSP);
    set(h, header::REFERRER_POLICY, "no-referrer");
    set(h, header::X_CONTENT_TYPE_OPTIONS, "nosniff");
    set(h, header::CACHE_CONTROL, "no-store");
    set(h, header::STRICT_TRANSPORT_SECURITY, "max-age=31536000");
    set(
        h,
        HeaderName::from_static("permissions-policy"),
        "microphone=(self), camera=(), geolocation=()",
    );
    set(
        h,
        HeaderName::from_static("cross-origin-opener-policy"),
        "same-origin",
    );
    resp
}

fn client_ip(req: &Request, trust_proxy: bool) -> IpAddr {
    let forwarded = || {
        req.headers()
            .get("x-forwarded-for")?
            .to_str()
            .ok()?
            .rsplit(',')
            .next()?
            .trim()
            .parse()
            .ok()
    };
    let peer = || {
        req.extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0.ip())
    };
    match trust_proxy {
        true => forwarded(),
        false => peer(),
    }
    // Unknown: every such client shares one bucket
    .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}

fn retry_after(status: StatusCode, secs: u64) -> Response {
    (status, [(header::RETRY_AFTER, secs.to_string())]).into_response()
}

async fn create_room(State(state): State<AppState>, req: Request) -> Response {
    let ip = client_ip(&req, state.0.trust_proxy);
    if let Err(secs) = state.0.limiter.allow(ip, Instant::now()) {
        return retry_after(StatusCode::TOO_MANY_REQUESTS, secs);
    }
    match state.0.rooms.create() {
        Ok(created) => (
            StatusCode::CREATED,
            [(header::CONTENT_TYPE, "application/json")],
            format!(
                r#"{{"room":"{}","desktop":"{}","phone":"{}"}}"#,
                created.id, created.desktop_secret, created.phone_secret
            ),
        )
            .into_response(),
        Err(rooms::ServerFull) => retry_after(StatusCode::SERVICE_UNAVAILABLE, 60),
    }
}

/// The room, if the request carries the secret of `role`. Unknown room and
/// wrong secret both answer 404.
fn open_room(
    state: &AppState,
    id: &str,
    role: Role,
    headers: &HeaderMap,
) -> Result<Arc<Room>, Response> {
    let secret = headers
        .get(SECRET_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    state
        .0
        .rooms
        .open(id, role, secret)
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())
}

fn push_error(e: PushError) -> Response {
    match e {
        // The reader is not polling (asleep, offline): slow down
        PushError::Full => retry_after(StatusCode::TOO_MANY_REQUESTS, 1),
        PushError::TooManyPhones => retry_after(StatusCode::TOO_MANY_REQUESTS, 30),
    }
}

/// Appends the body to `inbox`: only the other role may write to it. The
/// secret is checked before the body is read.
async fn send(state: AppState, id: &str, inbox: Inbox<'_>, req: Request) -> Response {
    let room = match open_room(&state, id, inbox.owner().peer(), req.headers()) {
        Ok(room) => room,
        Err(resp) => return resp,
    };
    let body = match axum::body::to_bytes(req.into_body(), MAX_BODY).await {
        Ok(body) => body,
        Err(_) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
    };
    if validate(&body).is_err() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    match room.push(inbox, &body) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => push_error(e),
    }
}

/// Empties `inbox`: only its owner may read it.
async fn poll(state: AppState, id: &str, inbox: Inbox<'_>, headers: &HeaderMap) -> Response {
    let room = match open_room(&state, id, inbox.owner(), headers) {
        Ok(room) => room,
        Err(resp) => return resp,
    };
    match room.take(inbox, Duration::from_secs(POLL_WAIT_SECS)).await {
        Ok(Taken::Frames(bytes)) => {
            ([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response()
        }
        Ok(Taken::Empty) => StatusCode::NO_CONTENT.into_response(),
        Ok(Taken::Closed) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => push_error(e),
    }
}

async fn send_desktop(
    State(state): State<AppState>,
    Path(id): Path<String>,
    req: Request,
) -> Response {
    send(state, &id, Inbox::Desktop, req).await
}

async fn poll_desktop(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    poll(state, &id, Inbox::Desktop, &headers).await
}

async fn send_phone(
    State(state): State<AppState>,
    Path((id, tag)): Path<(String, String)>,
    req: Request,
) -> Response {
    match valid_inbox_tag(&tag) {
        true => send(state, &id, Inbox::Phone(&tag), req).await,
        false => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn poll_phone(
    State(state): State<AppState>,
    Path((id, tag)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    match valid_inbox_tag(&tag) {
        true => poll(state, &id, Inbox::Phone(&tag), &headers).await,
        false => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Only the desktop may close its room.
async fn close_room(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    match open_room(&state, &id, Role::Desktop, &headers) {
        Ok(_) => {
            state.0.rooms.close(&id);
            StatusCode::NO_CONTENT.into_response()
        }
        Err(resp) => resp,
    }
}
