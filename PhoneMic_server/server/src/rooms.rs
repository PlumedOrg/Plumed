//! In-memory rooms. Nothing is ever written to disk: a restart drops every room.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use plumed_relay_protocol::{Role, MAILBOX_CAP, MAX_PHONE_INBOXES};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::Notify;

/// Random bytes of a room id (16 base64url chars) and of its secrets (24 chars).
const ID_BYTES: usize = 12;
const SECRET_BYTES: usize = 18;
/// A phone inbox nobody used for this long may be dropped to make room for a
/// new phone (a polling phone touches its inbox every 25 s).
const PHONE_INBOX_IDLE: Duration = Duration::from_secs(60);

/// Bytes queued in every mailbox of the relay together: a full server stays
/// well inside the container memory limit (256 MiB in docker-compose.yml).
pub const QUEUE_BUDGET: usize = 64 * 1024 * 1024;

/// Bytes queued across all mailboxes, against a maximum.
struct Budget {
    used: AtomicUsize,
    max: usize,
}

impl Budget {
    fn reserve(&self, n: usize) -> bool {
        self.used
            .try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                (used + n <= self.max).then_some(used + n)
            })
            .is_ok()
    }

    fn release(&self, n: usize) {
        self.used.fetch_sub(n, Ordering::AcqRel);
    }
}

/// One inbox: request bodies appended as they come (envelopes concatenate),
/// taken all at once by the next poll.
struct Mailbox {
    queue: Mutex<Vec<u8>>,
    notify: Notify,
    last_used: Mutex<Instant>,
    budget: Arc<Budget>,
}

impl Mailbox {
    fn new(now: Instant, budget: Arc<Budget>) -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            notify: Notify::new(),
            last_used: Mutex::new(now),
            budget,
        }
    }
}

/// A dropped inbox (reaped room, evicted phone) gives its bytes back.
impl Drop for Mailbox {
    fn drop(&mut self) {
        let queue = self.queue.get_mut().unwrap_or_else(|e| e.into_inner());
        self.budget.release(queue.len());
    }
}

/// Which inbox of a room.
#[derive(Clone, Copy, Debug)]
pub enum Inbox<'a> {
    Desktop,
    /// A phone, by the tag it chose.
    Phone(&'a str),
}

impl Inbox<'_> {
    /// Who reads it (the writer is the other role).
    pub fn owner(self) -> Role {
        match self {
            Inbox::Desktop => Role::Desktop,
            Inbox::Phone(_) => Role::Phone,
        }
    }
}

pub struct Room {
    /// Who created it (see `limiter::bucket`), for the per-client room cap.
    owner: IpAddr,
    desktop_secret: String,
    phone_secret: String,
    created: Instant,
    last_seen: Mutex<Instant>,
    closed: AtomicBool,
    desktop: Arc<Mailbox>,
    phones: Mutex<HashMap<String, Arc<Mailbox>>>,
    budget: Arc<Budget>,
}

#[derive(Debug, PartialEq)]
pub enum Taken {
    Frames(Vec<u8>),
    /// Nothing arrived before the deadline.
    Empty,
    Closed,
}

#[derive(Debug, PartialEq)]
pub enum PushError {
    /// The reader is not polling (asleep, offline), or the relay as a whole
    /// holds too much: slow down.
    Full,
    /// Too many phone inboxes in use.
    TooManyPhones,
}

impl Room {
    fn new(
        owner: IpAddr,
        desktop_secret: String,
        phone_secret: String,
        now: Instant,
        budget: Arc<Budget>,
    ) -> Self {
        Self {
            owner,
            desktop_secret,
            phone_secret,
            created: now,
            last_seen: Mutex::new(now),
            closed: AtomicBool::new(false),
            desktop: Arc::new(Mailbox::new(now, budget.clone())),
            phones: Mutex::new(HashMap::new()),
            budget,
        }
    }

    fn secret(&self, role: Role) -> &str {
        match role {
            Role::Desktop => &self.desktop_secret,
            Role::Phone => &self.phone_secret,
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
        self.desktop.notify.notify_waiters();
        for mailbox in lock(&self.phones).values() {
            mailbox.notify.notify_waiters();
        }
    }

    /// The inbox, created for a new phone tag (dropping idle phone inboxes
    /// when [`MAX_PHONE_INBOXES`] are in use).
    fn mailbox(&self, inbox: Inbox<'_>) -> Result<Arc<Mailbox>, PushError> {
        let now = Instant::now();
        let mailbox = match inbox {
            Inbox::Desktop => self.desktop.clone(),
            Inbox::Phone(tag) => {
                let mut phones = lock(&self.phones);
                if !phones.contains_key(tag) && phones.len() >= MAX_PHONE_INBOXES {
                    phones
                        .retain(|_, m| now.duration_since(*lock(&m.last_used)) < PHONE_INBOX_IDLE);
                }
                if !phones.contains_key(tag) && phones.len() >= MAX_PHONE_INBOXES {
                    return Err(PushError::TooManyPhones);
                }
                phones
                    .entry(tag.to_string())
                    .or_insert_with(|| Arc::new(Mailbox::new(now, self.budget.clone())))
                    .clone()
            }
        };
        *lock(&mailbox.last_used) = now;
        Ok(mailbox)
    }

    /// Queues `envelope` in `inbox`.
    pub fn push(&self, inbox: Inbox<'_>, envelope: &[u8]) -> Result<(), PushError> {
        let mailbox = self.mailbox(inbox)?;
        {
            let mut queue = lock(&mailbox.queue);
            if queue.len() + envelope.len() > MAILBOX_CAP || !mailbox.budget.reserve(envelope.len())
            {
                return Err(PushError::Full);
            }
            queue.extend_from_slice(envelope);
        }
        mailbox.notify.notify_waiters();
        Ok(())
    }

    /// Everything queued in `inbox`, waiting up to `wait` for something to come.
    pub async fn take(&self, inbox: Inbox<'_>, wait: Duration) -> Result<Taken, PushError> {
        let mailbox = self.mailbox(inbox)?;
        let deadline = tokio::time::Instant::now() + wait;
        let taken = loop {
            // Registered before the checks, so a push in between still wakes us
            let notified = mailbox.notify.notified();
            if self.is_closed() {
                break Taken::Closed;
            }
            {
                let mut queue = lock(&mailbox.queue);
                if !queue.is_empty() {
                    let bytes = std::mem::take(&mut *queue);
                    mailbox.budget.release(bytes.len());
                    break Taken::Frames(bytes);
                }
            }
            if tokio::time::timeout_at(deadline, notified).await.is_err() {
                break Taken::Empty;
            }
        };
        // Still in use while the poll was held
        *lock(&mailbox.last_used) = Instant::now();
        Ok(taken)
    }
}

#[derive(Debug, PartialEq)]
pub enum Refused {
    ServerFull,
    /// This client already has its share of the open rooms.
    TooManyForClient,
}

/// A new room: its id and the secret of each role.
#[derive(Debug, PartialEq)]
pub struct Created {
    pub id: String,
    pub desktop_secret: String,
    pub phone_secret: String,
}

pub struct Rooms {
    map: Mutex<HashMap<String, Arc<Room>>>,
    max: usize,
    per_owner: usize,
    budget: Arc<Budget>,
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
    /// At most `max` rooms, `per_owner` of them for one client, and
    /// `queue_budget` bytes queued in all of them together.
    pub fn new(max: usize, per_owner: usize, queue_budget: usize) -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
            max,
            per_owner,
            budget: Arc::new(Budget {
                used: AtomicUsize::new(0),
                max: queue_budget,
            }),
        }
    }

    pub fn len(&self) -> usize {
        lock(&self.map).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn create(&self, owner: IpAddr) -> Result<Created, Refused> {
        let mut map = lock(&self.map);
        if map.len() >= self.max {
            return Err(Refused::ServerFull);
        }
        if map.values().filter(|r| r.owner == owner).count() >= self.per_owner {
            return Err(Refused::TooManyForClient);
        }
        let created = Created {
            id: random_token(ID_BYTES),
            desktop_secret: random_token(SECRET_BYTES),
            phone_secret: random_token(SECRET_BYTES),
        };
        let room = Room::new(
            owner,
            created.desktop_secret.clone(),
            created.phone_secret.clone(),
            Instant::now(),
            self.budget.clone(),
        );
        map.insert(created.id.clone(), Arc::new(room));
        Ok(created)
    }

    /// The room, if it exists and `secret` is the one of `role`. Counts as activity.
    pub fn open(&self, id: &str, role: Role, secret: &str) -> Option<Arc<Room>> {
        let room = lock(&self.map).get(id).cloned()?;
        match same_secret(room.secret(role), secret) && !room.is_closed() {
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

    const TAG_A: &str = "AAAAAAAAAAAAAAAAAAAAAA";
    const TAG_B: &str = "BBBBBBBBBBBBBBBBBBBBBB";

    const IP: IpAddr = IpAddr::V4(std::net::Ipv4Addr::LOCALHOST);

    fn room(rooms: &Rooms) -> (Created, Arc<Room>) {
        let created = rooms.create(IP).unwrap();
        let room = rooms
            .open(&created.id, Role::Desktop, &created.desktop_secret)
            .unwrap();
        (created, room)
    }

    #[tokio::test]
    async fn frames_reach_their_inbox_only() {
        let rooms = Rooms::new(10, 10, QUEUE_BUDGET);
        let (created, room) = room(&rooms);
        assert_eq!(created.id.len(), 16);
        assert_eq!(created.desktop_secret.len(), 24);
        assert_ne!(created.desktop_secret, created.phone_secret);
        room.push(Inbox::Desktop, b"ab").unwrap();
        room.push(Inbox::Desktop, b"cd").unwrap();
        room.push(Inbox::Phone(TAG_A), b"a").unwrap();
        let short = Duration::from_millis(10);
        assert_eq!(
            room.take(Inbox::Phone(TAG_B), short).await,
            Ok(Taken::Empty)
        );
        assert_eq!(
            room.take(Inbox::Phone(TAG_A), short).await,
            Ok(Taken::Frames(b"a".to_vec()))
        );
        assert_eq!(
            room.take(Inbox::Desktop, short).await,
            Ok(Taken::Frames(b"abcd".to_vec()))
        );
        assert_eq!(room.take(Inbox::Desktop, short).await, Ok(Taken::Empty));
    }

    #[tokio::test]
    async fn a_waiting_poll_wakes_on_push_and_on_close() {
        let rooms = Arc::new(Rooms::new(10, 10, QUEUE_BUDGET));
        let (created, room) = room(&rooms);
        let poll = |room: Arc<Room>| {
            tokio::spawn(
                async move { room.take(Inbox::Phone(TAG_A), Duration::from_secs(5)).await },
            )
        };
        let waiting = poll(room.clone());
        tokio::time::sleep(Duration::from_millis(20)).await;
        room.push(Inbox::Phone(TAG_A), b"x").unwrap();
        assert_eq!(waiting.await.unwrap(), Ok(Taken::Frames(b"x".to_vec())));

        let waiting = poll(room.clone());
        tokio::time::sleep(Duration::from_millis(20)).await;
        rooms.close(&created.id);
        assert_eq!(waiting.await.unwrap(), Ok(Taken::Closed));
        assert!(rooms
            .open(&created.id, Role::Desktop, &created.desktop_secret)
            .is_none());
    }

    #[test]
    fn each_role_needs_its_own_secret() {
        let rooms = Rooms::new(1, 1, QUEUE_BUDGET);
        let created = rooms.create(IP).unwrap();
        let (id, desktop, phone) = (&created.id, &created.desktop_secret, &created.phone_secret);
        assert!(rooms.open(id, Role::Desktop, desktop).is_some());
        assert!(rooms.open(id, Role::Phone, phone).is_some());
        assert!(rooms.open(id, Role::Desktop, phone).is_none());
        assert!(rooms.open(id, Role::Phone, desktop).is_none());
        assert!(rooms.open(id, Role::Phone, "nope").is_none());
        assert!(rooms.open(id, Role::Phone, &phone[..23]).is_none());
        assert!(rooms.open("unknown", Role::Phone, phone).is_none());
        assert_eq!(rooms.create(IP), Err(Refused::ServerFull));
    }

    #[test]
    fn each_client_gets_a_share_of_the_rooms() {
        let rooms = Rooms::new(10, 2, QUEUE_BUDGET);
        let other = IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 2));
        rooms.create(IP).unwrap();
        let second = rooms.create(IP).unwrap();
        assert_eq!(rooms.create(IP), Err(Refused::TooManyForClient));
        rooms.create(other).unwrap();
        rooms.close(&second.id);
        rooms.create(IP).unwrap();
    }

    #[tokio::test]
    async fn queued_bytes_count_against_the_relay_budget() {
        let rooms = Rooms::new(10, 10, 10);
        let (_, a) = room(&rooms);
        let (b_created, b) = room(&rooms);
        a.push(Inbox::Desktop, &[0; 6]).unwrap();
        // Room b has its own mailbox cap, but the relay is full
        assert_eq!(b.push(Inbox::Desktop, &[0; 6]), Err(PushError::Full));
        // Taking frees the bytes...
        a.take(Inbox::Desktop, Duration::ZERO).await.unwrap();
        b.push(Inbox::Desktop, &[0; 6]).unwrap();
        // ...and so does dropping a room with bytes still queued
        rooms.close(&b_created.id);
        drop(b);
        a.push(Inbox::Phone(TAG_A), &[0; 10]).unwrap();
    }

    #[test]
    fn mailbox_and_phone_caps() {
        let rooms = Rooms::new(1, 1, QUEUE_BUDGET);
        let (_, room) = room(&rooms);
        assert_eq!(
            room.push(Inbox::Phone(TAG_A), &vec![0; MAILBOX_CAP + 1]),
            Err(PushError::Full)
        );
        room.push(Inbox::Phone(TAG_A), &vec![0; MAILBOX_CAP])
            .unwrap();
        assert_eq!(room.push(Inbox::Phone(TAG_A), &[0]), Err(PushError::Full));
        let tags: Vec<String> = (1..MAX_PHONE_INBOXES)
            .map(|i| format!("{i:0>22}"))
            .collect();
        for tag in &tags {
            room.push(Inbox::Phone(tag), b"x").unwrap();
        }
        assert_eq!(
            room.push(Inbox::Phone(TAG_B), b"x"),
            Err(PushError::TooManyPhones)
        );
        // Idle inboxes make room for a new phone
        for m in lock(&room.phones).values() {
            *lock(&m.last_used) = Instant::now() - 2 * PHONE_INBOX_IDLE;
        }
        room.push(Inbox::Phone(TAG_B), b"x").unwrap();
    }

    #[test]
    fn idle_and_old_rooms_are_reaped() {
        let rooms = Rooms::new(10, 10, QUEUE_BUDGET);
        let created = rooms.create(IP).unwrap();
        let (minute, hour) = (Duration::from_secs(60), Duration::from_secs(3600));
        assert_eq!(rooms.reap(Instant::now(), minute, hour), 0);
        let later = Instant::now() + 2 * minute;
        assert_eq!(rooms.reap(later, minute, hour), 1);
        assert!(rooms
            .open(&created.id, Role::Phone, &created.phone_secret)
            .is_none());
        rooms.create(IP).unwrap();
        assert_eq!(rooms.reap(Instant::now() + 2 * hour, 10 * hour, hour), 1);
        assert!(rooms.is_empty());
    }
}
