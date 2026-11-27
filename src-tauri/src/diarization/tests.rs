//! Parity tests of the Sortformer port against transformers, through the
//! fixtures parakeet_web generated with `scripts/gen-sortformer-fixtures.py`
//! (copied to `tests/fixtures/diarization/`).

use super::diarization::{SortformerStream, FRAME_SEC, STEP_FILE_FP32};
use super::segments::{probs_to_segments, SegmentOptions};
use super::sortformer::ChunkLoop;
use super::types::{SortformerConfig, SortformerRunner};
use base64::Engine;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/diarization")
        .join(name)
}

fn f64s(b64: &serde_json::Value) -> Vec<f64> {
    base64::engine::general_purpose::STANDARD
        .decode(b64.as_str().expect("base64 string"))
        .expect("base64")
        .as_chunks::<8>()
        .0
        .iter()
        .map(|b| f64::from_le_bytes(*b))
        .collect()
}

/// Same closed form as `fake_step` in the generator:
/// x(8t+j, s) = 3 sin(0.7 (s+1) e[t, s mod H] + 0.02 j + s + 0.5 m[s]),
/// m[s] = sum_r cos(r) e[r, (s+1) mod H]. The m term mixes every input row by
/// position (cache rows included), which makes the cache observable. Only each
/// row's argmax speaker stays high (the others get -10).
struct FakeStep {
    hidden: usize,
}

impl SortformerRunner for FakeStep {
    fn run_embed(&mut self, _: &[f32], _: usize) -> anyhow::Result<Vec<f32>> {
        unreachable!("the cache fixture starts from embeds")
    }

    fn run_step(&mut self, input: &[f64], num_rows: usize) -> anyhow::Result<Vec<f64>> {
        let h = self.hidden;
        let mut m = [0f64; 8];
        for r in 0..num_rows {
            for (s, v) in m.iter_mut().enumerate() {
                *v += (r as f64).cos() * input[r * h + (s + 1) % h];
            }
        }
        let mut out = vec![0f64; num_rows * 64];
        for t in 0..num_rows {
            for j in 0..8 {
                let x: Vec<f64> = (0..8)
                    .map(|s| {
                        3.0 * (0.7 * (s + 1) as f64 * input[t * h + s % h]
                            + 0.02 * j as f64
                            + s as f64
                            + 0.5 * m[s])
                            .sin()
                    })
                    .collect();
                let top = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                for s in 0..8 {
                    out[(t * 8 + j) * 8 + s] = if x[s] == top { x[s] } else { x[s] - 10.0 };
                }
            }
        }
        Ok(out)
    }
}

struct CacheFixture {
    config: SortformerConfig,
    embeds: Vec<f64>,
    silence: Vec<f64>,
    probs: Vec<f64>,
    num_embeds: usize,
}

fn cache_fixture() -> CacheFixture {
    let raw: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture("sortformer-cache.json")).expect("fixture"))
            .expect("json");
    CacheFixture {
        config: serde_json::from_value(raw["config"].clone()).expect("config"),
        embeds: f64s(&raw["embeds"]),
        silence: f64s(&raw["silence"]),
        probs: f64s(&raw["probs"]),
        num_embeds: raw["numEmbeds"].as_u64().expect("numEmbeds") as usize,
    }
}

/// Feeds the embeds in pieces of `piece` rows, running the loop after each.
fn run_cache_fixture(fx: &CacheFixture, num_frames: usize, piece: usize) -> Vec<f32> {
    let hidden = fx.config.hidden_size;
    let mut chunks = ChunkLoop::new(fx.config.clone(), fx.silence.clone());
    let mut step = FakeStep { hidden };
    let mut got = Vec::new();
    for rows in fx.embeds.chunks(piece * hidden) {
        chunks.push_embed_rows(rows.chunks_exact(hidden).map(<[f64]>::to_vec));
        got.extend(chunks.run(&mut step, None).expect("step"));
    }
    got.extend(chunks.run(&mut step, Some(num_frames)).expect("step"));
    assert_eq!(
        chunks.chunks_done(),
        fx.num_embeds.div_ceil(fx.config.offline.chunk_length)
    );
    got
}

fn max_diff(got: &[f32], reference: &[f64]) -> f64 {
    got.iter()
        .zip(reference)
        .map(|(&g, &r)| (g as f64 - r).abs())
        .fold(0.0, f64::max)
}

// The chunk loop + speaker cache + FIFO + compression with a tiny cache (24
// rows for 8 speakers, a FIFO of 6, an update period of 17), so compression
// runs at almost every one of the 10 chunks. Any slip in which frames the
// cache keeps changes the next step's input: a wrong row is O(0.1).
#[test]
fn chunk_loop_reproduces_transformers_through_repeated_compression() {
    let fx = cache_fixture();
    let num_frames = fx.num_embeds * fx.config.subsampling_factor;
    // all at once, and trickled in (the incremental path must be exact too)
    for piece in [fx.num_embeds, 7, 1] {
        let got = run_cache_fixture(&fx, num_frames, piece);
        assert_eq!(got.len(), fx.probs.len());
        let diff = max_diff(&got, &fx.probs);
        assert!(diff < 1e-6, "piece {piece}: max |dp| {diff}");
    }
}

#[test]
fn chunk_loop_truncates_to_the_clip_when_the_last_embed_is_padding() {
    let fx = cache_fixture();
    let num_frames = fx.num_embeds * fx.config.subsampling_factor - 5;
    let got = run_cache_fixture(&fx, num_frames, fx.num_embeds);
    assert_eq!(got.len(), num_frames * 8);
    assert!(max_diff(&got, &fx.probs) < 1e-6);
}

fn read_wav(path: &Path) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).expect("wav");
    assert_eq!(reader.spec().sample_rate, 16000);
    reader
        .samples::<i16>()
        .map(|s| s.expect("sample") as f32 / 32768.0)
        .collect()
}

const X3_GAP: usize = 8000;

/// two-speakers.wav three times over with 8000-sample gaps (the clip the
/// x3 reference was made from), and that reference, `[frames][8]`.
fn two_speakers_x3() -> (Vec<f32>, Vec<f64>, f64) {
    let pcm = read_wav(&fixture("two-speakers.wav"));
    let mut clip = Vec::new();
    for k in 0..3 {
        clip.extend_from_slice(&pcm);
        if k < 2 {
            clip.extend(std::iter::repeat_n(0f32, X3_GAP));
        }
    }
    let raw = std::fs::read(fixture("sortformer-two-speakers-x3.bin")).expect("fixture");
    let reference = raw
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes(*b) as f64 / 65535.0)
        .collect();
    let period = (pcm.len() + X3_GAP) as f64 / 16000.0;
    (clip, reference, period)
}

fn model_dir() -> PathBuf {
    PathBuf::from(std::env::var("PLUMED_DIAR_MODEL_DIR").expect("PLUMED_DIAR_MODEL_DIR"))
}

fn agreement(probs: &[f32], reference: &[f64]) -> usize {
    probs
        .iter()
        .zip(reference)
        .filter(|(&p, &r)| (p > 0.5) == (r > 0.5))
        .count()
}

/// End to end with the REAL model (fp32): mel front end, embed.onnx, chunk
/// loop and speaker cache, step.onnx, against the probabilities transformers'
/// own forward gave for two-speakers.wav three times over (48 s, 8000-sample
/// gaps), so the second chunk runs against a cache compressed from real,
/// overlapping speech. Needs a local copy of
/// Olicorne/Nemotron-3-Diarization-web-onnx in PLUMED_DIAR_MODEL_DIR:
/// `PLUMED_DIAR_MODEL_DIR=... cargo test --release --lib diarization -- --ignored --nocapture`
#[test]
#[ignore]
fn real_model_matches_transformers_on_two_speakers_x3() {
    let mut model = super::diarization::DiarizationModel::load(&model_dir(), STEP_FILE_FP32, None)
        .expect("model");
    let (clip, reference, period) = two_speakers_x3();

    // whole clip at once, then trickled in 0.37 s pieces like live audio
    for piece in [clip.len(), 5920] {
        let start = std::time::Instant::now();
        let mut stream: SortformerStream = model.stream();
        let mut probs = Vec::new();
        for samples in clip.chunks(piece) {
            probs.extend(stream.push(&mut model.runner, samples).expect("push"));
        }
        probs.extend(stream.finish(&mut model.runner).expect("finish"));
        println!(
            "piece {piece}: {:?} for {:.1} s of audio",
            start.elapsed(),
            clip.len() as f64 / 16000.0
        );
        assert_eq!(probs.len(), reference.len());
        // parakeet_web measured max |dp| 8e-6 on onnxruntime-node fp32, of
        // which the fixture's uint16 rounding alone is up to 7.6e-6
        let diff = max_diff(&probs, &reference);
        let agree = agreement(&probs, &reference);
        assert!(diff < 1e-4, "piece {piece}: max |dp| {diff}");
        assert_eq!(agree, reference.len(), "piece {piece}: decisions differ");

        let segments = probs_to_segments(&probs, 8, FRAME_SEC, SegmentOptions::default());
        let mut speakers: Vec<usize> = segments.iter().map(|s| s.speaker).collect();
        speakers.sort_unstable();
        speakers.dedup();
        assert_eq!(speakers, vec![0, 1]);
        // the repeats agree on who speaks when: the cache carried identities
        let at = |t: f64| {
            segments
                .iter()
                .find(|s| s.start <= t && t < s.end)
                .map(|s| s.speaker)
        };
        let mut t = 0.5;
        while t < period - 0.5 {
            if let Some(first) = at(t) {
                assert_eq!(at(t + 2.0 * period), Some(first), "speaker at {t:.1} s");
            }
            t += 0.5;
        }
    }
}

/// int8 step graph vs the fp32 reference: speed and decision agreement, to
/// back the choice of STEP_FILE_INT8 (prints, asserts only a loose floor).
#[test]
#[ignore]
fn int8_step_agrees_with_the_reference() {
    let mut model = super::diarization::DiarizationModel::load(
        &model_dir(),
        super::diarization::STEP_FILE_INT8,
        None,
    )
    .expect("model");
    let (clip, reference, _) = two_speakers_x3();
    let start = std::time::Instant::now();
    let mut stream = model.stream();
    let mut probs = stream.push(&mut model.runner, &clip).expect("push");
    probs.extend(stream.finish(&mut model.runner).expect("finish"));
    let agree = agreement(&probs, &reference) as f64 / reference.len() as f64;
    println!(
        "int8: {:?} for 48 s, agreement {agree:.5}, max |dp| {:.4}",
        start.elapsed(),
        max_diff(&probs, &reference)
    );
    assert!(agree > 0.98);
}
