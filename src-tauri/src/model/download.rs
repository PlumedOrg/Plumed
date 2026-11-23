//! Downloads one catalog variant into the model store (Plumed fork, written
//! with Claude Code). Each file goes to `<name>.partial`, resumed with an
//! HTTP Range request after an interruption, checked for its size and
//! SHA-256, then renamed into place; the `model.json` marker comes last.
//!
//! Hugging Face first (redirects only to its own https hosts), then the
//! Plumed relay (`PLUMED_MODEL_RELAY_URL`, default `https://models.plumed.fr`)
//! when the hub fails, with retries and backoff on each.

use super::catalog::{ModelFile, Variant};
use super::store::{self, Source};
use crate::utils::verified_file::{file_sha256, VerifiedFile, VerifyError};
use plumed_relay_protocol::models::{allowed_hub_host, hub_path, relay_path, HUB_URL};
use serde::Serialize;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::sync::watch;

/// No version, no system: nothing tells users apart.
const USER_AGENT: &str = "Plumed";
const DEFAULT_RELAY_URL: &str = "https://models.plumed.fr";
/// Set at build time to use another relay (a test deployment).
const RELAY_URL_OVERRIDE: Option<&str> = option_env!("PLUMED_MODEL_RELAY_URL");
/// Tries per source and file before moving on.
const ATTEMPTS: u32 = 3;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);
/// Free space kept beyond the download (logs, settings, the OS).
const SPACE_MARGIN: u64 = 50 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DownloadError {
    /// Neither source could be reached, or both kept failing.
    Network {
        detail: String,
    },
    DiskSpace {
        needed: u64,
        available: u64,
    },
    /// A file downloaded whole did not have its published hash.
    Checksum {
        file: String,
    },
    Cancelled,
    /// Another download is running.
    Busy,
    /// The variant is not in the catalog or cannot run yet.
    Unavailable,
    Io {
        detail: String,
    },
}

impl DownloadError {
    fn io(e: impl std::fmt::Display) -> Self {
        Self::Io {
            detail: e.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub variant: &'static str,
    pub downloaded: u64,
    pub total: u64,
    /// Bytes per second, smoothed.
    pub speed: f64,
    pub source: Source,
}

/// Where the files come from. Tests point both at a local server.
pub struct Endpoints {
    pub hub: String,
    pub relay: String,
    pub hub_redirect: fn(&reqwest::Url) -> bool,
    /// First retry delay, doubled at each retry.
    pub backoff: Duration,
}

impl Endpoints {
    pub fn public() -> Self {
        Self {
            hub: HUB_URL.to_owned(),
            relay: relay_base().to_owned(),
            hub_redirect: |url| allowed_hub_host(url.scheme(), url.host_str().unwrap_or("")),
            backoff: Duration::from_secs(1),
        }
    }

    fn url(&self, source: Source, variant: &Variant, file: &ModelFile) -> String {
        let repo = &variant.family.repo;
        match source {
            Source::HuggingFace => format!(
                "{}{}",
                self.hub.trim_end_matches('/'),
                hub_path(repo.repo, repo.revision, file.path)
            ),
            Source::Relay => format!(
                "{}{}",
                self.relay.trim_end_matches('/'),
                relay_path(repo, file.path)
            ),
        }
    }
}

/// The relay base url of this build (also used by `app_update`).
pub fn relay_base() -> &'static str {
    relay_url(RELAY_URL_OVERRIDE)
}

/// The relay base url: the build-time override unless it is empty.
fn relay_url(over: Option<&'static str>) -> &'static str {
    match over.map(str::trim) {
        Some(url) if !url.is_empty() => url,
        _ => DEFAULT_RELAY_URL,
    }
}

/// Free bytes on the disk holding `dir`, `None` when unknown.
#[cfg(unix)]
fn available_space(dir: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(dir.as_os_str().as_bytes()).ok()?;
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: `path` is a valid C string and `stats` a writable statvfs.
    let stats = match unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) } {
        0 => unsafe { stats.assume_init() },
        _ => return None,
    };
    #[allow(clippy::unnecessary_cast)] // the field types differ between unixes
    Some(stats.f_bavail as u64 * stats.f_frsize as u64)
}

#[cfg(windows)]
fn available_space(dir: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut free = 0u64;
    // SAFETY: `wide` is NUL-terminated, the other outputs may be null.
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (ok != 0).then_some(free)
}

#[cfg(not(any(unix, windows)))]
fn available_space(_dir: &Path) -> Option<u64> {
    None
}

fn check_space(root: &Path, variant: &Variant) -> Result<(), DownloadError> {
    let needed = variant.total_size() - store::bytes_on_disk(root, variant) + SPACE_MARGIN;
    match available_space(root) {
        Some(available) if available < needed => {
            Err(DownloadError::DiskSpace { needed, available })
        }
        _ => Ok(()),
    }
}

/// Smooths the speed and limits progress events to a few per second.
struct Meter {
    last: Instant,
    last_bytes: u64,
    speed: f64,
}

impl Meter {
    fn new(now: Instant, bytes: u64) -> Self {
        Self {
            last: now,
            last_bytes: bytes,
            speed: 0.0,
        }
    }

    /// The smoothed speed if an update is due at `now`.
    fn tick(&mut self, now: Instant, bytes: u64) -> Option<f64> {
        let elapsed = now.duration_since(self.last);
        if elapsed < PROGRESS_INTERVAL {
            return None;
        }
        let instant = bytes.saturating_sub(self.last_bytes) as f64 / elapsed.as_secs_f64();
        self.speed = match self.speed {
            s if s > 0.0 => 0.7 * s + 0.3 * instant,
            _ => instant,
        };
        self.last = now;
        self.last_bytes = bytes;
        Some(self.speed)
    }
}

/// What one request attempt ended with.
enum Failure {
    /// Worth another try (network, server error, bad hash of a whole file).
    Retry(DownloadError),
    /// Stop everything (cancelled, local disk error).
    Fatal(DownloadError),
}

struct Job<'a, F: FnMut(Progress)> {
    variant: &'static Variant,
    endpoints: &'a Endpoints,
    hub: reqwest::Client,
    relay: reqwest::Client,
    cancel: watch::Receiver<bool>,
    on_progress: F,
    meter: Meter,
    /// Bytes of finished files.
    done: u64,
    source: Source,
}

/// Downloads `variant` into `root` and marks it installed. Returns the
/// source the last file came from.
pub async fn download(
    root: &Path,
    variant: &'static Variant,
    endpoints: &Endpoints,
    cancel: watch::Receiver<bool>,
    on_progress: impl FnMut(Progress),
) -> Result<Source, DownloadError> {
    if !variant.available {
        return Err(DownloadError::Unavailable);
    }
    let dir = store::variant_dir(root, variant);
    std::fs::create_dir_all(&dir).map_err(DownloadError::io)?;
    check_space(root, variant)?;
    let network = |e: String| DownloadError::Network { detail: e };
    let mut job = Job {
        variant,
        endpoints,
        hub: crate::utils::http::streaming_client(
            &endpoints.hub,
            USER_AGENT,
            Some(endpoints.hub_redirect),
        )
        .map_err(network)?,
        relay: crate::utils::http::streaming_client(&endpoints.relay, USER_AGENT, None)
            .map_err(network)?,
        cancel,
        on_progress,
        meter: Meter::new(Instant::now(), 0),
        done: 0,
        source: Source::HuggingFace,
    };
    for file in variant.files {
        job.fetch_file(&dir, file).await?;
        job.done += file.size;
    }
    store::write_marker(root, variant, job.source).map_err(DownloadError::io)?;
    Ok(job.source)
}

impl<F: FnMut(Progress)> Job<'_, F> {
    /// Gets `file` into place, unless it already is (checked by hash).
    async fn fetch_file(&mut self, dir: &Path, file: &ModelFile) -> Result<(), DownloadError> {
        let target = dir.join(file.local_name);
        if target.is_file() {
            match file_sha256(&target) {
                Ok(sha) if sha == file.sha256 => return Ok(()),
                _ => std::fs::remove_file(&target).map_err(DownloadError::io)?,
            }
        }
        let partial = store::partial_path(dir, file.local_name);
        let mut last_error = DownloadError::Network {
            detail: "no source tried".to_owned(),
        };
        // The hub first, then the relay, staying on the relay once there.
        let sources: &[Source] = match self.source {
            Source::HuggingFace => &[Source::HuggingFace, Source::Relay],
            Source::Relay => &[Source::Relay],
        };
        for &source in sources {
            self.source = source;
            for attempt in 0..ATTEMPTS {
                if attempt > 0 {
                    self.pause(self.endpoints.backoff * 2u32.pow(attempt - 1))
                        .await?;
                }
                match self.attempt(source, file, &partial).await {
                    Ok(()) => return std::fs::rename(&partial, &target).map_err(DownloadError::io),
                    Err(Failure::Fatal(e)) => return Err(e),
                    Err(Failure::Retry(e)) => {
                        log::warn!("Model download of {} failed: {e:?}", file.path);
                        last_error = e;
                    }
                }
            }
        }
        Err(last_error)
    }

    /// Waits `delay` unless cancelled meanwhile.
    async fn pause(&mut self, delay: Duration) -> Result<(), DownloadError> {
        tokio::select! {
            _ = tokio::time::sleep(delay) => Ok(()),
            _ = self.cancel.wait_for(|c| *c) => Err(DownloadError::Cancelled),
        }
    }

    /// One request for the rest of `file`, appended to `partial`.
    async fn attempt(
        &mut self,
        source: Source,
        file: &ModelFile,
        partial: &Path,
    ) -> Result<(), Failure> {
        let fatal_io = |e: std::io::Error| Failure::Fatal(DownloadError::io(e));
        let mut out = VerifiedFile::resume(partial, file.size, file.sha256).map_err(fatal_io)?;
        if out.written() == file.size {
            // A previous run got every byte but failed the check: start over.
            out = VerifiedFile::create(partial, file.size, file.sha256).map_err(fatal_io)?;
        }
        let start = out.written();
        let client = match source {
            Source::HuggingFace => &self.hub,
            Source::Relay => &self.relay,
        };
        let mut request = client.get(self.endpoints.url(source, self.variant, file));
        if start > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={start}-"));
        }
        let mut response = self.until_cancelled(request.send()).await?;
        match (response.status().as_u16(), start) {
            (200, 0) => {}
            // The server ignored the range: take the whole file again.
            (200, _) => {
                out = VerifiedFile::create(partial, file.size, file.sha256).map_err(fatal_io)?
            }
            (206, _) if content_range_starts_at(&response, start) => {}
            (status, _) => {
                return Err(Failure::Retry(DownloadError::Network {
                    detail: format!("HTTP {status}"),
                }))
            }
        }
        while let Some(chunk) = self.until_cancelled(response.chunk()).await? {
            match out.write(&chunk) {
                Ok(()) => self.report(out.written()),
                Err(e) => return Err(self.write_failure(e, file, partial)),
            }
        }
        out.finish()
            .map_err(|e| self.write_failure(e, file, partial))
    }

    async fn until_cancelled<T>(
        &mut self,
        future: impl std::future::Future<Output = reqwest::Result<T>>,
    ) -> Result<T, Failure> {
        tokio::select! {
            result = future => result.map_err(|e| Failure::Retry(DownloadError::Network {
                detail: e.to_string(),
            })),
            _ = self.cancel.wait_for(|c| *c) => Err(Failure::Fatal(DownloadError::Cancelled)),
        }
    }

    /// A wrong hash or size discards the partial file (the next try starts
    /// over); a disk error stops the download.
    fn write_failure(&self, e: VerifyError, file: &ModelFile, partial: &Path) -> Failure {
        match e {
            VerifyError::Mismatch => {
                let _ = std::fs::remove_file(partial);
                Failure::Retry(DownloadError::Checksum {
                    file: file.path.to_owned(),
                })
            }
            VerifyError::Io(e) => Failure::Fatal(DownloadError::io(e)),
        }
    }

    fn report(&mut self, written_in_file: u64) {
        let downloaded = self.done + written_in_file;
        if let Some(speed) = self.meter.tick(Instant::now(), downloaded) {
            (self.on_progress)(Progress {
                variant: self.variant.id,
                downloaded,
                total: self.variant.total_size(),
                speed,
                source: self.source,
            });
        }
    }
}

fn content_range_starts_at(response: &reqwest::Response, start: u64) -> bool {
    response
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with(&format!("bytes {start}-")))
}

#[cfg(test)]
mod tests;
