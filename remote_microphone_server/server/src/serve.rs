//! Serving helpers shared by the two binaries of this workspace, the PhoneMic
//! relay (`plumed-relay`) and the model relay (`plumed-models`): environment
//! settings, client IP behind the reverse proxy, shutdown and self check.

use axum::extract::{ConnectInfo, Request};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// `name` parsed from the environment, `default` when unset or empty.
pub fn env_or<T: std::str::FromStr>(name: &str, default: T) -> Result<T, String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => v.trim().parse().map_err(|_| format!("invalid {name}")),
        _ => Ok(default),
    }
}

/// A reverse proxy on this machine or in the container network.
pub fn is_local(ip: IpAddr) -> bool {
    match ip.to_canonical() {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private(),
        IpAddr::V6(v6) => v6.is_loopback() || (v6.segments()[0] & 0xfe00) == 0xfc00,
    }
}

/// The client, as counted by the rate limits (see [`crate::limiter::bucket`]).
/// `X-Forwarded-For` is only believed when a local proxy sent it: exposed
/// directly by mistake, the relay would otherwise let anyone pick their IP.
pub fn client_key(req: &Request, trust_proxy: bool) -> IpAddr {
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
    crate::limiter::bucket(ip.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)))
}

pub fn retry_after(status: StatusCode, secs: u64) -> Response {
    (status, [(header::RETRY_AFTER, secs.to_string())]).into_response()
}

pub async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut term =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(term) => term,
                Err(_) => return std::future::pending().await,
            };
        tokio::select! {
            _ = ctrl_c => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    let _ = ctrl_c.await;
}

/// `--healthcheck`: the container image has no curl, so the
/// binary checks itself (GET /healthz on the bound port of this host).
pub async fn healthcheck(bind: SocketAddr) -> Result<(), String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let addr = SocketAddr::from(([127, 0, 0, 1], bind.port()));
    let check = async {
        let mut stream = tokio::net::TcpStream::connect(addr).await?;
        stream
            .write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await?;
        let mut answer = Vec::new();
        stream.read_to_end(&mut answer).await?;
        Ok::<_, std::io::Error>(answer)
    };
    let answer = tokio::time::timeout(std::time::Duration::from_secs(5), check)
        .await
        .map_err(|_| "timeout".to_owned())?
        .map_err(|e| e.to_string())?;
    match answer.starts_with(b"HTTP/1.1 200") {
        true => Ok(()),
        false => Err("unhealthy".to_owned()),
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
