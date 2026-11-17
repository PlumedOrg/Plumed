//! Per-IP cap on room creation (fixed one-hour windows, kept in RAM only).

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(3600);

pub struct Limiter {
    per_window: u32,
    seen: Mutex<HashMap<IpAddr, (Instant, u32)>>,
}

impl Limiter {
    pub fn new(per_window: u32) -> Self {
        Self {
            per_window,
            seen: Mutex::new(HashMap::new()),
        }
    }

    /// `Ok` and counts it, or `Err(seconds until the window resets)`.
    pub fn allow(&self, ip: IpAddr, now: Instant) -> Result<(), u64> {
        let mut seen = self.seen.lock().unwrap_or_else(|e| e.into_inner());
        let entry = seen.entry(ip).or_insert((now, 0));
        if now.duration_since(entry.0) >= WINDOW {
            *entry = (now, 0);
        }
        match entry.1 < self.per_window {
            true => {
                entry.1 += 1;
                Ok(())
            }
            false => Err((WINDOW - now.duration_since(entry.0)).as_secs().max(1)),
        }
    }

    /// Forgets the windows that are over.
    pub fn prune(&self, now: Instant) {
        let mut seen = self.seen.lock().unwrap_or_else(|e| e.into_inner());
        seen.retain(|_, (start, _)| now.duration_since(*start) < WINDOW);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_each_ip_per_hour() {
        let limiter = Limiter::new(2);
        let (a, b): (IpAddr, IpAddr) = ("1.2.3.4".parse().unwrap(), "::1".parse().unwrap());
        let now = Instant::now();
        assert!(limiter.allow(a, now).is_ok());
        assert!(limiter.allow(a, now).is_ok());
        assert_eq!(limiter.allow(a, now), Err(3600));
        assert!(limiter.allow(b, now).is_ok());
        assert!(limiter.allow(a, now + WINDOW).is_ok());
        limiter.prune(now + 2 * WINDOW);
        assert!(limiter.seen.lock().unwrap().is_empty());
    }
}
