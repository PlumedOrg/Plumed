//! The app release routes of the relay (see the crate docs), so the app's
//! updater works where GitHub is blocked. A relay cannot forge an update:
//! the app installs only what the updater key signed. Written with Claude Code.

use super::{forwarded, retry_after, stream, Inner, RETRY_AFTER_SECS};
use axum::body::Bytes;
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use futures_util::StreamExt;
use plumed_relay::serve::client_key;
use plumed_relay_protocol::releases::{ReleaseFile, LISTING_API_PATH};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How long the listing is reused: 12 GitHub API calls an hour at most, far
/// below the 60 an unauthenticated address may make.
const LISTING_TTL: Duration = Duration::from_secs(5 * 60);
/// Ten releases with their assets take well under this.
const LISTING_MAX_BYTES: usize = 4 * 1024 * 1024;
const LISTING_LABEL: &str = "release listing";

pub(crate) struct Listing {
    fetched: Instant,
    body: Bytes,
}

/// `GET /releases/latest/download/latest.json` and
/// `GET /releases/download/<tag>/<asset>`, streamed from github.com.
pub(crate) async fn serve(State(state): State<Arc<Inner>>, req: Request) -> Response {
    let file = match (req.uri().query(), ReleaseFile::parse(req.uri().path())) {
        (None, Some(file)) => file,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let url = state.github.url(&file.github_path());
    let ip = client_key(&req, state.trust_proxy);
    let forwarded = forwarded(req.headers());
    let label = format!("release {}", file.path());
    stream(state, ip, forwarded, label, url, |s| &s.github).await
}

/// `GET /releases/list`: the GitHub API listing of the 10 newest releases,
/// fetched at most once per [`LISTING_TTL`] (one fetch at a time).
pub(crate) async fn listing(State(state): State<Arc<Inner>>, req: Request) -> Response {
    if req.uri().query().is_some() {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(_slot) = state.per_client.enter(client_key(&req, state.trust_proxy)) else {
        return retry_after(StatusCode::SERVICE_UNAVAILABLE, RETRY_AFTER_SECS);
    };
    let mut cached = state.listing.lock().await;
    if let Some(listing) = cached.as_ref() {
        if listing.fetched.elapsed() < LISTING_TTL {
            return json(listing.body.clone());
        }
    }
    match tokio::time::timeout(super::HEADERS_TIMEOUT, fetch_listing(&state)).await {
        Ok(Ok(body)) => {
            *cached = Some(Listing {
                fetched: Instant::now(),
                body: body.clone(),
            });
            json(body)
        }
        Ok(Err(status)) => status.into_response(),
        Err(_) => StatusCode::GATEWAY_TIMEOUT.into_response(),
    }
}

async fn fetch_listing(state: &Inner) -> Result<Bytes, StatusCode> {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );
    let url = state.github_api.url(LISTING_API_PATH);
    let resp = state
        .fetch_following(LISTING_LABEL, &url, &headers, &state.github_api)
        .await?;
    if resp.status() != StatusCode::OK {
        let why = format!("upstream HTTP {}", resp.status());
        return Err(state.fail(LISTING_LABEL, &why));
    }
    let mut body = Vec::new();
    let mut chunks = resp.bytes_stream();
    while let Some(chunk) = chunks.next().await {
        let chunk = chunk.map_err(|e| state.fail(LISTING_LABEL, &e.to_string()))?;
        if body.len() + chunk.len() > LISTING_MAX_BYTES {
            return Err(state.fail(LISTING_LABEL, "listing too large"));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body.into())
}

fn json(body: Bytes) -> Response {
    ([(header::CONTENT_TYPE, "application/json")], body).into_response()
}
