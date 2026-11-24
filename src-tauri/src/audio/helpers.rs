use anyhow::{Context, Result};

use hound::{WavSpec, WavWriter};
use log::warn;
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{
    calculate_cutoff, Async, FixedAsync, Resampler, SincInterpolationParameters,
    SincInterpolationType, WindowFunction,
};
use std::fs::{File, OpenOptions};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use tauri::Manager;
use unicode_normalization::UnicodeNormalization;

/// Capture buffer requested from cpal, in frames. The backend-default size
/// can silently drop a large share of capture periods, heard as crackling,
/// robotic audio.
const CAPTURE_BUFFER_FRAMES: u32 = 4096;

/// Build an input stream asking for the fixed capture buffer first, falling
/// back to the backend default when the device rejects it. `build` is called
/// with each candidate config.
pub fn build_input_with_buffer_fallback<S>(
    base: &cpal::StreamConfig,
    mut build: impl FnMut(&cpal::StreamConfig) -> std::result::Result<S, cpal::BuildStreamError>,
) -> std::result::Result<S, cpal::BuildStreamError> {
    let mut fixed = base.clone();
    fixed.buffer_size = cpal::BufferSize::Fixed(CAPTURE_BUFFER_FRAMES);
    build(&fixed).or_else(|e| {
        log::debug!(
            "Fixed capture buffer ({} frames) rejected: {}, falling back to default",
            CAPTURE_BUFFER_FRAMES,
            e
        );
        build(base)
    })
}

pub fn ensure_recordings_dir(app: &tauri::AppHandle) -> Result<PathBuf> {
    recordings_dir(app.path().runtime_dir().ok(), || {
        app.path()
            .app_cache_dir()
            .context("Failed to resolve app cache dir")
    })
}

/// `plumed_recordings`, in a folder private to this user so that no other
/// account can create it first (as it could in the shared `/tmp`). On Linux
/// the session runtime dir (`$XDG_RUNTIME_DIR`: RAM, 0700, emptied at logout)
/// when the session has a usable one, otherwise (a variable kept from another
/// account by `su`, and on Windows and macOS, where Tauri has no runtime dir)
/// the app cache dir.
fn recordings_dir(
    runtime_dir: Option<PathBuf>,
    cache_dir: impl FnOnce() -> Result<PathBuf>,
) -> Result<PathBuf> {
    if let Some(runtime_dir) = runtime_dir {
        match private_recordings_dir_in(&runtime_dir) {
            Ok(dir) => return Ok(dir),
            Err(e) => warn!("Runtime dir unusable for recordings, using the cache dir: {e:#}"),
        }
    }
    private_recordings_dir_in(&cache_dir()?)
}

fn private_recordings_dir_in(parent: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create {}", parent.display()))?;
    // Plumed fork: own folder, never shared with an installed Murmure.
    let recordings = parent.join("plumed_recordings");
    ensure_private_dir(&recordings)?;
    Ok(recordings)
}

/// Creates `dir` readable by this user only. Its parent is per user, but the
/// checks stay as defense in depth: a symlink planted to one of our folders
/// (whose files `cleanup_recordings` would then delete, and where the
/// dictation audio would land) or a folder owned by someone else is refused
/// rather than used.
#[cfg(unix)]
fn ensure_private_dir(dir: &Path) -> Result<()> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};
    if let Err(e) = std::fs::DirBuilder::new().mode(0o700).create(dir) {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(e).context("Failed to create recordings dir");
        }
    }
    let meta = std::fs::symlink_metadata(dir).context("Failed to read recordings dir")?;
    // SAFETY: geteuid has no preconditions and cannot fail.
    let euid = unsafe { libc::geteuid() };
    if !meta.file_type().is_dir() || meta.uid() != euid {
        anyhow::bail!(
            "{} is not a folder owned by this user, refusing to use it",
            dir.display()
        );
    }
    crate::utils::private_dir::restrict_to_owner(dir)
        .context("Failed to set recordings dir permissions")?;
    Ok(())
}

#[cfg(not(unix))]
fn ensure_private_dir(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir).context("Failed to create recordings dir")
}

pub fn create_owner_only_file(path: &Path) -> Result<File> {
    let mut opts = OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(path)
        .with_context(|| format!("Failed to create {}", path.display()))
}

pub fn generate_unique_wav_name() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("plumed-{}-{}.wav", ts, seq)
}

/// Recordings kept on disk when `keep_recordings` is enabled, rolling like
/// the transcription history (`MAX_HISTORY_ENTRIES`) so the audio matching a
/// history entry is still available without growing the folder forever.
const KEPT_RECORDINGS: usize = 5;

pub fn cleanup_recordings(app: &tauri::AppHandle) -> Result<()> {
    let recordings_dir = ensure_recordings_dir(app)?;

    if crate::settings::load_settings(app).keep_recordings {
        let mut recordings: Vec<(std::time::SystemTime, PathBuf)> =
            std::fs::read_dir(&recordings_dir)
                .context("Failed to read recordings directory")?
                .flatten()
                .filter(|entry| entry.path().is_file())
                .filter_map(|entry| {
                    let modified = entry.metadata().ok()?.modified().ok()?;
                    Some((modified, entry.path()))
                })
                .collect();
        recordings.sort_by(|a, b| b.0.cmp(&a.0));
        for (_, path) in recordings.into_iter().skip(KEPT_RECORDINGS) {
            if let Err(e) = std::fs::remove_file(&path) {
                warn!("Failed to delete old recording {}: {}", path.display(), e);
            }
        }
        log::info!(
            "Last {} recordings kept for debugging in {}",
            KEPT_RECORDINGS,
            recordings_dir.display()
        );
        return Ok(());
    }

    if !recordings_dir.exists() {
        return Ok(());
    }

    let entries =
        std::fs::read_dir(&recordings_dir).context("Failed to read recordings directory")?;

    for entry in entries.flatten() {
        if entry.path().is_file() {
            if let Err(e) = std::fs::remove_file(entry.path()) {
                warn!("Failed to delete {}: {}", entry.path().display(), e);
            }
        }
    }
    log::info!("Temporary audio files successfully cleaned up");

    Ok(())
}

pub(crate) fn read_wav_mono_native(wav_path: &Path) -> Result<(Vec<f32>, u32)> {
    decode_wav_mono(std::io::BufReader::new(std::fs::File::open(wav_path)?))
}

/// Decodes 16-bit integer PCM WAV from any reader (a file or uploaded bytes)
/// into mono f32 samples at their native rate.
pub(crate) fn decode_wav_mono<R: std::io::Read>(source: R) -> Result<(Vec<f32>, u32)> {
    let mut reader = hound::WavReader::new(source)?;
    let spec = reader.spec();

    if spec.bits_per_sample != 16 {
        return Err(anyhow::anyhow!(
            "Expected 16 bits per sample, found {}",
            spec.bits_per_sample
        ));
    }

    if spec.sample_format != hound::SampleFormat::Int {
        return Err(anyhow::anyhow!(
            "Expected Int sample format, found {:?}",
            spec.sample_format
        ));
    }

    let raw_i16: Result<Vec<i16>, _> = reader.samples::<i16>().collect();
    let mut raw_i16 = raw_i16?;

    if spec.channels > 1 {
        let ch = spec.channels as usize;
        let mut mono: Vec<i16> = Vec::with_capacity(raw_i16.len() / ch);
        for frame in raw_i16.chunks_exact(ch) {
            let sum: i32 = frame.iter().map(|&s| s as i32).sum();
            let avg = (sum / ch as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            mono.push(avg);
        }
        raw_i16 = mono;
    }

    let samples_f32: Vec<f32> = raw_i16
        .into_iter()
        .map(|s| s as f32 / i16::MAX as f32)
        .collect();

    Ok((samples_f32, spec.sample_rate))
}

pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

pub fn resample(input: &[f32], src_hz: usize, dst_hz: usize) -> Vec<f32> {
    if input.is_empty() || src_hz == 0 || dst_hz == 0 {
        return Vec::new();
    }
    if src_hz == dst_hz {
        return input.to_vec();
    }

    resample_inner(input, src_hz, dst_hz).unwrap_or_else(|e| {
        warn!("Resampling failed ({src_hz}->{dst_hz}): {e}");
        Vec::new()
    })
}

fn resample_inner(input: &[f32], src_hz: usize, dst_hz: usize) -> Result<Vec<f32>> {
    let ratio = dst_hz as f64 / src_hz as f64;
    let sinc_len = 128;
    let window = WindowFunction::BlackmanHarris2;
    let params = SincInterpolationParameters {
        sinc_len,
        f_cutoff: calculate_cutoff(sinc_len, window),
        oversampling_factor: 256,
        interpolation: SincInterpolationType::Linear,
        window,
    };

    let mut resampler = Async::<f32>::new_sinc(ratio, 1.0, &params, 1024, 1, FixedAsync::Input)?;

    let mut output = vec![0.0f32; resampler.process_all_needed_output_len(input.len())];
    let in_adapter = InterleavedSlice::new(input, 1, input.len())?;
    let out_capacity = output.len();
    let mut out_adapter = InterleavedSlice::new_mut(&mut output, 1, out_capacity)?;

    let (_, nbr_out) =
        resampler.process_all_into_buffer(&in_adapter, &mut out_adapter, input.len(), None)?;
    output.truncate(nbr_out);
    Ok(output)
}

pub fn create_wav_writer(
    path: &Path,
    config: &cpal::SupportedStreamConfig,
) -> Result<WavWriter<BufWriter<File>>> {
    let file = create_owner_only_file(path).context("Failed to create WAV file")?;
    let writer = BufWriter::new(file);
    let spec = WavSpec {
        channels: 1,
        sample_rate: config.sample_rate(),
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    WavWriter::new(writer, spec).context("Failed to create WAV writer")
}

/// Lowercase, accent-free, punctuation-free words joined by single spaces, to
/// compare transcribed words: "  Écoute, MURMÙRE!  " -> "ecoute murmure".
pub(crate) fn normalize_text(text: &str) -> String {
    text.to_lowercase()
        .nfd()
        // NFD decomposes é into e + \u{0301}; filter out the combining marks
        .filter(|c| !('\u{0300}'..='\u{036F}').contains(c))
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn sine(freq_hz: f32, sample_rate: usize, duration_s: f32) -> Vec<f32> {
        let n = (sample_rate as f32 * duration_s) as usize;
        (0..n)
            .map(|i| (2.0 * PI * freq_hz * i as f32 / sample_rate as f32).sin())
            .collect()
    }

    fn linear_reference(input: &[f32], src_hz: usize, dst_hz: usize) -> Vec<f32> {
        let ratio = dst_hz as f64 / src_hz as f64;
        let out_len = ((input.len() as f64) * ratio).ceil() as usize;
        let last_idx = input.len().saturating_sub(1);
        (0..out_len)
            .map(|i| {
                let t = i as f64 / ratio;
                let idx = t.floor() as usize;
                let frac = (t - idx as f64) as f32;
                let a = input[idx.min(last_idx)];
                let b = input[(idx + 1).min(last_idx)];
                a + (b - a) * frac
            })
            .collect()
    }

    #[cfg(unix)]
    fn scratch_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("plumed-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        dir
    }

    #[cfg(unix)]
    #[test]
    fn private_dir_is_created_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let scratch = scratch_dir();
        let dir = scratch.join("recordings");
        ensure_private_dir(&dir).unwrap();
        ensure_private_dir(&dir).unwrap();
        let mode = std::fs::metadata(&dir).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
        std::fs::remove_dir_all(scratch).unwrap();
    }

    /// Another user of a shared `/tmp` planted the folder as a symlink to one
    /// of ours: `cleanup_recordings` would delete its files.
    #[cfg(unix)]
    #[test]
    fn private_dir_refuses_a_planted_symlink() {
        use std::os::unix::fs::PermissionsExt;
        let scratch = scratch_dir();
        let victim = scratch.join("documents");
        std::fs::create_dir(&victim).unwrap();
        std::fs::set_permissions(&victim, std::fs::Permissions::from_mode(0o755)).unwrap();
        let planted = scratch.join("recordings");
        std::os::unix::fs::symlink(&victim, &planted).unwrap();
        assert!(ensure_private_dir(&planted).is_err());
        let mode = std::fs::metadata(&victim).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
        std::fs::remove_dir_all(scratch).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn recordings_go_to_the_runtime_dir() {
        let runtime = scratch_dir();
        let dir = recordings_dir(Some(runtime.clone()), || {
            anyhow::bail!("the cache dir must not be resolved")
        });
        assert_eq!(dir.unwrap(), runtime.join("plumed_recordings"));
        std::fs::remove_dir_all(runtime).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn recordings_go_to_the_cache_dir_without_a_runtime_dir() {
        let cache = scratch_dir();
        let dir = recordings_dir(None, || Ok(cache.clone()));
        assert_eq!(dir.unwrap(), cache.join("plumed_recordings"));
        std::fs::remove_dir_all(cache).unwrap();
    }

    /// `su` keeps the other account's `$XDG_RUNTIME_DIR`, which this user
    /// cannot write to: recording must still work.
    #[cfg(unix)]
    #[test]
    fn recordings_go_to_the_cache_dir_when_the_runtime_dir_is_unusable() {
        let scratch = scratch_dir();
        let runtime = scratch.join("not-a-folder");
        std::fs::write(&runtime, "").unwrap();
        let cache = scratch.join("cache");
        let dir = recordings_dir(Some(runtime), || Ok(cache.clone()));
        assert_eq!(dir.unwrap(), cache.join("plumed_recordings"));
        std::fs::remove_dir_all(scratch).unwrap();
    }

    #[test]
    fn should_return_empty_vec_when_input_is_empty() {
        assert_eq!(resample(&[], 48000, 16000), Vec::<f32>::new());
    }

    #[test]
    fn should_return_empty_vec_when_a_sample_rate_is_zero() {
        let signal = sine(1000.0, 48000, 0.1);
        assert_eq!(resample(&signal, 0, 16000), Vec::<f32>::new());
        assert_eq!(resample(&signal, 48000, 0), Vec::<f32>::new());
    }

    #[test]
    fn should_return_input_unchanged_when_src_equals_dst() {
        let signal = sine(1000.0, 16000, 0.1);
        assert_eq!(resample(&signal, 16000, 16000), signal);
    }

    #[test]
    fn should_produce_expected_length_without_nan_when_downsampling_48k_to_16k() {
        let signal = sine(1000.0, 48000, 1.0);
        let out = resample(&signal, 48000, 16000);

        let expected = signal.len() * 16000 / 48000;
        let block = 1024;
        assert!((out.len() as i64 - expected as i64).unsigned_abs() as usize <= block);
        assert!(out.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn should_attenuate_above_nyquist_component_when_downsampling() {
        // 12 kHz at 48 kHz is above the 16 kHz Nyquist (8 kHz). Linear interpolation
        // folds it back into the audible band; rubato's sinc filter removes it.
        let signal = sine(12000.0, 48000, 1.0);

        let rubato_out = resample(&signal, 48000, 16000);
        let linear_out = linear_reference(&signal, 48000, 16000);

        assert!(rubato_out.iter().all(|s| s.is_finite()));
        assert!(rms(&rubato_out) < rms(&linear_out) * 0.2);
    }

    #[test]
    fn should_not_panic_and_double_length_when_upsampling_8k_to_16k() {
        let signal = sine(1000.0, 8000, 0.5);
        let out = resample(&signal, 8000, 16000);

        let expected = signal.len() * 2;
        let block = 2048;
        assert!((out.len() as i64 - expected as i64).unsigned_abs() as usize <= block);
        assert!(out.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn should_produce_coherent_length_when_ratio_is_fractional_44100_to_16000() {
        let signal = sine(1000.0, 44100, 1.0);
        let out = resample(&signal, 44100, 16000);

        let expected = (signal.len() as f64 * 16000.0 / 44100.0) as usize;
        let block = 2048;
        assert!((out.len() as i64 - expected as i64).unsigned_abs() as usize <= block);
        assert!(out.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn generate_unique_wav_name_differs_on_rapid_calls() {
        let names: Vec<String> = (0..100).map(|_| generate_unique_wav_name()).collect();
        let unique: std::collections::HashSet<&String> = names.iter().collect();
        assert_eq!(unique.len(), names.len());
        assert!(names
            .iter()
            .all(|n| n.starts_with("plumed-") && n.ends_with(".wav")));
    }

    #[test]
    fn normalize_text_folds_case_accents_punctuation_and_spaces() {
        assert_eq!(normalize_text("MURMURE"), "murmure");
        assert_eq!(normalize_text("écoute café"), "ecoute cafe");
        assert_eq!(normalize_text("\"murmure.\""), "murmure");
        assert_eq!(normalize_text("  bonjour   murmure  "), "bonjour murmure");
        assert_eq!(normalize_text("  Écoute, MURMÙRE!  "), "ecoute murmure");
    }
}
