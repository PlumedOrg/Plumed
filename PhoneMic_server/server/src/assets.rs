//! Serves the phone pages: the current one, embedded in the protocol crate
//! (`page::PAGE`), and the archived ones of `pages/` (see `build.rs`), each
//! under `/p/<page version>/`.

use axum::http::header;
use axum::response::Redirect;
use axum::routing::get;
use axum::Router;
use plumed_relay_protocol::page::{content_type, page_path, PAGE, PAGE_VERSION};

include!(concat!(env!("OUT_DIR"), "/archived_pages.rs"));

/// Every page version served, oldest first. Example: `[1, 2, 3]`.
pub fn page_versions() -> Vec<u32> {
    ARCHIVED
        .iter()
        .map(|(v, _)| *v)
        .chain([PAGE_VERSION])
        .collect()
}

/// `(path, content type, body)` of every file of every page.
fn files() -> impl Iterator<Item = (String, &'static str, &'static [u8])> {
    let current = PAGE
        .iter()
        .map(|a| (page_path(PAGE_VERSION, a.path), a.content_type, a.body));
    let archived = ARCHIVED.iter().flat_map(|(version, files)| {
        files.iter().map(move |(name, body)| {
            let path = match *name {
                "index.html" => "/".to_string(),
                other => format!("/{other}"),
            };
            (page_path(*version, &path), content_type(name), *body)
        })
    });
    current.chain(archived)
}

pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    let latest = page_path(PAGE_VERSION, "/");
    files().fold(
        // Someone typing the bare address lands on the current page
        Router::new().route(
            "/",
            get(move || async move { Redirect::temporary(&latest) }),
        ),
        |router, (path, content_type, body)| {
            router.route(
                &path,
                get(move || async move { ([(header::CONTENT_TYPE, content_type)], body) }),
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use plumed_relay_protocol::PROTOCOL_VERSION;

    #[test]
    fn archived_pages_are_older_and_unique() {
        let versions = page_versions();
        assert!(versions.windows(2).all(|w| w[0] < w[1]), "{versions:?}");
    }

    /// The relay speaks one protocol: an archived page on another one could
    /// never pair. Empty `pages/` when bumping PROTOCOL_VERSION.
    #[test]
    fn archived_pages_speak_the_current_protocol() {
        let wanted = format!("PROTOCOL_VERSION = '{PROTOCOL_VERSION}'");
        for (version, files) in ARCHIVED {
            let wire = files
                .iter()
                .find(|(name, _)| *name == "wire.js")
                .map(|(_, b)| *b);
            let wire = std::str::from_utf8(wire.expect("an archived page has wire.js")).unwrap();
            assert!(
                wire.contains(&wanted),
                "pages/{version} speaks another protocol"
            );
        }
    }
}
