//! The HTTP client setup of the Plumed fork's outgoing requests (usage
//! statistics, medical data updates, speech model downloads): HTTPS only for
//! HTTPS urls, no cookie store, a fixed user agent. Written with Claude Code.

use std::time::Duration;

/// Most redirects a client follows.
const MAX_REDIRECTS: usize = 5;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Longest wait for the next bytes of a streamed download.
const READ_TIMEOUT: Duration = Duration::from_secs(60);

/// A client for requests to `url`. `max_redirects` is 0 for endpoints that
/// must answer themselves (a redirect would then fail, not be followed).
pub fn client(
    url: &str,
    timeout: Duration,
    user_agent: &str,
    max_redirects: usize,
) -> Result<reqwest::Client, String> {
    let redirect = match max_redirects {
        0 => reqwest::redirect::Policy::none(),
        n => reqwest::redirect::Policy::limited(n),
    };
    builder(url, user_agent, redirect)
        .timeout(timeout)
        .build()
        .map_err(|e| e.to_string())
}

/// A client for large downloads from `url`: no overall timeout, but one on
/// connecting and on each read. Redirects are followed only to urls
/// `redirect_allowed` accepts (none when it is `None`).
pub fn streaming_client(
    url: &str,
    user_agent: &str,
    redirect_allowed: Option<fn(&reqwest::Url) -> bool>,
) -> Result<reqwest::Client, String> {
    let redirect = match redirect_allowed {
        None => reqwest::redirect::Policy::none(),
        Some(allowed) => reqwest::redirect::Policy::custom(move |attempt| {
            match (
                attempt.previous().len() >= MAX_REDIRECTS,
                allowed(attempt.url()),
            ) {
                (true, _) => attempt.error("too many redirects"),
                (false, true) => attempt.follow(),
                (false, false) => attempt.stop(),
            }
        }),
    };
    builder(url, user_agent, redirect)
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())
}

fn builder(
    url: &str,
    user_agent: &str,
    redirect: reqwest::redirect::Policy,
) -> reqwest::ClientBuilder {
    // reqwest panics without a TLS provider; `run` installs it, tests do not.
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::Client::builder()
        .https_only(url.starts_with("https://"))
        .redirect(redirect)
        .user_agent(user_agent)
}
