//! The phone page (`server/static/`), embedded here so that the relay serves
//! it and the desktop app checks it from the very same table: Plumed fetches
//! every file from the relay and refuses it if one byte differs, which deters
//! a relay from serving a tampered page (see protocol_details.md, threat model).

const HTML: &str = "text/html; charset=utf-8";
const JS: &str = "text/javascript; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";

/// One file of the page.
pub struct Asset {
    pub path: &'static str,
    pub content_type: &'static str,
    pub body: &'static [u8],
}

macro_rules! asset {
    ($path:literal, $file:literal, $type:expr) => {
        Asset {
            path: $path,
            content_type: $type,
            body: include_bytes!(concat!("../../server/static/", $file)).as_slice(),
        }
    };
}

pub const PAGE: &[Asset] = &[
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

#[cfg(test)]
mod tests {
    use super::PAGE;

    /// Every `./x.js` a served file imports (or loads as a worklet) is served too.
    #[test]
    fn every_imported_module_is_served() {
        let served: Vec<&str> = PAGE.iter().map(|a| a.path).collect();
        for asset in PAGE {
            let text = std::str::from_utf8(asset.body).unwrap();
            for piece in text.split('\'').skip(1).step_by(2) {
                if let Some(module) = piece.strip_prefix("./").filter(|m| m.ends_with(".js")) {
                    let wanted = format!("/{module}");
                    assert!(served.contains(&wanted.as_str()), "{} needs {wanted}", asset.path);
                }
            }
        }
    }
}
