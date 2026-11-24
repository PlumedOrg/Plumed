//! Serves the phone page, embedded in the protocol crate (`page::PAGE`).

use axum::http::header;
use axum::routing::get;
use axum::Router;
use plumed_relay_protocol::page::PAGE;

pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    PAGE.iter().fold(Router::new(), |router, asset| {
        let (content_type, body) = (asset.content_type, asset.body);
        router.route(
            asset.path,
            get(move || async move { ([(header::CONTENT_TYPE, content_type)], body) }),
        )
    })
}
