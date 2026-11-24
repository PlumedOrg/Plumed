//! Plumed fork: OpenAI-compatible transcription API (`POST
//! /v1/audio/transcriptions`), off by default (Settings > System > Advanced).
//! Runs inside the app while the setting is on, or headless with `plumed
//! serve`. Listens on 127.0.0.1 only and wants the bearer key from the
//! settings. Transcribes like a dictation (dictionary, formatting rules,
//! grammar, text templates) but writes nothing to the history nor the local
//! stats; only the opt-in usage statistics count API calls, apart.

mod server;

use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::sync::Arc;
use std::time::Instant;

use log::{error, info};
use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tokio::sync::oneshot;

use server::ApiContext;

#[derive(Default)]
pub struct LocalApiState {
    token: Arc<RwLock<String>>,
    running: Mutex<Option<Running>>,
}

struct Running {
    port: u16,
    shutdown: oneshot::Sender<()>,
}

#[derive(Serialize)]
pub struct LocalApiStatus {
    pub enabled: bool,
    pub port: u16,
    /// The port the in-app server listens on, `None` when it is stopped.
    pub running_port: Option<u16>,
    pub token: String,
}

fn new_token() -> Result<String, String> {
    let mut bytes = [0u8; 24];
    ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut bytes)
        .map_err(|_| "Could not generate an API key".to_string())?;
    Ok(format!(
        "plumed-{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}

/// The saved key, generated and saved first if there is none yet.
fn ensure_token(app: &AppHandle) -> Result<String, String> {
    let mut settings = crate::settings::load_settings(app);
    if settings.local_api_token.is_empty() {
        settings.local_api_token = new_token()?;
        crate::settings::save_settings(app, &settings)?;
    }
    Ok(settings.local_api_token)
}

pub fn regenerate_token(app: &AppHandle) -> Result<String, String> {
    let mut settings = crate::settings::load_settings(app);
    settings.local_api_token = new_token()?;
    crate::settings::save_settings(app, &settings)?;
    *app.state::<LocalApiState>().token.write() = settings.local_api_token.clone();
    Ok(settings.local_api_token)
}

pub fn status(app: &AppHandle) -> LocalApiStatus {
    let settings = crate::settings::load_settings(app);
    LocalApiStatus {
        enabled: settings.local_api_enabled,
        port: settings.local_api_port,
        running_port: app
            .state::<LocalApiState>()
            .running
            .lock()
            .as_ref()
            .map(|r| r.port),
        token: settings.local_api_token,
    }
}

/// Binds 127.0.0.1 only: other computers can never reach the API.
fn bind(port: u16) -> Result<TcpListener, String> {
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))
        .map_err(|e| format!("Cannot listen on 127.0.0.1:{port}: {e}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("Cannot listen on 127.0.0.1:{port}: {e}"))?;
    Ok(listener)
}

fn context(app: &AppHandle, token: Arc<RwLock<String>>) -> ApiContext {
    let app = app.clone();
    ApiContext {
        token,
        transcribe: Arc::new(move |samples: &[f32], rate: u32| {
            let started = Instant::now();
            let text = crate::audio::pipeline::transcribe_samples_chunked(&app, samples, rate)?;
            let audio_secs = samples.len() as f64 / rate.max(1) as f64;
            info!(
                "Local API: transcribed {audio_secs:.1} s of audio in {:.1} s",
                started.elapsed().as_secs_f64()
            );
            crate::telemetry::record_transcription(
                &app,
                crate::telemetry::Source::Api,
                audio_secs,
                &text,
                started.elapsed(),
            );
            Ok(text)
        }),
        busy: Arc::new(tokio::sync::Mutex::new(())),
    }
}

async fn serve(
    listener: TcpListener,
    ctx: ApiContext,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::from_std(listener)?;
    axum::serve(listener, server::router(ctx))
        .with_graceful_shutdown(shutdown)
        .await
}

/// Starts the in-app server on the saved port (restarting it if it runs).
fn start(app: &AppHandle) -> Result<u16, String> {
    stop(app);
    let port = crate::settings::load_settings(app).local_api_port;
    let token = ensure_token(app)?;
    let listener = bind(port)?;
    let state = app.state::<LocalApiState>();
    *state.token.write() = token;
    let ctx = context(app, state.token.clone());
    let (shutdown, stopped) = oneshot::channel::<()>();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = serve(listener, ctx, async {
            let _ = stopped.await;
        })
        .await
        {
            error!("Local API stopped: {e}");
        }
    });
    *state.running.lock() = Some(Running { port, shutdown });
    info!("Local API listening on http://127.0.0.1:{port}");
    Ok(port)
}

fn stop(app: &AppHandle) {
    if let Some(running) = app.state::<LocalApiState>().running.lock().take() {
        let _ = running.shutdown.send(());
        info!("Local API on port {} stopped", running.port);
    }
}

/// Starts or stops the in-app server to match the settings. On a start
/// failure (port taken), turns the setting back off so the toggle tells the
/// truth, and returns the reason.
pub fn apply_settings(app: &AppHandle) -> Result<(), String> {
    let settings = crate::settings::load_settings(app);
    if !settings.local_api_enabled {
        stop(app);
        return Ok(());
    }
    start(app).map(|_| ()).inspect_err(|_| {
        let mut settings = crate::settings::load_settings(app);
        settings.local_api_enabled = false;
        let _ = crate::settings::save_settings(app, &settings);
    })
}

/// `plumed serve`: no window, blocks until Ctrl-C. Refuses to run unless the
/// advanced setting is on, so the CLI cannot open a port the user never
/// allowed.
pub fn serve_headless(app: &AppHandle, port: Option<u16>) -> Result<(), String> {
    let settings = crate::settings::load_settings(app);
    if !settings.local_api_enabled {
        return Err(
            "The local transcription API is turned off. Turn on Settings > System > \
             Advanced > Local transcription API in Plumed first."
                .to_string(),
        );
    }
    let port = port.unwrap_or(settings.local_api_port);
    let token = Arc::new(RwLock::new(ensure_token(app)?));
    let listener = bind(port)?;
    // Load the model now: a missing model fails here, not on the first call.
    crate::audio::preload_engine(app).map_err(|e| e.to_string())?;
    eprintln!(
        "Plumed local API listening on http://127.0.0.1:{port}/v1 (Ctrl-C to stop).\n\
         Use the API key shown in Settings > System > Advanced."
    );
    let ctx = context(app, token);
    tauri::async_runtime::block_on(serve(listener, ctx, async {
        let _ = tokio::signal::ctrl_c().await;
    }))
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_random_and_long() {
        let (a, b) = (new_token().unwrap(), new_token().unwrap());
        assert_ne!(a, b);
        assert!(a.starts_with("plumed-"));
        assert_eq!(a.len(), "plumed-".len() + 48);
    }

    #[test]
    fn binds_loopback_only() {
        let listener = bind(0).unwrap();
        assert!(listener.local_addr().unwrap().ip().is_loopback());
    }
}
