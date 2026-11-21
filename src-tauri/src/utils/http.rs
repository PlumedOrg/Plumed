//! The HTTP client setup of the Plumed fork's outgoing requests (usage
//! statistics, medical data updates): HTTPS only for HTTPS urls, no cookie
//! store, a fixed timeout and user agent. Written with Claude Code.

use std::time::Duration;

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
    reqwest::Client::builder()
        .timeout(timeout)
        .https_only(url.starts_with("https://"))
        .redirect(redirect)
        .user_agent(user_agent)
        .build()
        .map_err(|e| e.to_string())
}
