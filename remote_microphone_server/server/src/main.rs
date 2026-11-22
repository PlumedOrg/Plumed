//! `plumed-relay`: see the crate docs in lib.rs and remote_microphone_server/README.md.

use plumed_relay::serve::{env_or, healthcheck, shutdown_signal};
use plumed_relay::{app, reap_forever, Config};
use std::net::SocketAddr;

fn config() -> Result<(SocketAddr, Config), String> {
    let bind = env_or("PLUMED_RELAY_BIND", SocketAddr::from(([0, 0, 0, 0], 5174)))?;
    let config = Config {
        max_rooms: env_or("PLUMED_RELAY_MAX_ROOMS", 500)?,
        rooms_per_ip: env_or("PLUMED_RELAY_ROOMS_PER_IP", 100)?,
        rooms_per_ip_per_hour: env_or("PLUMED_RELAY_ROOMS_PER_IP_PER_HOUR", 300)?,
        trust_proxy: env_or("PLUMED_RELAY_TRUST_PROXY", false)?,
    };
    Ok((bind, config))
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
