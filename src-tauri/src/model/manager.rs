//! What the model commands do with the app (Plumed fork, written with Claude
//! Code): one download at a time, the active choice, and reloading the
//! engine when the model in use changes.

use super::catalog::{self, Variant};
use super::download::{self, DownloadError, Endpoints};
use super::memory;
use super::store::{self, Source};
use super::Model;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::watch;

const PROGRESS_EVENT: &str = "model-download-progress";
/// The installed or active model changed (download, switch, delete, damage).
const CHANGED_EVENT: &str = "model-status-changed";

/// One catalog variant as the UI lists it.
#[derive(Debug, Serialize)]
pub struct VariantStatus {
    pub id: &'static str,
    pub family: &'static str,
    pub name: &'static str,
    pub summary: &'static str,
    pub size: u64,
    pub available: bool,
    pub experimental: bool,
    /// Installed with this source (cheap check).
    pub installed: Option<Source>,
    /// Bytes already on disk (a download to resume).
    pub downloaded: u64,
}

#[derive(Debug, Serialize)]
pub struct ModelStatus {
    /// The variant in use, installed.
    pub active: Option<&'static str>,
    /// No download needed: the development fallback model is in use.
    pub dev: bool,
    /// The variant being downloaded.
    pub downloading: Option<&'static str>,
    /// The variant to preselect, lighter on a machine with little RAM.
    pub recommended: &'static str,
    pub variants: Vec<VariantStatus>,
}

/// The running download: its variant and how to cancel it.
static RUNNING: Mutex<Option<(&'static str, watch::Sender<bool>)>> = Mutex::new(None);

fn running() -> std::sync::MutexGuard<'static, Option<(&'static str, watch::Sender<bool>)>> {
    RUNNING.lock().unwrap_or_else(PoisonError::into_inner)
}

fn model(app: &AppHandle) -> Arc<Model> {
    app.state::<Arc<Model>>().inner().clone()
}

fn root(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    model(app).models_root().map_err(|e| e.to_string())
}

pub fn status(app: &AppHandle) -> ModelStatus {
    let model = model(app);
    let root = model.models_root().ok();
    let variants = catalog::VARIANTS
        .iter()
        .map(|v| variant_status(root.as_deref(), v))
        .collect();
    ModelStatus {
        active: model.active_variant().map(|v| v.id),
        dev: model.is_dev(),
        downloading: running().as_ref().map(|(id, _)| *id),
        recommended: catalog::recommended(memory::total_memory()).id,
        variants,
    }
}

fn variant_status(root: Option<&std::path::Path>, v: &'static Variant) -> VariantStatus {
    VariantStatus {
        id: v.id,
        family: v.family.name,
        name: v.name,
        summary: v.summary,
        size: v.total_size(),
        available: v.available,
        experimental: v.experimental,
        installed: root.and_then(|r| store::installed_source(r, v)),
        downloaded: root.map_or(0, |r| store::bytes_on_disk(r, v)),
    }
}

fn find(id: &str) -> Result<&'static Variant, String> {
    catalog::find(id).ok_or_else(|| format!("Unknown model {id}"))
}

/// Downloads `id`, then makes it the model in use.
pub async fn download(app: &AppHandle, id: &str) -> Result<ModelStatus, DownloadError> {
    let variant = catalog::find(id).ok_or(DownloadError::Unavailable)?;
    let root = root(app).map_err(|detail| DownloadError::Io { detail })?;
    fetch(app, &root, variant).await?;
    activate(app, &root, variant)
        .await
        .map_err(|detail| DownloadError::Io { detail })?;
    if let Err(e) = store::remove_stale(&root) {
        log::warn!("Could not remove old models: {e}");
    }
    Ok(status(app))
}

/// Downloads `variant` into `root`, one download at a time, with progress
/// events and cancellation.
async fn fetch(
    app: &AppHandle,
    root: &std::path::Path,
    variant: &'static Variant,
) -> Result<(), DownloadError> {
    let cancel = {
        let mut running = running();
        if running.is_some() {
            return Err(DownloadError::Busy);
        }
        let (sender, receiver) = watch::channel(false);
        *running = Some((variant.id, sender));
        receiver
    };
    let progress_app = app.clone();
    let result = download::download(root, variant, &Endpoints::public(), cancel, |p| {
        let _ = progress_app.emit(PROGRESS_EVENT, p);
    })
    .await;
    *running() = None;
    result.map(|_| ())
}

/// An auxiliary model (see `catalog::AUXILIARY`) as the UI shows it.
pub fn auxiliary_status(app: &AppHandle, variant: &'static Variant) -> VariantStatus {
    variant_status(root(app).ok().as_deref(), variant)
}

/// The folder of an installed auxiliary model.
pub fn auxiliary_dir(app: &AppHandle, variant: &'static Variant) -> Option<PathBuf> {
    let root = root(app).ok()?;
    store::installed_source(&root, variant).map(|_| store::variant_dir(&root, variant))
}

/// Downloads an auxiliary model; the speech model in use is unchanged.
pub async fn download_auxiliary(
    app: &AppHandle,
    variant: &'static Variant,
) -> Result<VariantStatus, DownloadError> {
    let root = root(app).map_err(|detail| DownloadError::Io { detail })?;
    fetch(app, &root, variant).await?;
    Ok(variant_status(Some(&root), variant))
}

pub fn cancel() {
    if let Some((_, sender)) = running().as_ref() {
        let _ = sender.send(true);
    }
}

/// Makes the installed `id` the model in use.
pub async fn set_active(app: &AppHandle, id: &str) -> Result<ModelStatus, String> {
    let variant = find(id)?;
    let root = root(app)?;
    if store::installed_source(&root, variant).is_none() {
        return Err(format!("{id} is not downloaded"));
    }
    activate(app, &root, variant).await?;
    Ok(status(app))
}

async fn activate(
    app: &AppHandle,
    root: &std::path::Path,
    variant: &Variant,
) -> Result<(), String> {
    store::set_active(root, variant).map_err(|e| e.to_string())?;
    reload_engine(app).await
}

/// Drops the loaded engine and loads the model now in use, if any.
async fn reload_engine(app: &AppHandle) -> Result<(), String> {
    let app = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        crate::audio::unload_engine(&app);
        let loaded = match model(&app).is_available() {
            true => crate::audio::preload_engine(&app).map_err(|e| e.to_string()),
            false => Ok(()),
        };
        let _ = app.emit(CHANGED_EVENT, ());
        loaded
    })
    .await;
    result.map_err(|e| e.to_string())?
}

/// Deletes the files of `id` (installed or partly downloaded).
pub async fn delete(app: &AppHandle, id: &str) -> Result<ModelStatus, String> {
    let variant = find(id)?;
    if running()
        .as_ref()
        .is_some_and(|(running, _)| *running == variant.id)
    {
        return Err("This model is being downloaded".to_owned());
    }
    let root = root(app)?;
    let was_active = model(app)
        .active_variant()
        .is_some_and(|a| a.id == variant.id);
    store::delete(&root, variant).map_err(|e| e.to_string())?;
    match was_active {
        true => reload_engine(app).await?,
        false => {
            let _ = app.emit(CHANGED_EVENT, ());
        }
    }
    Ok(status(app))
}

/// At launch, after the engine failed to load an installed model: a full
/// check, so a damaged model leads to the download dialog.
pub fn check_after_load_failure(app: &AppHandle) {
    let model = model(app);
    let (Some(variant), Ok(root)) = (model.active_variant(), model.models_root()) else {
        return;
    };
    match store::verify(&root, variant) {
        Ok(true) => log::warn!("The model {} is intact but did not load", variant.id),
        Ok(false) => {
            log::warn!("The model {} was damaged and is no longer used", variant.id);
            let _ = app.emit(CHANGED_EVENT, ());
        }
        Err(e) => log::warn!("Could not check the model {}: {e}", variant.id),
    }
}
