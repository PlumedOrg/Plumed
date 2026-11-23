use super::catalog::Variant;
use super::store;
use anyhow::{Context, Result};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Plumed fork: the speech model is downloaded on first launch (see
/// `download.rs`), not bundled. Development builds fall back to the model
/// `scripts/fetch-ultimed-model.sh` puts in `resources/`, so a dev checkout
/// works without going through the download.
#[cfg(debug_assertions)]
const DEV_MODEL_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../resources/parakeet-tdt-0.6b-v3-int8"
);

pub struct Model {
    app_handle: AppHandle,
}

impl Model {
    pub fn new(app_handle: AppHandle) -> Result<Self> {
        Ok(Self { app_handle })
    }

    /// `models/` of the app data folder: outside the install folder, so a
    /// downloaded model survives updates.
    pub fn models_root(&self) -> Result<PathBuf> {
        Ok(self.app_handle.path().app_data_dir()?.join("models"))
    }

    /// The chosen variant if it is installed (cheap check: marker and sizes).
    pub fn active_variant(&self) -> Option<&'static Variant> {
        let root = self.models_root().ok()?;
        store::active(&root).filter(|v| store::installed_source(&root, v).is_some())
    }

    pub fn get_model_path(&self) -> Result<PathBuf> {
        let root = self.models_root()?;
        match self.active_variant() {
            Some(variant) => Ok(store::variant_dir(&root, variant)),
            None => dev_model().context("No speech model downloaded"),
        }
    }

    pub fn get_tokenizer_path(&self) -> Option<PathBuf> {
        crate::utils::resources::resolve_resource_path(&self.app_handle, "tokenizer.json")
    }

    pub fn is_available(&self) -> bool {
        self.get_model_path().is_ok()
    }

    /// The model named in the usage statistics: a catalog variant id, `dev`
    /// for the development fallback, `none` before any download.
    pub fn telemetry_id(&self) -> &'static str {
        match (self.active_variant(), dev_model().is_some()) {
            (Some(variant), _) => variant.id,
            (None, true) => "dev",
            (None, false) => "none",
        }
    }

    /// Whether the development fallback model is in use.
    pub fn is_dev(&self) -> bool {
        self.active_variant().is_none() && dev_model().is_some()
    }
}

#[cfg(debug_assertions)]
fn dev_model() -> Option<PathBuf> {
    let dir = PathBuf::from(DEV_MODEL_DIR);
    dir.join("vocab.txt").is_file().then_some(dir)
}

#[cfg(not(debug_assertions))]
fn dev_model() -> Option<PathBuf> {
    None
}
