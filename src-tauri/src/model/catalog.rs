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

use plumed_relay_protocol::models::{
    ModelRepo, ULTIMED_V1_ULTRA, ULTIMED_V2_REDUX, ULTIMED_V2_ULTRA,
};
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
    /// A few English words on what it saves and costs (translated in the UI).
    pub summary: &'static str,
    pub files: &'static [ModelFile],
    /// False while the engine cannot run it: shown as "coming soon".
    pub available: bool,
    /// Listed last, labelled "(EXPERIMENTAL)", and choosing it shows a
    /// warning; never recommended.
    pub experimental: bool,
}

impl Variant {
    pub fn total_size(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }
}

pub const ULTRA_V1: Family = Family {
    id: "ultra-v1",
    name: "UltiMed v1 Ultra",
    repo: ULTIMED_V1_ULTRA,
};

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

const V1_DECODER_INT8: ModelFile = ModelFile {
    path: "int8/decoder_joint-model.int8.onnx",
    size: 18_203_490,
    sha256: "5c621d9d3769505c764e7c2de0c7508427cc1afdba73e1174003d52d4b784468",
    local_name: DECODER,
};

const ULTRA_DECODER_INT8: ModelFile = ModelFile {
    path: "int8/decoder_joint-model.int8.onnx",
    size: 18_203_490,
    sha256: "e1070b64c54a42c14172a6c8ee7e61e56dc92fdb7c4691da46b3944aa844de23",
    local_name: DECODER,
};

/// Every variant, in the order the UI lists them (the first available one
/// is the default, see `recommended`). v1 comes first; v2 is experimental (it
/// often does much worse than v1), so it is listed last and warned about. The v2 ids keep their original names
/// ("ultra-int8", "ultra-w4a8") so existing installs keep their model. Every
/// model uses the same medical data (rules mined from v2, see `MODEL_ID`).
pub const VARIANTS: &[Variant] = &[
    Variant {
        id: "ultra-v1-int8",
        family: &ULTRA_V1,
        name: "UltiMed v1 int8",
        summary: "the proven model, needs about 2 GB of memory",
        files: &[
            ModelFile {
                path: "int8/encoder-model.int8.onnx",
                size: 649_431_593,
                sha256: "f524b0d415dd142502fd504fb7dfe722f455edbe280131ac8fadf9eab3af23a3",
                local_name: ENCODER,
            },
            V1_DECODER_INT8,
            NEMO128,
            VOCAB,
            CONFIG,
        ],
        available: true,
        experimental: false,
    },
    Variant {
        id: "ultra-v1-w4a8",
        family: &ULTRA_V1,
        name: "UltiMed v1 w4a8",
        summary: "1.7x smaller download, 20% less memory, barely less accurate",
        files: &[
            ModelFile {
                path: "w4a8/encoder-model.w4a8.onnx",
                size: 383_355_483,
                sha256: "cd8fc9c08d2d0623f9891f6deb88e6753139fba2a435c7fb4dae4d9e0ff95154",
                local_name: ENCODER,
            },
            V1_DECODER_INT8,
            NEMO128,
            VOCAB,
            CONFIG,
        ],
        available: true,
        experimental: false,
    },
    Variant {
        id: "ultra-int8",
        family: &ULTRA,
        name: "UltiMed v2 int8",
        summary: "often much worse than v1 for now",
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
        experimental: true,
    },
    Variant {
        id: "ultra-w4a8",
        family: &ULTRA,
        name: "UltiMed v2 w4a8",
        summary: "often much worse than v1 for now",
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
        experimental: true,
    },
    // TODO: ONNX Runtime 1.31 runs the redux w2a8 encoder (1.23 refused its
    // 2-bit MatMulNBits), but it is kept off for its accuracy: on the 400
    // clip benchmark its WER is 5.04 %, against 2.87 % for Ultra w4a8, for
    // only 220 MB less peak RAM (1.47 GB against 1.69 GB) and a 108 MB
    // smaller download, and it is not faster. Enable it if a later redux
    // revision comes close to w4a8 (the low RAM recommendation would pick
    // it, being the lightest).
    Variant {
        id: "redux-w2a8",
        family: &REDUX,
        name: "UltiMed v2 Redux w2a8",
        summary: "2.3x smaller download, but nearly twice as many errors",
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
        experimental: true,
    },
];

pub fn find(id: &str) -> Option<&'static Variant> {
    VARIANTS.iter().find(|v| v.id == id)
}

/// Below this much total RAM, the lightest available variant is recommended.
/// Peak RSS of the 400 clip benchmark under ONNX Runtime 1.31: Ultra int8
/// 2.07 GB, w4a8 1.69 GB (380 MB less). An 8 GB machine reports a bit less
/// than 8 GiB (firmware and integrated GPU reservations, e.g. 7.7 GiB), and
/// with the OS, a browser and the hospital software open it often has about
/// 2 GB free: int8's peak would push it into swap, w4a8 fits. From 12 GB on,
/// the difference does not matter and the more accurate default is kept.
const LOW_MEMORY: u64 = 8 << 30;

/// The variant to preselect: the default (the first, tested available), or
/// the lightest available one on a machine with less than `LOW_MEMORY` of
/// RAM. Unknown RAM keeps the default.
pub fn recommended(total_memory: Option<u64>) -> &'static Variant {
    let default = &VARIANTS[0];
    match total_memory {
        Some(total) if total < LOW_MEMORY => VARIANTS
            .iter()
            .filter(|v| v.available && !v.experimental)
            .min_by_key(|v| v.total_size())
            .unwrap_or(default),
        _ => default,
    }
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
        assert_eq!(VARIANTS[0].id, "ultra-v1-int8");
        let first_experimental = VARIANTS.iter().position(|v| v.experimental).unwrap();
        assert!(VARIANTS[first_experimental..]
            .iter()
            .all(|v| v.experimental));
        // int8: 668 MB, w4a8: 402 MB, for v1 and v2 alike
        assert_eq!(VARIANTS[0].total_size(), 667_868_883);
        assert_eq!(VARIANTS[1].total_size(), 401_792_773);
        assert_eq!(find("ultra-int8").unwrap().total_size(), 667_868_883);
        assert_eq!(find("ultra-w4a8").unwrap().total_size(), 401_792_773);
    }

    #[test]
    fn the_default_is_recommended_with_enough_or_unknown_memory() {
        assert_eq!(recommended(None).id, VARIANTS[0].id);
        assert_eq!(recommended(Some(LOW_MEMORY)).id, VARIANTS[0].id);
        assert_eq!(recommended(Some(32 << 30)).id, VARIANTS[0].id);
        // What a 12 GB machine reports.
        assert_eq!(recommended(Some(11_700 << 20)).id, VARIANTS[0].id);
    }

    #[test]
    fn the_lightest_available_variant_is_recommended_with_little_memory() {
        assert_eq!(recommended(Some(LOW_MEMORY - 1)).id, "ultra-v1-w4a8");
        assert_eq!(recommended(Some(4 << 30)).id, "ultra-v1-w4a8");
        // What an 8 GB laptop reports.
        assert_eq!(recommended(Some(7_700 << 20)).id, "ultra-v1-w4a8");
    }
}
