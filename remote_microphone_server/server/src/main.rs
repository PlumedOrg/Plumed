//! `plumed-relay`: see the crate docs in lib.rs and remote_microphone_server/README.md.

use plumed_relay::{app, reap_forever, Config};
use std::net::SocketAddr;

fn env_or<T: std::str::FromStr>(name: &str, default: T) -> Result<T, String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => v.trim().parse().map_err(|_| format!("invalid {name}")),
        _ => Ok(default),
    }
}

fn config() -> Result<(SocketAddr, Config), String> {
    let bind = env_or("PLUMED_RELAY_BIND", SocketAddr::from(([0, 0, 0, 0], 5174)))?;
    let config = Config {
        max_rooms: env_or("PLUMED_RELAY_MAX_ROOMS", 500)?,
        rooms_per_ip_per_hour: env_or("PLUMED_RELAY_ROOMS_PER_IP_PER_HOUR", 60)?,
        trust_proxy: env_or("PLUMED_RELAY_TRUST_PROXY", false)?,
    };
    Ok((bind, config))
}

async fn shutdown_signal() {
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

/// `plumed-relay --healthcheck`: the container image has no curl, so the
/// binary checks itself (GET /healthz on the bound port of this host).
async fn healthcheck(bind: SocketAddr) -> Result<(), String> {
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

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let (bind, config) = match config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("plumed-relay: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    if std::env::args().nth(1).as_deref() == Some("--healthcheck") {
        return match healthcheck(bind).await {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("plumed-relay: {e}");
                std::process::ExitCode::FAILURE
            }
        };
    }
    let (router, state) = app(&config);
    tokio::spawn(reap_forever(state));
    let listener = match tokio::net::TcpListener::bind(bind).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("plumed-relay: cannot listen on {bind}: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    eprintln!("plumed-relay: listening on {bind}");
    let served = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await;
    match served {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("plumed-relay: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
