// The bound session, kept in IndexedDB so a page reload (or the phone killing
// a background tab) does not need a new scan of the QR code. Built with Claude Code.
//
// Stored record, example:
//   { room: 'R00M', secret: 'S3CRET', inbox: 'aB3...', keys: {send, recv},
//     seqOut: 1048576, seqIn: 12, expiresAt: 1763470000000 }
// The AES keys are non-extractable CryptoKeys: IndexedDB stores them as they
// are, script on the page can use them but never read their bytes. The record
// expires with the desktop's binding (BOUND_LIFETIME, 12 h) and is deleted on
// any terminal error (room closed, phone replaced...).

const DB = 'plumed';
const STORE = 'session';
const KEY = 'bound';

export const LIFETIME_MS = 12 * 3600 * 1000;
// Sequence numbers are reserved by blocks: the stored seqOut is always ahead
// of every number sent, so a reload never reuses one (the desktop would drop
// the frame as a replay). 2^20 frames is more than 12 h of audio chunks.
export const SEQ_BLOCK = 2 ** 20;

/** Whether `record` can resume a session at `now` (ms). */
export function resumable(record, now) {
    return (
        record !== null &&
        typeof record === 'object' &&
        typeof record.room === 'string' &&
        typeof record.secret === 'string' &&
        typeof record.inbox === 'string' &&
        record.keys?.send !== undefined &&
        record.keys?.recv !== undefined &&
        Number.isSafeInteger(record.seqOut) &&
        Number.isSafeInteger(record.seqIn) &&
        typeof record.expiresAt === 'number' &&
        now < record.expiresAt
    );
}

function request(req) {
    return new Promise((resolve, reject) => {
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
    });
}

let db = null;

function open() {
    if (db === null) {
        const req = indexedDB.open(DB, 1);
        req.onupgradeneeded = () => req.result.createObjectStore(STORE);
        db = request(req);
    }
    return db;
}

async function run(mode, action) {
    const tx = (await open()).transaction(STORE, mode);
    return request(action(tx.objectStore(STORE)));
}

/** The stored record, null when there is none or storage is unavailable. */
export async function load() {
    try {
        return (await run('readonly', (s) => s.get(KEY))) ?? null;
    } catch {
        return null;
    }
}

/** Best effort: without storage the session simply does not survive a reload. */
export async function save(record) {
    try {
        await run('readwrite', (s) => s.put(record, KEY));
    } catch {
        // private browsing or storage disabled
    }
}

export async function clear() {
    try {
        await run('readwrite', (s) => s.delete(KEY));
    } catch {
        // nothing stored
    }
}
