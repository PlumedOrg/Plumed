//! The speech models Plumed can download (Plumed fork, written with Claude
//! Code): every file pinned by repository revision, size and SHA-256,
//! compiled in. The repositories and revisions come from the crate shared
//! with the model relay (`plumed_relay_protocol::models`), so the relay's
//! default allowlist always matches. To add a model: add its files here (and
//! its repository to `MODEL_REPOS` if new), see AGENTS.FORK.md.
//!
//! The engine loads `encoder-model.int8.onnx`, `decoder_joint-model.int8.onnx`,
//! `nemo128.onnx`, `vocab.txt` and `config.json` from one folder, so every
//! variant is installed under those local names whatever its quantization.

use plumed_relay_protocol::models::{ModelRepo, ULTIMED_V2_REDUX, ULTIMED_V2_ULTRA};
use serde::Serialize;

/// One file of a variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ModelFile {
    /// Path in the repository.
    pub path: &'static str,
    pub size: u64,
    /// Lowercase hex.
    pub sha256: &'static str,
    /// Name in the installed model folder (what the engine loads).
    pub local_name: &'static str,
}

/// A family of models trained the same way (one repository).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Family {
    pub id: &'static str,
    pub name: &'static str,
    pub repo: ModelRepo,
}

/// One downloadable variant (a quantization of a family).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Variant {
    /// Stable id: settings, folder names, telemetry (allowlisted values).
    pub id: &'static str,
    pub family: &'static Family,
    pub name: &'static str,
    /// A few English words on accuracy and speed (translated in the UI).
    pub summary: &'static str,
    pub files: &'static [ModelFile],
    /// False while the engine cannot run it: shown as "coming soon".
    pub available: bool,
}

impl Variant {
    pub fn total_size(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }
}

pub const ULTRA: Family = Family {
    id: "ultra",
    name: "UltiMed v2 Ultra",
    repo: ULTIMED_V2_ULTRA,
};

pub const REDUX: Family = Family {
    id: "redux",
    name: "UltiMed v2 Redux",
    repo: ULTIMED_V2_REDUX,
};

const NEMO128: ModelFile = ModelFile {
    path: "nemo128.onnx",
    size: 139_764,
    sha256: "a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f",
    local_name: "nemo128.onnx",
};

const VOCAB: ModelFile = ModelFile {
    path: "vocab.txt",
    size: 93_939,
    sha256: "d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d",
    local_name: "vocab.txt",
};

const CONFIG: ModelFile = ModelFile {
    path: "config.json",
    size: 97,
    sha256: "666903c76b9798caf2c210afd4f6cd60b08a8dbf9800ec8d7a3bc0d2148ac466",
    local_name: "config.json",
};

const ENCODER: &str = "encoder-model.int8.onnx";
const DECODER: &str = "decoder_joint-model.int8.onnx";

const ULTRA_DECODER_INT8: ModelFile = ModelFile {
    path: "int8/decoder_joint-model.int8.onnx",
    size: 18_203_490,
    sha256: "e1070b64c54a42c14172a6c8ee7e61e56dc92fdb7c4691da46b3944aa844de23",
    local_name: DECODER,
};

/// Every variant, in the order the UI lists them (the first available one
/// is the recommended default).
pub const VARIANTS: &[Variant] = &[
    Variant {
        id: "ultra-int8",
        family: &ULTRA,
        name: "Ultra int8",
        summary: "the most accurate",
        files: &[
            ModelFile {
                path: "int8/encoder-model.int8.onnx",
                size: 649_431_593,
                sha256: "837c1689edd757b69f52b562b50667ab53f8d2693d58645108702c6621ca3fbd",
                local_name: ENCODER,
            },
            ULTRA_DECODER_INT8,
            NEMO128,
            VOCAB,
            CONFIG,
        ],
        available: true,
    },
    Variant {
        id: "ultra-w4a8",
        family: &ULTRA,
        name: "Ultra w4a8",
        summary: "about half the size, barely less accurate",
        files: &[
            ModelFile {
                path: "w4a8/encoder-model.w4a8.onnx",
                size: 383_355_483,
                sha256: "5de4e3498dd6ba5595e0e0166aa595c01e088c4982b73b49dc9c480ef167fd51",
                local_name: ENCODER,
            },
            ULTRA_DECODER_INT8,
            NEMO128,
            VOCAB,
            CONFIG,
        ],
        available: true,
    },
    // TODO: the redux w2a8 encoder needs 2-bit MatMulNBits, which ONNX
    // Runtime 1.23 (the one Plumed ships) refuses ("Only 4b and 8b
    // quantization is supported"). Enable it once Plumed ships an ONNX
    // Runtime that runs it, or once the repository publishes an int8 or
    // w4a8 encoder.
    Variant {
        id: "redux-w2a8",
        family: &REDUX,
        name: "Redux w2a8",
        summary: "experimental, smallest",
        files: &[
            ModelFile {
                path: "w2a8/encoder-model.w2a8.onnx",
                size: 256_215_598,
                sha256: "85177a74c05491e8af0fc32d6be72b0abf366162894d43f3b2f01dff01e6f733",
                local_name: ENCODER,
            },
            ModelFile {
                path: "int8/decoder_joint-model.int8.onnx",
                size: 37_135_949,
                sha256: "e514fd01ea4dbca9deaae9df3b72f3e17da4f0c1e345105d9a8947d64fd66560",
                local_name: DECODER,
            },
            NEMO128,
            VOCAB,
            CONFIG,
        ],
        available: false,
    },
];

pub fn find(id: &str) -> Option<&'static Variant> {
    VARIANTS.iter().find(|v| v.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use plumed_relay_protocol::models::{safe_path, MODEL_REPOS};
    use std::collections::HashSet;

    /// What the engine loads from the model folder.
    const ENGINE_FILES: [&str; 5] = [ENCODER, DECODER, "nemo128.onnx", "vocab.txt", "config.json"];

    #[test]
    fn every_variant_installs_exactly_the_files_the_engine_loads() {
        for variant in VARIANTS {
            let names: HashSet<&str> = variant.files.iter().map(|f| f.local_name).collect();
            assert_eq!(names, ENGINE_FILES.into_iter().collect(), "{}", variant.id);
            assert_eq!(names.len(), variant.files.len(), "{}", variant.id);
        }
    }

    #[test]
    fn every_file_is_pinned_and_safe() {
        for file in VARIANTS.iter().flat_map(|v| v.files) {
            assert!(safe_path(file.path, 1), "{}", file.path);
            assert_eq!(file.sha256.len(), 64, "{}", file.path);
            assert!(file
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
            assert!(file.size > 0);
        }
    }

    #[test]
    fn ids_are_unique_and_the_relay_serves_every_repository() {
        let ids: HashSet<&str> = VARIANTS.iter().map(|v| v.id).collect();
        assert_eq!(ids.len(), VARIANTS.len());
        for variant in VARIANTS {
            assert!(MODEL_REPOS.contains(&variant.family.repo), "{}", variant.id);
            assert!(variant.id.starts_with(variant.family.id));
        }
    }

    #[test]
    fn the_first_variant_is_available_and_sizes_add_up() {
        assert!(VARIANTS[0].available);
        // Ultra int8: 668 MB, w4a8: 402 MB
        assert_eq!(VARIANTS[0].total_size(), 667_868_883);
        assert_eq!(VARIANTS[1].total_size(), 401_792_773);
    }
}
