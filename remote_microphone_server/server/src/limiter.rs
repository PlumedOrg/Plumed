//! Per-IP cap on room creation (fixed one-hour windows, kept in RAM only).

use std::collections::HashMap;
use std::net::{IpAddr, Ipv6Addr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(3600);
/// Clients tracked at once: past that, unknown clients wait for old windows
/// to expire (bounds the memory an address-rotating client can take).
const MAX_TRACKED: usize = 65_536;

/// The unit a client is counted by: its IPv4 address, or its IPv6 /64 (one
/// subscriber usually gets a whole /64, so single addresses cost nothing).
pub fn bucket(ip: IpAddr) -> IpAddr {
    match ip.to_canonical() {
        IpAddr::V6(v6) => {
            let s = v6.segments();
            IpAddr::V6(Ipv6Addr::new(s[0], s[1], s[2], s[3], 0, 0, 0, 0))
        }
        v4 => v4,
    }
}

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
        if !seen.contains_key(&ip) && seen.len() >= MAX_TRACKED {
            seen.retain(|_, (start, _)| now.duration_since(*start) < WINDOW);
            if seen.len() >= MAX_TRACKED {
                return Err(60);
            }
        }
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

/// Requests each client has in flight, capped so one client cannot hold
/// every slot of the relay (a long-poll stays open 25 s). Entries leave the
/// map when their count drops to zero, so its size is bounded by the global
/// in-flight cap.
pub struct InFlight {
    per_client: usize,
    counts: Arc<Mutex<HashMap<IpAddr, usize>>>,
}

/// One request in flight, released when dropped.
pub struct Slot {
    ip: IpAddr,
    counts: Arc<Mutex<HashMap<IpAddr, usize>>>,
}

impl InFlight {
    pub fn new(per_client: usize) -> Self {
        Self {
            per_client,
            counts: Arc::default(),
        }
    }

    /// `None` when `ip` already has `per_client` requests in flight.
    pub fn enter(&self, ip: IpAddr) -> Option<Slot> {
        let mut counts = self.counts.lock().unwrap_or_else(|e| e.into_inner());
        let count = counts.entry(ip).or_insert(0);
        match *count < self.per_client {
            true => {
                *count += 1;
                Some(Slot {
                    ip,
                    counts: self.counts.clone(),
                })
            }
            false => None,
        }
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        let mut counts = self.counts.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(count) = counts.get_mut(&self.ip) {
            *count -= 1;
            if *count == 0 {
                counts.remove(&self.ip);
            }
        }
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

    #[test]
    fn ipv6_counts_per_64_and_mapped_ipv4_as_ipv4() {
        let ip = |s: &str| s.parse::<IpAddr>().unwrap();
        assert_eq!(bucket(ip("2001:db8:1:2:aaaa::1")), ip("2001:db8:1:2::"));
        assert_eq!(bucket(ip("2001:db8:1:2:ffff::9")), ip("2001:db8:1:2::"));
        assert_eq!(bucket(ip("::ffff:1.2.3.4")), ip("1.2.3.4"));
        assert_eq!(bucket(ip("1.2.3.4")), ip("1.2.3.4"));
    }

    #[test]
    fn tracked_clients_are_bounded() {
        let limiter = Limiter::new(1);
        let now = Instant::now();
        for i in 0..MAX_TRACKED as u32 {
            limiter.allow(IpAddr::from(i.to_be_bytes()), now).unwrap();
        }
        let newcomer = IpAddr::from(u32::MAX.to_be_bytes());
        assert_eq!(limiter.allow(newcomer, now), Err(60));
        // Old windows make room
        assert!(limiter.allow(newcomer, now + WINDOW).is_ok());
    }

    #[test]
    fn each_client_has_its_own_in_flight_share() {
        let in_flight = InFlight::new(2);
        let (a, b): (IpAddr, IpAddr) = ("1.2.3.4".parse().unwrap(), "5.6.7.8".parse().unwrap());
        let first = in_flight.enter(a).unwrap();
        let _second = in_flight.enter(a).unwrap();
        assert!(in_flight.enter(a).is_none());
        assert!(in_flight.enter(b).is_some());
        drop(first);
        assert!(in_flight.enter(a).is_some());
        // b's request ended: only a is still tracked
        assert_eq!(in_flight.counts.lock().unwrap().len(), 1);
    }
}
