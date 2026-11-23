//! The speech model repositories Plumed downloads, and the URL scheme of the
//! Plumed model relay (`plumed-models`, `remote_microphone_server/models`).
//! The one place that names the repositories and their pinned revisions: the
//! app's catalog (`src-tauri/src/model/catalog.rs`) builds on these, and the
//! relay's default allowlist is this list. Written with Claude Code.

/// A Hugging Face model repository pinned to one commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelRepo {
    /// `owner/name`
    pub repo: &'static str,
    /// Full commit hash
    pub revision: &'static str,
}

/// UltiMed v1, "ultra" family (int8 and w4a8 encoders): the default while v2
/// is still being evaluated.
pub const ULTIMED_V1_ULTRA: ModelRepo = ModelRepo {
    repo: "Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-v1-onnx",
    revision: "ce5160b5dcc9481fae31a98c3ae0e94c6f12326d",
};

/// UltiMed v2, "ultra" family (int8 and w4a8 encoders).
pub const ULTIMED_V2_ULTRA: ModelRepo = ModelRepo {
    repo: "Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-v2-onnx",
    revision: "675b4b8d7ee9f3ee15a273e3037f76ddc3e730bb",
};

/// UltiMed v2, "redux" family (experimental, w2a8 encoder).
pub const ULTIMED_V2_REDUX: ModelRepo = ModelRepo {
    repo: "Olicorne/parakeet-tdt-0.6b-v3-redux-UltiMed-v2-experimental-onnx",
    revision: "fa44bccba301fed3d8ed14f27ac7cf51db7b6d4e",
};

/// Every repository the app may download from, so every one the relay serves
/// by default. A new pinned revision goes here (and in the relay's
/// `PLUMED_MODELS_REPOS` if the deployment overrides it).
pub const MODEL_REPOS: &[ModelRepo] = &[ULTIMED_V1_ULTRA, ULTIMED_V2_ULTRA, ULTIMED_V2_REDUX];

/// Path prefix of the relay's files.
pub const MODELS_PATH: &str = "/models";

/// Characters allowed in one segment of a repository id or file path.
pub fn safe_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && segment
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// A repository id (`owner/name`) or file path made only of safe segments.
pub fn safe_path(path: &str, min_segments: usize) -> bool {
    let segments: Vec<&str> = path.split('/').collect();
    segments.len() >= min_segments && segments.iter().all(|s| safe_segment(s))
}

/// The relay URL path of `path` in `repo` at `revision`:
/// `/models/<owner>/<name>/<path>?revision=<commit>`.
pub fn relay_path(repo: &ModelRepo, path: &str) -> String {
    format!(
        "{MODELS_PATH}/{}/{path}?revision={}",
        repo.repo, repo.revision
    )
}

/// The Hugging Face hub, where downloads start.
pub const HUB_URL: &str = "https://huggingface.co";

/// The hub URL path of `path` in `repo` at `revision`
/// (`/<owner>/<name>/resolve/<commit>/<path>`), appended to [`HUB_URL`] or
/// to a test upstream.
pub fn hub_path(repo: &str, revision: &str, path: &str) -> String {
    format!("/{repo}/resolve/{revision}/{path}")
}

/// Hosts a Hugging Face download may be redirected to (its CDN), https only.
pub fn allowed_hub_host(scheme: &str, host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    scheme == "https"
        && (host == "huggingface.co"
            || host.ends_with(".huggingface.co")
            || host == "hf.co"
            || host.ends_with(".hf.co"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pinned_repositories_are_safe_and_pinned_to_full_commits() {
        for repo in MODEL_REPOS {
            assert!(safe_path(repo.repo, 2) && repo.repo.split('/').count() == 2);
            assert_eq!(repo.revision.len(), 40);
            assert!(repo.revision.bytes().all(|b| b.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn segments_refuse_traversal_and_encoding_tricks() {
        assert!(safe_segment("encoder-model.int8.onnx"));
        for bad in ["", ".", "..", "a%2e", "a b", "a?b", "é"] {
            assert!(!safe_segment(bad), "{bad}");
        }
        assert!(safe_path("int8/encoder.onnx", 1));
        assert!(!safe_path("int8//encoder.onnx", 1));
        assert!(!safe_path("../x", 1));
    }

    #[test]
    fn urls_follow_the_documented_schemes() {
        assert_eq!(
            relay_path(&ULTIMED_V2_ULTRA, "int8/x.onnx"),
            "/models/Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-v2-onnx/int8/x.onnx\
             ?revision=675b4b8d7ee9f3ee15a273e3037f76ddc3e730bb"
        );
        assert_eq!(
            hub_path(
                ULTIMED_V2_ULTRA.repo,
                ULTIMED_V2_ULTRA.revision,
                "vocab.txt"
            ),
            "/Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-v2-onnx\
             /resolve/675b4b8d7ee9f3ee15a273e3037f76ddc3e730bb/vocab.txt"
        );
    }

    #[test]
    fn only_https_hub_hosts_are_allowed() {
        assert!(allowed_hub_host("https", "huggingface.co"));
        assert!(allowed_hub_host("https", "cdn-lfs.huggingface.co"));
        assert!(allowed_hub_host("https", "us.aws.cdn.hf.co"));
        assert!(!allowed_hub_host("http", "huggingface.co"));
        assert!(!allowed_hub_host("https", "huggingface.co.evil.com"));
        assert!(!allowed_hub_host("https", "evilhf.co"));
    }
}
