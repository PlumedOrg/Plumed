//! The HTTP side of the local API: routes, OpenAI-style errors and the
//! guards every request goes through (Host, Origin, bearer token). Knows
//! nothing of Tauri: the transcription is injected, so tests run without an
//! app or a model.

use std::io::Cursor;
use std::net::IpAddr;
use std::sync::Arc;

use axum::extract::multipart::MultipartRejection;
use axum::extract::{DefaultBodyLimit, Multipart, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use parking_lot::RwLock;
use serde_json::json;

/// Uploads above this are refused (100 MB is about 52 min of 16 kHz mono
/// WAV, or 9 min of 48 kHz stereo).
const MAX_BODY_BYTES: usize = 100_000_000;
const MODEL_ID: &str = "plumed";

/// Transcribes mono samples at their native rate into post-processed text.
pub type Transcriber = dyn Fn(&[f32], u32) -> anyhow::Result<String> + Send + Sync;

#[derive(Clone)]
pub struct ApiContext {
    /// Read on each request, so regenerating the key applies at once.
    pub token: Arc<RwLock<String>>,
    pub transcribe: Arc<Transcriber>,
    /// One transcription at a time: they share the single speech engine.
    pub busy: Arc<tokio::sync::Mutex<()>>,
}

pub fn router(ctx: ApiContext) -> Router {
    Router::new()
        .route("/v1/audio/transcriptions", post(transcriptions))
        .route("/v1/models", get(models))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(middleware::from_fn_with_state(ctx.clone(), require_token))
        // Layers run from the last added: Host, then Origin, then the token.
        .layer(middleware::from_fn(reject_browsers))
        .layer(middleware::from_fn(reject_foreign_host))
        .with_state(ctx)
}

fn error(status: StatusCode, kind: &str, message: impl Into<String>) -> Response {
    let body =
        json!({"error": {"message": message.into(), "type": kind, "param": null, "code": null}});
    (status, Json(body)).into_response()
}

fn bad_request(message: impl Into<String>) -> Response {
    error(StatusCode::BAD_REQUEST, "invalid_request_error", message)
}

/// `localhost` or a loopback address, with or without a port. Anything else
/// means a web page reached us through DNS rebinding.
fn is_local_host(host: &str) -> bool {
    let name = match host.trim().strip_prefix('[') {
        Some(bracketed) => match bracketed.split_once(']') {
            Some((name, _)) => name,
            None => return false,
        },
        None => host
            .trim()
            .split_once(':')
            .map_or(host.trim(), |(name, _)| name),
    };
    name.eq_ignore_ascii_case("localhost")
        || name.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

async fn reject_foreign_host(request: Request, next: Next) -> Response {
    let allowed = match request.headers().get(header::HOST) {
        None => true,
        Some(value) => value.to_str().is_ok_and(is_local_host),
    };
    match allowed {
        true => next.run(request).await,
        false => error(
            StatusCode::FORBIDDEN,
            "invalid_request_error",
            "Host is not allowed",
        ),
    }
}

/// Browsers add `Origin` to cross-site requests; scripts and programs do not.
/// No web page has any business here, so no CORS either.
async fn reject_browsers(request: Request, next: Next) -> Response {
    match request.headers().contains_key(header::ORIGIN) {
        false => next.run(request).await,
        true => error(
            StatusCode::FORBIDDEN,
            "invalid_request_error",
            "Requests from web pages are not allowed",
        ),
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    scheme.eq_ignore_ascii_case("bearer").then(|| token.trim())
}

async fn require_token(State(ctx): State<ApiContext>, request: Request, next: Next) -> Response {
    let valid = {
        let expected = ctx.token.read();
        !expected.is_empty()
            && bearer(request.headers())
                .is_some_and(|given| constant_time_eq(given.as_bytes(), expected.as_bytes()))
    };
    match valid {
        true => next.run(request).await,
        false => error(
            StatusCode::UNAUTHORIZED,
            "invalid_request_error",
            "Missing or invalid API key (Authorization: Bearer <key>, shown in Plumed's settings)",
        ),
    }
}

async fn models() -> Response {
    Json(json!({
        "object": "list",
        "data": [{"id": MODEL_ID, "object": "model", "created": 0, "owned_by": "plumed"}],
    }))
    .into_response()
}

enum Format {
    Json,
    Text,
}

/// `POST /v1/audio/transcriptions`, multipart like OpenAI's: `file` (WAV),
/// optional `response_format` (`json` or `text`). `model`, `language`,
/// `prompt` and `temperature` are accepted and ignored.
async fn transcriptions(
    State(ctx): State<ApiContext>,
    multipart: Result<Multipart, MultipartRejection>,
) -> Response {
    let mut multipart = match multipart {
        Ok(m) => m,
        Err(e) => return bad_request(format!("Expected multipart/form-data: {}", e.body_text())),
    };
    let mut file = None;
    let mut format = Format::Json;
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(e) => return error(e.status(), "invalid_request_error", e.body_text()),
        };
        match field.name() {
            Some("file") => match field.bytes().await {
                Ok(bytes) => file = Some(bytes),
                Err(e) => return error(e.status(), "invalid_request_error", e.body_text()),
            },
            Some("response_format") => {
                format = match field.text().await.as_deref() {
                    Ok("json") => Format::Json,
                    Ok("text") => Format::Text,
                    Ok(other) => {
                        return bad_request(format!(
                            "Unsupported response_format: {other} (json or text)"
                        ))
                    }
                    Err(e) => return error(e.status(), "invalid_request_error", e.body_text()),
                }
            }
            _ => {}
        }
    }
    let Some(file) = file else {
        return bad_request("Missing the 'file' field");
    };
    let (samples, rate) = match crate::audio::helpers::decode_wav_mono(Cursor::new(file)) {
        Ok(decoded) => decoded,
        Err(e) => return bad_request(format!("Only 16-bit PCM WAV files are supported: {e}")),
    };

    let guard = ctx.busy.clone().lock_owned().await;
    let transcribe = ctx.transcribe.clone();
    let joined = tokio::task::spawn_blocking(move || {
        let _guard = guard;
        transcribe(&samples, rate)
    })
    .await;
    match joined {
        Ok(Ok(text)) => match format {
            Format::Json => Json(json!({ "text": text })).into_response(),
            Format::Text => text.into_response(),
        },
        Ok(Err(e)) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "server_error",
            e.to_string(),
        ),
        Err(e) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "server_error",
            e.to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use tower::ServiceExt;

    const TOKEN: &str = "secret-key";
    const BOUNDARY: &str = "XBOUNDARYX";

    fn app() -> Router {
        router(ApiContext {
            token: Arc::new(RwLock::new(TOKEN.to_string())),
            // Echoes what it received, so tests see the decoded audio.
            transcribe: Arc::new(|samples: &[f32], rate: u32| {
                Ok(format!("{} samples at {rate} Hz", samples.len()))
            }),
            busy: Arc::new(tokio::sync::Mutex::new(())),
        })
    }

    /// 0.1 s of 16 kHz mono silence.
    fn wav() -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::new(&mut out, spec).unwrap();
        for _ in 0..1600 {
            writer.write_sample(0i16).unwrap();
        }
        writer.finalize().unwrap();
        out.into_inner()
    }

    fn multipart(fields: &[(&str, &[u8])]) -> Vec<u8> {
        let mut body = Vec::new();
        for (name, value) in fields {
            body.extend(
                format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"").bytes(),
            );
            if *name == "file" {
                body.extend(b"; filename=\"a.wav\"\r\nContent-Type: audio/wav");
            }
            body.extend(b"\r\n\r\n");
            body.extend(*value);
            body.extend(b"\r\n");
        }
        body.extend(format!("--{BOUNDARY}--\r\n").bytes());
        body
    }

    fn post(fields: &[(&str, &[u8])]) -> axum::http::request::Builder {
        Request::post("/v1/audio/transcriptions")
            .header(header::HOST, "127.0.0.1:4800")
            .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .extension(multipart(fields))
    }

    /// Replaces a header (`Builder::header` appends a second one).
    fn set(
        mut builder: axum::http::request::Builder,
        name: header::HeaderName,
        value: &str,
    ) -> axum::http::request::Builder {
        builder
            .headers_mut()
            .unwrap()
            .insert(name, value.parse().unwrap());
        builder
    }

    async fn send(builder: axum::http::request::Builder) -> (StatusCode, String) {
        let body = builder
            .extensions_ref()
            .and_then(|e| e.get::<Vec<u8>>().cloned())
            .unwrap_or_default();
        let response = app()
            .oneshot(builder.body(Body::from(body)).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn transcribes_a_wav_as_json_by_default() {
        let (status, body) = send(post(&[("model", b"whisper-1"), ("file", &wav())])).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, r#"{"text":"1600 samples at 16000 Hz"}"#);
    }

    #[tokio::test]
    async fn text_format_returns_plain_text() {
        let (status, body) = send(post(&[("file", &wav()), ("response_format", b"text")])).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "1600 samples at 16000 Hz");
    }

    #[tokio::test]
    async fn unsupported_format_missing_file_and_non_wav_are_400() {
        let (status, body) = send(post(&[("file", &wav()), ("response_format", b"srt")])).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.contains("invalid_request_error"), "{body}");
        let (status, _) = send(post(&[("model", b"whisper-1")])).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let (status, body) = send(post(&[("file", b"ID3 this is an mp3")])).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.contains("WAV"), "{body}");
    }

    #[tokio::test]
    async fn malformed_or_non_multipart_bodies_are_400() {
        let truncated = post(&[]).extension(b"--XBOUNDARYX\r\nContent-Disp".to_vec());
        assert_eq!(send(truncated).await.0, StatusCode::BAD_REQUEST);
        let json = set(post(&[]), header::CONTENT_TYPE, "application/json");
        assert_eq!(send(json).await.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn missing_or_wrong_key_is_401() {
        let mut no_key = post(&[("file", &wav())]);
        no_key.headers_mut().unwrap().remove(header::AUTHORIZATION);
        assert_eq!(send(no_key).await.0, StatusCode::UNAUTHORIZED);
        let wrong = set(
            post(&[("file", &wav())]),
            header::AUTHORIZATION,
            "Bearer nope",
        );
        assert_eq!(send(wrong).await.0, StatusCode::UNAUTHORIZED);
        let models = Request::get("/v1/models").header(header::HOST, "localhost:4800");
        assert_eq!(send(models).await.0, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn an_empty_key_never_matches() {
        let app = router(ApiContext {
            token: Arc::new(RwLock::new(String::new())),
            transcribe: Arc::new(|_: &[f32], _: u32| Ok(String::new())),
            busy: Arc::new(tokio::sync::Mutex::new(())),
        });
        let request = Request::get("/v1/models")
            .header(header::HOST, "localhost")
            .header(header::AUTHORIZATION, "Bearer ")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.oneshot(request).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn web_pages_and_foreign_hosts_are_403_even_with_the_key() {
        let from_page = post(&[("file", &wav())]).header(header::ORIGIN, "https://evil.example");
        assert_eq!(send(from_page).await.0, StatusCode::FORBIDDEN);
        let rebound = set(post(&[("file", &wav())]), header::HOST, "evil.example:4800");
        assert_eq!(send(rebound).await.0, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn models_lists_plumed() {
        let request = Request::get("/v1/models")
            .header(header::HOST, "[::1]:4800")
            .header(header::AUTHORIZATION, format!("bearer {TOKEN}"));
        let (status, body) = send(request).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains(r#""id":"plumed""#), "{body}");
    }

    #[test]
    fn local_hosts_are_recognised() {
        for host in [
            "localhost",
            "LOCALHOST:4800",
            "127.0.0.1",
            "127.0.0.1:4800",
            "[::1]:4800",
        ] {
            assert!(is_local_host(host), "{host}");
        }
        for host in [
            "evil.example",
            "localhost.evil.example",
            "192.168.1.2:4800",
            "[::1",
            "0.0.0.0",
        ] {
            assert!(!is_local_host(host), "{host}");
        }
    }
}
