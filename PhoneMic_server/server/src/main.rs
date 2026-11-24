//! `plumed-relay`: see the crate docs in lib.rs and PhoneMic_server/README.md.

use plumed_relay::serve::{env_or, exit_code, healthcheck, healthcheck_requested, run};
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
    exit_code("plumed-relay", run_relay().await)
}

async fn run_relay() -> Result<(), String> {
    let (bind, config) = config()?;
    if healthcheck_requested() {
        return healthcheck(bind).await;
    }
    let (router, state) = app(&config);
    tokio::spawn(reap_forever(state));
    run("plumed-relay", bind, router).await
}
