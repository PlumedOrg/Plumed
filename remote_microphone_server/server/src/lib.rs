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

use axum::body::Bytes;
use axum::extract::{ConnectInfo, DefaultBodyLimit, Path, Request, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{middleware, Router};
use limiter::Limiter;
use plumed_relay_protocol::{validate, Role, MAX_BODY, POLL_WAIT_SECS, ROOMS_PATH, SECRET_HEADER};
use rooms::{Room, Rooms, Taken};
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
            &format!("{ROOMS_PATH}/{{id}}/{{role}}"),
            get(poll).post(send),
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
        Ok((id, secret)) => (
            StatusCode::CREATED,
            [(header::CONTENT_TYPE, "application/json")],
            format!(r#"{{"room":"{id}","secret":"{secret}"}}"#),
        )
            .into_response(),
        Err(rooms::ServerFull) => retry_after(StatusCode::SERVICE_UNAVAILABLE, 60),
    }
}

/// The room for this request. Unknown room and wrong secret both answer 404.
fn open_room(state: &AppState, id: &str, headers: &HeaderMap) -> Result<Arc<Room>, StatusCode> {
    let secret = headers
        .get(SECRET_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    state.0.rooms.open(id, secret).ok_or(StatusCode::NOT_FOUND)
}

fn open_mailbox(
    state: &AppState,
    id: &str,
    role: &str,
    headers: &HeaderMap,
) -> Result<(Arc<Room>, Role), StatusCode> {
    let role = Role::parse(role).ok_or(StatusCode::NOT_FOUND)?;
    Ok((open_room(state, id, headers)?, role))
}

async fn send(
    State(state): State<AppState>,
    Path((id, role)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let (room, role) = match open_mailbox(&state, &id, &role, &headers) {
        Ok(found) => found,
        Err(status) => return status.into_response(),
    };
    if validate(&body).is_err() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    match room.push(role.peer(), &body) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        // The other end is not polling (asleep, offline): slow down
        Err(rooms::MailboxFull) => retry_after(StatusCode::TOO_MANY_REQUESTS, 1),
    }
}

async fn poll(
    State(state): State<AppState>,
    Path((id, role)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let (room, role) = match open_mailbox(&state, &id, &role, &headers) {
        Ok(found) => found,
        Err(status) => return status.into_response(),
    };
    match room.take(role, Duration::from_secs(POLL_WAIT_SECS)).await {
        Taken::Frames(bytes) => {
            ([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response()
        }
        Taken::Empty => StatusCode::NO_CONTENT.into_response(),
        Taken::Closed => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn close_room(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> StatusCode {
    match open_room(&state, &id, &headers) {
        Ok(_) => {
            state.0.rooms.close(&id);
            StatusCode::NO_CONTENT
        }
        Err(status) => status,
    }
}
