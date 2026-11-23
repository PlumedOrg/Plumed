//! The Plumed app releases on GitHub, and the URL scheme of their routes on
//! the Plumed model relay (`plumed-models`), which the app's updater falls
//! back to when GitHub is unreachable. Shared by the relay and the app
//! (`src-tauri/src/app_update.rs`). Written with Claude Code.
//!
//! A relay path is the GitHub path without the `/<owner>/<name>` prefix, so
//! the app turns a GitHub release URL into a relay URL by swapping the base:
//! - `/releases/latest/download/latest.json`: the latest release's `latest.json`,
//! - `/releases/download/<tag>/<asset>`: one asset of one release,
//! - [`RELAY_LISTING_PATH`]: the 10 newest releases (GitHub API, [`LISTING_API_PATH`]).

use crate::models::safe_segment;

/// The only repository whose releases the relay serves.
pub const GITHUB_REPO: &str = "PlumedOrg/Plumed";
pub const GITHUB_URL: &str = "https://github.com";
pub const GITHUB_API_URL: &str = "https://api.github.com";
/// The updater manifest every app release carries.
pub const LATEST_JSON: &str = "latest.json";
/// The GitHub API listing the beta channel reads (newest first).
pub const LISTING_API_PATH: &str = "/repos/PlumedOrg/Plumed/releases?per_page=10";
/// The relay path of that listing.
pub const RELAY_LISTING_PATH: &str = "/releases/list";
/// Path prefix of the relay's release files.
pub const RELEASES_PATH: &str = "/releases";

/// A release file the relay serves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseFile {
    /// `latest.json` of the latest (non pre-release) release.
    LatestJson,
    /// One asset of one release.
    Asset { tag: String, asset: String },
}

impl ReleaseFile {
    /// Parses a relay path (`/releases/...`). `None` for anything else,
    /// including segments with characters outside `[A-Za-z0-9._-]`.
    pub fn parse(path: &str) -> Option<Self> {
        let rest = path.strip_prefix(RELEASES_PATH)?;
        if rest == format!("/latest/download/{LATEST_JSON}") {
            return Some(Self::LatestJson);
        }
        let mut parts = rest.strip_prefix("/download/")?.split('/');
        match (parts.next(), parts.next(), parts.next()) {
            (Some(tag), Some(asset), None) if safe_segment(tag) && safe_segment(asset) => {
                Some(Self::Asset {
                    tag: tag.to_owned(),
                    asset: asset.to_owned(),
                })
            }
            _ => None,
        }
    }

    /// The relay path, also the GitHub path after `/<owner>/<name>`.
    pub fn path(&self) -> String {
        match self {
            Self::LatestJson => format!("{RELEASES_PATH}/latest/download/{LATEST_JSON}"),
            Self::Asset { tag, asset } => format!("{RELEASES_PATH}/download/{tag}/{asset}"),
        }
    }

    /// The URL path on github.com (appended to [`GITHUB_URL`] or a test upstream).
    pub fn github_path(&self) -> String {
        format!("/{GITHUB_REPO}{}", self.path())
    }

    /// The file a full github.com URL names, if it is one of ours.
    pub fn from_github_url(url: &str) -> Option<Self> {
        let path = url.strip_prefix(GITHUB_URL)?.strip_prefix('/')?;
        Self::parse(path.strip_prefix(GITHUB_REPO)?)
    }
}

/// Hosts a GitHub release download may be redirected to, https only.
pub fn allowed_github_host(scheme: &str, host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    scheme == "https"
        && matches!(
            host.as_str(),
            "github.com" | "objects.githubusercontent.com" | "release-assets.githubusercontent.com"
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_paths_round_trip_and_junk_is_refused() {
        for path in [
            "/releases/latest/download/latest.json",
            "/releases/download/0.2.0-beta.1/Plumed_x64-setup.exe",
        ] {
            assert_eq!(ReleaseFile::parse(path).expect(path).path(), path);
        }
        for bad in [
            "/releases/latest/download/other.json",
            "/releases/latest/download/latest.json/x",
            "/releases/download/tag",
            "/releases/download/tag/a/b",
            "/releases/download/../x",
            "/releases/download/tag/%2e%2e",
            "/releases/download//x",
            "/releases/list",
            "/models/a/b",
        ] {
            assert_eq!(ReleaseFile::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn only_our_github_urls_map_to_a_release_file() {
        let file = ReleaseFile::from_github_url(
            "https://github.com/PlumedOrg/Plumed/releases/download/0.2.0/Plumed_amd64.deb",
        );
        assert_eq!(
            file,
            Some(ReleaseFile::Asset {
                tag: "0.2.0".into(),
                asset: "Plumed_amd64.deb".into()
            })
        );
        assert_eq!(
            file.map(|f| f.github_path()).as_deref(),
            Some("/PlumedOrg/Plumed/releases/download/0.2.0/Plumed_amd64.deb")
        );
        for other in [
            "https://github.com/Other/Plumed/releases/download/0.2.0/x.deb",
            "https://github.com.evil/PlumedOrg/Plumed/releases/download/0.2.0/x.deb",
            "http://github.com/PlumedOrg/Plumed/releases/download/0.2.0/x.deb",
            "https://github.com/PlumedOrg/PlumedX/releases/download/0.2.0/x.deb",
        ] {
            assert_eq!(ReleaseFile::from_github_url(other), None, "{other}");
        }
    }

    #[test]
    fn redirects_stay_on_github_asset_hosts_over_https() {
        assert!(allowed_github_host("https", "github.com"));
        assert!(allowed_github_host(
            "https",
            "release-assets.githubusercontent.com"
        ));
        assert!(allowed_github_host(
            "https",
            "Objects.GitHubUserContent.com"
        ));
        assert!(!allowed_github_host("http", "github.com"));
        assert!(!allowed_github_host("https", "evil.githubusercontent.com"));
        assert!(!allowed_github_host("https", "github.com.evil"));
    }
}
