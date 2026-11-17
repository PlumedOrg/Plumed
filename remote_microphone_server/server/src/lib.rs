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
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{middleware, Router};
use limiter::{InFlight, Limiter};
use plumed_relay_protocol::{
    valid_inbox_tag, validate, Role, MAX_BODY, POLL_WAIT_SECS, ROOMS_PATH, SECRET_HEADER,
};
use rooms::{Inbox, PushError, Refused, Room, Rooms, Taken, QUEUE_BUDGET};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

/// A room nobody polled for this long is dropped (both ends poll every 25 s).
pub const ROOM_IDLE: Duration = Duration::from_secs(10 * 60);
/// Hard cap on a room's life, even in use: Plumed then opens a new one.
pub const ROOM_MAX_AGE: Duration = Duration::from_secs(24 * 3600);
const REAP_EVERY: Duration = Duration::from_secs(30);
/// Requests handled at once (a poll holds one for up to 25 s): past that the
/// relay answers 503 rather than piling up connections. Below the file
/// descriptor limit set in docker-compose.yml.
const MAX_IN_FLIGHT: usize = 2048;
/// Requests in flight per client, per room it may open: a desktop and a
/// phone behind the same NAT each hold a poll and sometimes send.
const IN_FLIGHT_PER_ROOM: usize = 4;
/// Memory for request bodies being read, before they are queued (and
/// counted in [`QUEUE_BUDGET`]). Kept in KiB permits.
const BODY_BUDGET_KIB: usize = 32 * 1024;
/// A client that has not sent its whole body by then is cut off.
const BODY_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Config {
    pub max_rooms: usize,
    /// Rooms open at once for one client IP (an IPv6 /64).
    pub rooms_per_ip: usize,
    pub rooms_per_ip_per_hour: u32,
    /// Take the client IP from the last `X-Forwarded-For` entry (behind Caddy),
    /// when the request comes from a loopback or private address.
    pub trust_proxy: bool,
}

struct Inner {
    rooms: Rooms,
    limiter: Limiter,
    trust_proxy: bool,
    in_flight: Arc<Semaphore>,
    per_client: InFlight,
    bodies: Arc<Semaphore>,
}

#[derive(Clone)]
pub struct AppState(Arc<Inner>);

pub fn app(config: &Config) -> (Router, AppState) {
    let state = AppState(Arc::new(Inner {
        rooms: Rooms::new(config.max_rooms, config.rooms_per_ip, QUEUE_BUDGET),
        limiter: Limiter::new(config.rooms_per_ip_per_hour),
        trust_proxy: config.trust_proxy,
        in_flight: Arc::new(Semaphore::new(MAX_IN_FLIGHT)),
        per_client: InFlight::new(config.rooms_per_ip * IN_FLIGHT_PER_ROOM),
        bodies: Arc::new(Semaphore::new(BODY_BUDGET_KIB)),
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
        .layer(middleware::from_fn_with_state(
            state.clone(),
            limit_in_flight,
        ))
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

async fn limit_in_flight(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let client = client_key(&req, state.0.trust_proxy);
    let Some(_slot) = state.0.per_client.enter(client) else {
        return retry_after(StatusCode::TOO_MANY_REQUESTS, 5);
    };
    match state.0.in_flight.clone().try_acquire_owned() {
        // Both held until the response is ready
        Ok(_permit) => next.run(req).await,
        Err(_) => retry_after(StatusCode::SERVICE_UNAVAILABLE, 5),
    }
}

/// A reverse proxy on this machine or in the container network.
fn is_local(ip: IpAddr) -> bool {
    match ip.to_canonical() {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private(),
        IpAddr::V6(v6) => v6.is_loopback() || (v6.segments()[0] & 0xfe00) == 0xfc00,
    }
}

/// The client, as counted by the rate limits (see [`limiter::bucket`]).
/// `X-Forwarded-For` is only believed when a local proxy sent it: exposed
/// directly by mistake, the relay would otherwise let anyone pick their IP.
fn client_key(req: &Request, trust_proxy: bool) -> IpAddr {
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
    let peer = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip());
    let ip = match peer {
        Some(p) if trust_proxy && is_local(p) => forwarded().or(peer),
        _ => peer,
    };
    // Unknown: every such client shares one bucket
    limiter::bucket(ip.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)))
}

fn retry_after(status: StatusCode, secs: u64) -> Response {
    (status, [(header::RETRY_AFTER, secs.to_string())]).into_response()
}

/// Only for the desktop app, which sends no `Origin`: a browser always does
/// on a POST, so a web page cannot make its visitors open rooms here.
async fn create_room(State(state): State<AppState>, req: Request) -> Response {
    if req.headers().contains_key(header::ORIGIN) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let client = client_key(&req, state.0.trust_proxy);
    if let Err(secs) = state.0.limiter.allow(client, Instant::now()) {
        return retry_after(StatusCode::TOO_MANY_REQUESTS, secs);
    }
    match state.0.rooms.create(client) {
        Ok(created) => (
            StatusCode::CREATED,
            [(header::CONTENT_TYPE, "application/json")],
            format!(
                r#"{{"room":"{}","desktop":"{}","phone":"{}"}}"#,
                created.id, created.desktop_secret, created.phone_secret
            ),
        )
            .into_response(),
        Err(Refused::ServerFull) => retry_after(StatusCode::SERVICE_UNAVAILABLE, 60),
        Err(Refused::TooManyForClient) => retry_after(StatusCode::TOO_MANY_REQUESTS, 60),
    }
}

/// The room, if the request carries the secret of `role`. Unknown room and
/// wrong secret both answer 404.
fn open_room(
    state: &AppState,
    id: &str,
    role: Role,
    headers: &HeaderMap,
) -> Result<Arc<Room>, StatusCode> {
    let secret = headers
        .get(SECRET_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    state
        .0
        .rooms
        .open(id, role, secret)
        .ok_or(StatusCode::NOT_FOUND)
}

fn push_error(e: PushError) -> Response {
    match e {
        // The reader is not polling (asleep, offline): slow down
        PushError::Full => retry_after(StatusCode::TOO_MANY_REQUESTS, 1),
        PushError::TooManyPhones => retry_after(StatusCode::TOO_MANY_REQUESTS, 30),
    }
}

/// The body of `req`, read under [`BODY_BUDGET_KIB`] and [`BODY_TIMEOUT`]:
/// many slow or large uploads at once must not exhaust the relay's memory.
async fn read_body(state: &AppState, req: Request) -> Result<axum::body::Bytes, StatusCode> {
    let declared = req
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok());
    let size = match declared {
        Some(n) if n > MAX_BODY => return Err(StatusCode::PAYLOAD_TOO_LARGE),
        Some(n) => n,
        // Chunked: assume the worst
        None => MAX_BODY,
    };
    let kib = u32::try_from(size.div_ceil(1024)).unwrap_or(u32::MAX);
    let Ok(_reserved) = state.0.bodies.clone().try_acquire_many_owned(kib) else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };
    match tokio::time::timeout(BODY_TIMEOUT, axum::body::to_bytes(req.into_body(), size)).await {
        Ok(Ok(body)) => Ok(body),
        Ok(Err(_)) => Err(StatusCode::PAYLOAD_TOO_LARGE),
        Err(_) => Err(StatusCode::REQUEST_TIMEOUT),
    }
}

/// Appends the body to `inbox`: only the other role may write to it. The
/// secret is checked before the body is read.
async fn send(state: AppState, id: &str, inbox: Inbox<'_>, req: Request) -> Response {
    let room = match open_room(&state, id, inbox.owner().peer(), req.headers()) {
        Ok(room) => room,
        Err(status) => return status.into_response(),
    };
    let body = match read_body(&state, req).await {
        Ok(body) => body,
        Err(StatusCode::SERVICE_UNAVAILABLE) => {
            return retry_after(StatusCode::SERVICE_UNAVAILABLE, 1)
        }
        Err(status) => return status.into_response(),
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
        Err(status) => return status.into_response(),
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
        Err(status) => status.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(peer: &str, xff: Option<&str>) -> Request {
        let mut builder = Request::builder();
        if let Some(xff) = xff {
            builder = builder.header("x-forwarded-for", xff);
        }
        let mut req = builder.body(axum::body::Body::empty()).unwrap();
        let peer: SocketAddr = peer.parse().unwrap();
        req.extensions_mut().insert(ConnectInfo(peer));
        req
    }

    #[test]
    fn forwarded_for_is_only_believed_from_a_local_proxy() {
        let ip = |s: &str| s.parse::<IpAddr>().unwrap();
        // Caddy through the docker bridge
        let via_proxy = request("172.17.0.1:4000", Some("6.6.6.6, 1.2.3.4"));
        assert_eq!(client_key(&via_proxy, true), ip("1.2.3.4"));
        // Exposed directly: a client cannot pick its own IP
        let spoofed = request("9.9.9.9:4000", Some("1.2.3.4"));
        assert_eq!(client_key(&spoofed, true), ip("9.9.9.9"));
        // Not behind a proxy at all
        let local = request("127.0.0.1:4000", Some("1.2.3.4"));
        assert_eq!(client_key(&local, false), ip("127.0.0.1"));
        // A local proxy that sent no header
        assert_eq!(client_key(&request("[::1]:4000", None), true), ip("::"));
    }
}
