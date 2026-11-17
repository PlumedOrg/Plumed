//! In-memory rooms. Nothing is ever written to disk: a restart drops every room.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use plumed_relay_protocol::{Role, MAILBOX_CAP};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::Notify;

/// Random bytes of a room id (16 base64url chars) and of its secret (24 chars).
const ID_BYTES: usize = 12;
const SECRET_BYTES: usize = 18;

/// The queue toward one end of a room: request bodies appended as they come
/// (envelopes concatenate), taken all at once by the next poll.
#[derive(Default)]
struct Mailbox {
    queue: Mutex<Vec<u8>>,
    notify: Notify,
}

pub struct Room {
    secret: String,
    created: Instant,
    last_seen: Mutex<Instant>,
    closed: AtomicBool,
    mailboxes: [Mailbox; 2],
}

#[derive(Debug, PartialEq)]
pub enum Taken {
    Frames(Vec<u8>),
    /// Nothing arrived before the deadline.
    Empty,
    Closed,
}

#[derive(Debug, PartialEq)]
pub struct MailboxFull;

impl Room {
    fn new(secret: String, now: Instant) -> Self {
        Self {
            secret,
            created: now,
            last_seen: Mutex::new(now),
            closed: AtomicBool::new(false),
            mailboxes: Default::default(),
        }
    }

    fn touch(&self) {
        *lock(&self.last_seen) = Instant::now();
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    /// Wakes the pollers so they see the room is gone.
    fn close(&self) {
        self.closed.store(true, Ordering::Release);
        for mailbox in &self.mailboxes {
            mailbox.notify.notify_waiters();
        }
    }

    /// Queues `envelope` for `to`.
    pub fn push(&self, to: Role, envelope: &[u8]) -> Result<(), MailboxFull> {
        let mailbox = &self.mailboxes[to.index()];
        {
            let mut queue = lock(&mailbox.queue);
            if queue.len() + envelope.len() > MAILBOX_CAP {
                return Err(MailboxFull);
            }
            queue.extend_from_slice(envelope);
        }
        mailbox.notify.notify_waiters();
        Ok(())
    }

    /// Everything queued for `role`, waiting up to `wait` for something to come.
    pub async fn take(&self, role: Role, wait: Duration) -> Taken {
        let mailbox = &self.mailboxes[role.index()];
        let deadline = tokio::time::Instant::now() + wait;
        loop {
            // Registered before the checks, so a push in between still wakes us
            let notified = mailbox.notify.notified();
            if self.is_closed() {
                return Taken::Closed;
            }
            {
                let mut queue = lock(&mailbox.queue);
                if !queue.is_empty() {
                    return Taken::Frames(std::mem::take(&mut *queue));
                }
            }
            if tokio::time::timeout_at(deadline, notified).await.is_err() {
                return Taken::Empty;
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ServerFull;

pub struct Rooms {
    map: Mutex<HashMap<String, Arc<Room>>>,
    max: usize,
}

/// A poisoned lock only means another request panicked: the data is still usable.
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn random_token(len: usize) -> String {
    let mut buf = vec![0u8; len];
    // No entropy means no safe room: abort rather than hand out guessable ids
    getrandom::fill(&mut buf).expect("the system random generator is available");
    B64.encode(buf)
}

/// Compares secrets in constant time (for equal lengths).
fn same_secret(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

impl Rooms {
    pub fn new(max: usize) -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
            max,
        }
    }

    pub fn len(&self) -> usize {
        lock(&self.map).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `(id, secret)` of a new room.
    pub fn create(&self) -> Result<(String, String), ServerFull> {
        let mut map = lock(&self.map);
        if map.len() >= self.max {
            return Err(ServerFull);
        }
        let id = random_token(ID_BYTES);
        let secret = random_token(SECRET_BYTES);
        map.insert(
            id.clone(),
            Arc::new(Room::new(secret.clone(), Instant::now())),
        );
        Ok((id, secret))
    }

    /// The room, if it exists and `secret` matches. Counts as activity.
    pub fn open(&self, id: &str, secret: &str) -> Option<Arc<Room>> {
        let room = lock(&self.map).get(id).cloned()?;
        match same_secret(&room.secret, secret) && !room.is_closed() {
            true => {
                room.touch();
                Some(room)
            }
            false => None,
        }
    }

    pub fn close(&self, id: &str) {
        if let Some(room) = lock(&self.map).remove(id) {
            room.close();
        }
    }

    /// Drops rooms idle for `idle` or older than `max_age`. Returns how many.
    pub fn reap(&self, now: Instant, idle: Duration, max_age: Duration) -> usize {
        let mut map = lock(&self.map);
        let before = map.len();
        map.retain(|_, room| {
            let keep = now.duration_since(*lock(&room.last_seen)) < idle
                && now.duration_since(room.created) < max_age;
            if !keep {
                room.close();
            }
            keep
        });
        before - map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn frames_reach_the_other_side_only() {
        let rooms = Rooms::new(10);
        let (id, secret) = rooms.create().unwrap();
        assert_eq!((id.len(), secret.len()), (16, 24));
        let room = rooms.open(&id, &secret).unwrap();
        room.push(Role::Desktop, b"ab").unwrap();
        room.push(Role::Desktop, b"cd").unwrap();
        let short = Duration::from_millis(10);
        assert_eq!(room.take(Role::Phone, short).await, Taken::Empty);
        assert_eq!(
            room.take(Role::Desktop, short).await,
            Taken::Frames(b"abcd".to_vec())
        );
        assert_eq!(room.take(Role::Desktop, short).await, Taken::Empty);
    }

    #[tokio::test]
    async fn a_waiting_poll_wakes_on_push_and_on_close() {
        let rooms = Arc::new(Rooms::new(10));
        let (id, secret) = rooms.create().unwrap();
        let room = rooms.open(&id, &secret).unwrap();
        let poll = {
            let room = room.clone();
            tokio::spawn(async move { room.take(Role::Phone, Duration::from_secs(5)).await })
        };
        tokio::time::sleep(Duration::from_millis(20)).await;
        room.push(Role::Phone, b"x").unwrap();
        assert_eq!(poll.await.unwrap(), Taken::Frames(b"x".to_vec()));

        let poll = {
            let room = room.clone();
            tokio::spawn(async move { room.take(Role::Phone, Duration::from_secs(5)).await })
        };
        tokio::time::sleep(Duration::from_millis(20)).await;
        rooms.close(&id);
        assert_eq!(poll.await.unwrap(), Taken::Closed);
        assert!(rooms.open(&id, &secret).is_none());
    }

    #[test]
    fn wrong_secret_and_caps() {
        let rooms = Rooms::new(1);
        let (id, secret) = rooms.create().unwrap();
        assert!(rooms.open(&id, "nope").is_none());
        assert!(rooms.open(&id, &secret[..23]).is_none());
        assert!(rooms.open("unknown", &secret).is_none());
        assert_eq!(rooms.create(), Err(ServerFull));
        let room = rooms.open(&id, &secret).unwrap();
        assert_eq!(
            room.push(Role::Phone, &vec![0; MAILBOX_CAP + 1]),
            Err(MailboxFull)
        );
        room.push(Role::Phone, &vec![0; MAILBOX_CAP]).unwrap();
        assert_eq!(room.push(Role::Phone, &[0]), Err(MailboxFull));
    }

    #[test]
    fn idle_and_old_rooms_are_reaped() {
        let rooms = Rooms::new(10);
        let (id, secret) = rooms.create().unwrap();
        let (minute, hour) = (Duration::from_secs(60), Duration::from_secs(3600));
        assert_eq!(rooms.reap(Instant::now(), minute, hour), 0);
        let later = Instant::now() + 2 * minute;
        assert_eq!(rooms.reap(later, minute, hour), 1);
        assert!(rooms.open(&id, &secret).is_none());
        rooms.create().unwrap();
        assert_eq!(rooms.reap(Instant::now() + 2 * hour, 10 * hour, hour), 1);
        assert!(rooms.is_empty());
    }
}
