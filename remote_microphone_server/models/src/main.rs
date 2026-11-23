//! `plumed-models`: see the crate docs in lib.rs and remote_microphone_server/README.md.

use plumed_models::{
    app, github_redirect, hub_redirect, Allowlist, Config, DEFAULT_GITHUB, DEFAULT_GITHUB_API,
    DEFAULT_UPSTREAM,
};
use plumed_relay::serve::{env_or, exit_code, healthcheck, healthcheck_requested, run};
use std::net::SocketAddr;

fn config() -> Result<(SocketAddr, Config), String> {
    let bind = env_or("PLUMED_MODELS_BIND", SocketAddr::from(([0, 0, 0, 0], 5175)))?;
    let allowlist = match std::env::var("PLUMED_MODELS_REPOS") {
        Ok(raw) if !raw.trim().is_empty() => Allowlist::parse(&raw)?,
        _ => Allowlist::app_defaults(),
    };
    let config = Config {
        allowlist,
        token: std::env::var("HF_TOKEN").ok(),
        upstream: DEFAULT_UPSTREAM.to_owned(),
        redirect_allowed: hub_redirect,
        github: DEFAULT_GITHUB.to_owned(),
        github_api: DEFAULT_GITHUB_API.to_owned(),
        github_redirect,
        max_streams: env_or("PLUMED_MODELS_MAX_STREAMS", 32)?,
        streams_per_ip: env_or("PLUMED_MODELS_STREAMS_PER_IP", 8)?,
        trust_proxy: env_or("PLUMED_MODELS_TRUST_PROXY", false)?,
    };
    Ok((bind, config))
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    exit_code("plumed-models", run_models().await)
}

async fn run_models() -> Result<(), String> {
    let (bind, config) = config()?;
    if healthcheck_requested() {
        return healthcheck(bind).await;
    }
    run("plumed-models", bind, app(config)?).await
}
