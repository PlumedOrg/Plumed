// HTTPS long-poll transport to the relay (see the protocol crate for the API).
// The phone posts to the desktop inbox and polls its own inbox, named by a
// random tag so phones of the same room never take each other's frames.
// Network errors are retried forever with backoff: a phone that loses the
// network picks up where it stopped. Duplicated POSTs are harmless, the
// desktop drops frames whose sequence number it has already seen.

import { encode, decode } from './wire.js';

const SECRET_HEADER = 'x-room-secret';
const MAX_BODY = 1024 * 1024;
const MAX_BACKOFF_S = 10;

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** Exponential retry delay in seconds: 1, 2, 4, 8, 10, 10... until reset. */
export class Backoff {
    constructor() {
        this.reset();
    }

    reset() {
        this.delay = 1;
    }

    /** The delay to wait now; the next one doubles. */
    next() {
        const delay = this.delay;
        this.delay = Math.min(delay * 2, MAX_BACKOFF_S);
        return delay;
    }
}

export class Relay {
    /** handlers: {onFrames(frames) async, onClosed(), onOnline(boolean)} */
    constructor(room, secret, inbox, handlers) {
        const base = `/api/rooms/${encodeURIComponent(room)}`;
        this.postUrl = `${base}/desktop`;
        this.pollUrl = `${base}/phone/${encodeURIComponent(inbox)}`;
        this.secret = secret;
        this.handlers = handlers;
        this.queue = [];
        this.sending = false;
        this.running = true;
        this.abort = new AbortController();
        this.online = true;
    }

    start() {
        this.pollLoop();
    }

    stop() {
        this.running = false;
        this.abort.abort();
    }

    send(frames) {
        this.queue.push(encode(frames));
        this.flush();
    }

    setOnline(online) {
        if (online === this.online) return;
        this.online = online;
        this.handlers.onOnline(online);
    }

    closed() {
        if (!this.running) return;
        this.stop();
        this.handlers.onClosed();
    }

    request(method, url, body) {
        return fetch(url, {
            method,
            body,
            cache: 'no-store',
            signal: this.abort.signal,
            headers: { [SECRET_HEADER]: this.secret },
        });
    }

    /** Envelopes queued while a POST is in flight leave together in the next one. */
    nextBatch() {
        let n = 0;
        let size = 0;
        while (n < this.queue.length && (n === 0 || size + this.queue[n].length <= MAX_BODY)) {
            size += this.queue[n].length;
            n += 1;
        }
        const body = new Uint8Array(size);
        let at = 0;
        for (const part of this.queue.slice(0, n)) {
            body.set(part, at);
            at += part.length;
        }
        return { n, body };
    }

    async flush() {
        if (this.sending) return;
        this.sending = true;
        const backoff = new Backoff();
        while (this.running && this.queue.length > 0) {
            const { n, body } = this.nextBatch();
            let wait = 0;
            try {
                const r = await this.request('POST', this.postUrl, body);
                switch (r.status) {
                    case 204:
                    case 400: // malformed: a bug, retrying would block the queue
                        this.queue.splice(0, n);
                        this.setOnline(true);
                        backoff.reset();
                        continue;
                    case 404:
                        this.closed();
                        break;
                    case 429:
                        wait = Number(r.headers.get('retry-after')) || 1;
                        break;
                    default:
                        wait = backoff.next();
                }
            } catch {
                this.setOnline(false);
                wait = backoff.next();
            }
            await sleep(wait * 1000);
        }
        this.sending = false;
    }

    async pollLoop() {
        const backoff = new Backoff();
        while (this.running) {
            try {
                const r = await this.request('GET', this.pollUrl);
                if (r.status === 200 || r.status === 204) {
                    this.setOnline(true);
                    backoff.reset();
                    if (r.status === 200) await this.deliver(new Uint8Array(await r.arrayBuffer()));
                    continue;
                }
                if (r.status === 404) {
                    this.closed();
                    return;
                }
            } catch {
                if (!this.running) return;
                this.setOnline(false);
            }
            await sleep(backoff.next() * 1000);
        }
    }

    async deliver(bytes) {
        let frames;
        try {
            frames = decode(bytes);
        } catch {
            return;
        }
        await this.handlers.onFrames(frames);
    }
}
