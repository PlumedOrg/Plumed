//! The Plumed model relay: serves the speech model files Plumed downloads by
//! streaming them from huggingface.co, for computers that reach plumed.fr but
//! not the hub (hospital networks). A Rust port of ParakeetWeb's
//! `signaling/hf-relay.js`, written with Claude Code.
//!
//! - `GET`/`HEAD /models/<owner>/<name>/<path>?revision=<commit>`: the file
//!   at that commit (scheme in `plumed_relay_protocol::models`).
//! - Nothing is buffered beyond the stream's own backpressure window and
//!   nothing touches the disk: the cost is bandwidth only.
//! - `Range`, `If-Range` and `If-None-Match` are forwarded and
//!   `Content-Length`, `Content-Range`, `Accept-Ranges` and `ETag` come back,
//!   so the app can resume a download.
//!
//! Never an open proxy: only the allowlisted `repo@revision` pairs are served
//! (anything else is a 404 without contacting the hub), every path segment
//! is a plain `[A-Za-z0-9._-]` token, redirects are followed by hand and each
//! hop must stay on https `huggingface.co`, `*.huggingface.co` or `*.hf.co`
//! (at most 5), and the optional token only goes to the hub itself, never to
//! its CDN. Concurrent streams are capped globally and per client IP. Client
//! IPs are never logged.

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use futures_util::StreamExt;
use plumed_relay::limiter::InFlight;
use plumed_relay::serve::{client_key, retry_after};
use plumed_relay_protocol::models::{allowed_hub_host, safe_path, safe_segment, MODELS_PATH};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

pub const DEFAULT_UPSTREAM: &str = "https://huggingface.co";
const MAX_REDIRECTS: usize = 5;
/// Connection plus response headers, per upstream request chain.
const HEADERS_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// A body that sends nothing for this long is cut (the app then resumes).
const READ_TIMEOUT: Duration = Duration::from_secs(60);
const RETRY_AFTER_SECS: u64 = 5;

/// Copied from the client request to the hub.
const FORWARDED_REQUEST: [HeaderName; 3] = [header::RANGE, header::IF_RANGE, header::IF_NONE_MATCH];
/// Copied from the hub's file response to the client.
const PASSTHROUGH: [HeaderName; 7] = [
    header::CONTENT_TYPE,
    header::CONTENT_LENGTH,
    header::CONTENT_RANGE,
    header::ACCEPT_RANGES,
    header::ETAG,
    header::LAST_MODIFIED,
    header::CONTENT_ENCODING,
];

/// The `repo@revision` pairs the relay serves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allowlist(HashMap<String, HashSet<String>>);

impl Allowlist {
    /// Parses comma-separated `owner/name@revision` entries. A malformed
    /// entry is an error, so a typo stops the boot instead of silently
    /// dropping a repository the app then fails to download.
    pub fn parse(raw: &str) -> Result<Self, String> {
        let mut repos: HashMap<String, HashSet<String>> = HashMap::new();
        for entry in raw.split(',').map(str::trim).filter(|e| !e.is_empty()) {
            let (repo, revision) = entry
                .split_once('@')
                .ok_or_else(|| format!("missing @revision in \"{entry}\""))?;
            if repo.split('/').count() != 2 || !safe_path(repo, 2) {
                return Err(format!("invalid repository in \"{entry}\""));
            }
            if !safe_segment(revision) {
                return Err(format!("invalid revision in \"{entry}\""));
            }
            repos
                .entry(repo.to_owned())
                .or_default()
                .insert(revision.to_owned());
        }
        match repos.is_empty() {
            true => Err("the allowlist is empty".to_owned()),
            false => Ok(Self(repos)),
        }
    }

    /// The repositories and pinned revisions the app downloads.
    pub fn app_defaults() -> Self {
        let raw: Vec<String> = plumed_relay_protocol::models::MODEL_REPOS
            .iter()
            .map(|r| format!("{}@{}", r.repo, r.revision))
            .collect();
        Self::parse(&raw.join(",")).unwrap_or(Self(HashMap::new()))
    }

    pub fn allows(&self, repo: &str, revision: &str) -> bool {
        self.0.get(repo).is_some_and(|r| r.contains(revision))
    }
}

pub struct Config {
    pub allowlist: Allowlist,
    /// Hugging Face token, sent to `upstream` only.
    pub token: Option<String>,
    pub upstream: String,
    /// Where a redirect may land besides `upstream` itself.
    pub redirect_allowed: fn(&reqwest::Url) -> bool,
    pub max_streams: usize,
    pub streams_per_ip: usize,
    /// Take the client IP from `X-Forwarded-For` (behind Caddy).
    pub trust_proxy: bool,
}

/// The hub's CDN hosts, https only.
pub fn hub_redirect(url: &reqwest::Url) -> bool {
    allowed_hub_host(url.scheme(), url.host_str().unwrap_or_default())
}

struct Inner {
    allowlist: Allowlist,
    token: Option<String>,
    upstream: reqwest::Url,
    redirect_allowed: fn(&reqwest::Url) -> bool,
    client: reqwest::Client,
    streams: Arc<Semaphore>,
    per_client: InFlight,
    trust_proxy: bool,
}

pub fn app(config: Config) -> Result<Router, String> {
    // reqwest is built without a default crypto provider (no C toolchain
    // needed for aws-lc); an already installed provider is fine.
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .user_agent("plumed-models")
        .build()
        .map_err(|e| e.to_string())?;
    let upstream = reqwest::Url::parse(config.upstream.trim_end_matches('/'))
        .map_err(|e| format!("invalid upstream: {e}"))?;
    let state = Arc::new(Inner {
        allowlist: config.allowlist,
        token: config.token.filter(|t| !t.trim().is_empty()),
        upstream,
        redirect_allowed: config.redirect_allowed,
        client,
        streams: Arc::new(Semaphore::new(config.max_streams)),
        per_client: InFlight::new(config.streams_per_ip),
        trust_proxy: config.trust_proxy,
    });
    Ok(Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route(&format!("{MODELS_PATH}/{{*file}}"), get(serve).head(serve))
        .layer(middleware::map_response(security_headers))
        .with_state(state))
}

async fn security_headers(mut resp: Response) -> Response {
    let h = resp.headers_mut();
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    h.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    resp
}

/// A requested file: repository, path in it and revision.
#[derive(Debug, PartialEq, Eq)]
pub struct Wanted {
    pub repo: String,
    pub file: String,
    pub revision: String,
}

/// Splits `/models/<owner>/<name>/<path>` and the `revision=<commit>` query.
/// `None` for anything not made only of safe segments (percent-encoded
/// characters included) or with another query.
pub fn parse_request(path: &str, query: Option<&str>) -> Option<Wanted> {
    let rest = path.strip_prefix(MODELS_PATH)?.strip_prefix('/')?;
    let mut parts = rest.splitn(3, '/');
    let (owner, name, file) = (parts.next()?, parts.next()?, parts.next()?);
    let revision = query?.strip_prefix("revision=")?;
    match safe_segment(owner) && safe_segment(name) && safe_path(file, 1) && safe_segment(revision)
    {
        true => Some(Wanted {
            repo: format!("{owner}/{name}"),
            file: file.to_owned(),
            revision: revision.to_owned(),
        }),
        false => None,
    }
}

async fn serve(State(state): State<Arc<Inner>>, req: Request) -> Response {
    let wanted = match parse_request(req.uri().path(), req.uri().query()) {
        Some(w) if state.allowlist.allows(&w.repo, &w.revision) => w,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let url = format!(
        "{}/{}/resolve/{}/{}",
        state.upstream.as_str().trim_end_matches('/'),
        wanted.repo,
        wanted.revision,
        wanted.file
    );
    if req.method() == Method::HEAD {
        return head(&state, &wanted, &url).await;
    }
    // Taken before any await: a `&Request` is not `Send`.
    let ip = client_key(&req, state.trust_proxy);
    let mut forwarded = HeaderMap::new();
    for name in FORWARDED_REQUEST {
        if let Some(value) = req.headers().get(&name) {
            forwarded.insert(name, value.clone());
        }
    }
    stream(state, ip, forwarded, &wanted, &url).await
}

fn same_origin(a: &reqwest::Url, b: &reqwest::Url) -> bool {
    a.scheme() == b.scheme()
        && a.host_str() == b.host_str()
        && a.port_or_known_default() == b.port_or_known_default()
}

impl Inner {
    fn request(
        &self,
        method: Method,
        url: &reqwest::Url,
        headers: &HeaderMap,
    ) -> reqwest::RequestBuilder {
        let builder = self
            .client
            .request(method, url.clone())
            .headers(headers.clone())
            .header(header::ACCEPT_ENCODING, "identity");
        match &self.token {
            Some(token) if same_origin(url, &self.upstream) => builder.bearer_auth(token),
            _ => builder,
        }
    }

    /// Follows redirects by hand, checking every hop. The final
    /// non-redirect answer, or the status to give the client.
    async fn fetch_following(
        &self,
        wanted: &Wanted,
        start: &str,
        headers: &HeaderMap,
    ) -> Result<reqwest::Response, StatusCode> {
        let mut url = reqwest::Url::parse(start).map_err(|_| StatusCode::BAD_GATEWAY)?;
        for _ in 0..=MAX_REDIRECTS {
            let resp = self
                .request(Method::GET, &url, headers)
                .send()
                .await
                .map_err(|e| self.fail(wanted, &e.to_string()))?;
            if !resp.status().is_redirection() || resp.status() == StatusCode::NOT_MODIFIED {
                return Ok(resp);
            }
            let next = resp
                .headers()
                .get(header::LOCATION)
                .and_then(|l| l.to_str().ok())
                .and_then(|l| url.join(l).ok())
                .ok_or_else(|| self.fail(wanted, "redirect without a valid Location"))?;
            if !same_origin(&next, &self.upstream) && !(self.redirect_allowed)(&next) {
                let host = next.host_str().unwrap_or_default().to_owned();
                return Err(self.fail(wanted, &format!("refusing a redirect to {host}")));
            }
            url = next;
        }
        Err(self.fail(wanted, "too many redirects"))
    }

    /// Logs an upstream failure (repository and file only, never the client).
    fn fail(&self, wanted: &Wanted, why: &str) -> StatusCode {
        eprintln!("plumed-models: {}/{}: {why}", wanted.repo, wanted.file);
        StatusCode::BAD_GATEWAY
    }
}

/// `HEAD` stops at the hub: its resolve answer already carries the real size
/// and ETag of a large file (`X-Linked-Size`, `X-Linked-Etag`), so a
/// presence probe never touches the CDN.
async fn head(state: &Inner, wanted: &Wanted, url: &str) -> Response {
    let Ok(url) = reqwest::Url::parse(url) else {
        return StatusCode::BAD_GATEWAY.into_response();
    };
    let sent = state.request(Method::HEAD, &url, &HeaderMap::new()).send();
    let resp = match tokio::time::timeout(HEADERS_TIMEOUT, sent).await {
        Ok(Ok(resp)) => resp,
        Ok(Err(e)) => return state.fail(wanted, &e.to_string()).into_response(),
        Err(_) => return StatusCode::GATEWAY_TIMEOUT.into_response(),
    };
    match resp.status().as_u16() {
        404 => return StatusCode::NOT_FOUND.into_response(),
        400.. => {
            return state
                .fail(wanted, &format!("HEAD HTTP {}", resp.status()))
                .into_response()
        }
        _ => {}
    }
    let pick = |linked: &str, plain: HeaderName| {
        resp.headers()
            .get(linked)
            .or_else(|| resp.headers().get(plain))
            .cloned()
    };
    let mut out = StatusCode::OK.into_response();
    let h = out.headers_mut();
    if let Some(size) = pick("x-linked-size", header::CONTENT_LENGTH) {
        h.insert(header::CONTENT_LENGTH, size);
    }
    if let Some(etag) = pick("x-linked-etag", header::ETAG) {
        h.insert(header::ETAG, etag);
    }
    h.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    out
}

async fn stream(
    state: Arc<Inner>,
    ip: std::net::IpAddr,
    forwarded: HeaderMap,
    wanted: &Wanted,
    url: &str,
) -> Response {
    let Some(slot) = state.per_client.enter(ip) else {
        return retry_after(StatusCode::SERVICE_UNAVAILABLE, RETRY_AFTER_SECS);
    };
    let Ok(permit) = state.streams.clone().try_acquire_owned() else {
        return retry_after(StatusCode::SERVICE_UNAVAILABLE, RETRY_AFTER_SECS);
    };
    let fetched = tokio::time::timeout(
        HEADERS_TIMEOUT,
        state.fetch_following(wanted, url, &forwarded),
    )
    .await;
    let upstream = match fetched {
        Ok(Ok(resp)) => resp,
        Ok(Err(status)) => return status.into_response(),
        Err(_) => return StatusCode::GATEWAY_TIMEOUT.into_response(),
    };
    let status = upstream.status();
    let has_body = match status.as_u16() {
        404 => return StatusCode::NOT_FOUND.into_response(),
        200 | 206 => true,
        304 | 416 => false,
        _ => {
            return state
                .fail(wanted, &format!("upstream HTTP {status}"))
                .into_response()
        }
    };
    let mut headers = HeaderMap::new();
    for name in PASSTHROUGH {
        if let Some(value) = upstream.headers().get(&name) {
            headers.insert(name, value.clone());
        }
    }
    if !has_body {
        headers.remove(header::CONTENT_LENGTH);
        return (status, headers).into_response();
    }
    // The slot and the permit live as long as the body is being sent.
    let body = upstream.bytes_stream().map(move |chunk| {
        let _held = (&slot, &permit);
        chunk.map_err(std::io::Error::other)
    });
    (status, headers, Body::from_stream(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_allowlist_takes_owner_name_at_revision_and_refuses_junk() {
        let list = Allowlist::parse(" a/b@abc , a/b@def,c/d@main ").expect("valid");
        assert!(list.allows("a/b", "abc") && list.allows("a/b", "def"));
        assert!(list.allows("c/d", "main"));
        assert!(!list.allows("a/b", "main") && !list.allows("x/y", "abc"));
        for bad in [
            "", "a/b", "a@x", "a/b/c@x", "../b@x", "a/b@", "a/b@x/y", "a/b@..",
        ] {
            assert!(Allowlist::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_defaults_are_the_app_repositories() {
        let list = Allowlist::app_defaults();
        for repo in plumed_relay_protocol::models::MODEL_REPOS {
            assert!(list.allows(repo.repo, repo.revision));
        }
    }

    #[test]
    fn only_safe_paths_with_a_revision_parse() {
        let wanted = parse_request("/models/a/b/int8/x.onnx", Some("revision=abc"));
        assert_eq!(
            wanted,
            Some(Wanted {
                repo: "a/b".into(),
                file: "int8/x.onnx".into(),
                revision: "abc".into()
            })
        );
        for (path, query) in [
            ("/models/a/b/x.onnx", None),
            ("/models/a/b/x.onnx", Some("rev=abc")),
            ("/models/a/b/x.onnx", Some("revision=abc&x=1")),
            ("/models/a/b", Some("revision=abc")),
            ("/models/a/b/../x", Some("revision=abc")),
            ("/models/a/b/%2e%2e/x", Some("revision=abc")),
            ("/models/a/b//x", Some("revision=abc")),
            ("/other/a/b/x", Some("revision=abc")),
        ] {
            assert_eq!(parse_request(path, query), None, "{path} {query:?}");
        }
    }
}
