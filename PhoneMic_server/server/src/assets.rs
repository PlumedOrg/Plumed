//! The phone page, embedded in the binary (no file system access at runtime).

use axum::http::header;
use axum::routing::get;
use axum::Router;

const HTML: &str = "text/html; charset=utf-8";
const JS: &str = "text/javascript; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";

macro_rules! asset {
    ($path:literal, $file:literal, $type:expr) => {
        (
            $path,
            $type,
            include_bytes!(concat!("../static/", $file)).as_slice(),
        )
    };
}

const ASSETS: &[(&str, &str, &[u8])] = &[
    asset!("/", "index.html", HTML),
    asset!("/style.css", "style.css", CSS),
    asset!("/app.js", "app.js", JS),
    asset!("/audio.js", "audio.js", JS),
    asset!("/crypto.js", "crypto.js", JS),
    asset!("/i18n.js", "i18n.js", JS),
    asset!("/outbox.js", "outbox.js", JS),
    asset!("/relay.js", "relay.js", JS),
    asset!("/store.js", "store.js", JS),
    asset!("/vad.js", "vad.js", JS),
    asset!("/wire.js", "wire.js", JS),
    asset!("/worklet.js", "worklet.js", JS),
];

pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    ASSETS
        .iter()
        .fold(Router::new(), |router, &(path, content_type, body)| {
            router.route(
                path,
                get(move || async move { ([(header::CONTENT_TYPE, content_type)], body) }),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::ASSETS;

    /// Every `./x.js` a served file imports (or loads as a worklet) is served too.
    #[test]
    fn every_imported_module_is_served() {
        let served: Vec<&str> = ASSETS.iter().map(|&(path, _, _)| path).collect();
        for &(path, _, body) in ASSETS {
            let text = std::str::from_utf8(body).unwrap();
            for piece in text.split('\'').skip(1).step_by(2) {
                if let Some(module) = piece.strip_prefix("./").filter(|m| m.ends_with(".js")) {
                    let wanted = format!("/{module}");
                    assert!(served.contains(&wanted.as_str()), "{path} needs {wanted}");
                }
            }
        }
    }
}
