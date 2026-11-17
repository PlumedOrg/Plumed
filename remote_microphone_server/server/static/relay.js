// HTTPS long-poll transport to the relay (see the server's lib.rs for the API).
// Network errors are retried forever with backoff: a phone that loses the
// network picks up where it stopped. Duplicated POSTs are harmless, the
// desktop drops frames whose sequence number it has already seen.

import { encode, decode } from './wire.js';

const SECRET_HEADER = 'x-room-secret';
const MAX_BODY = 1024 * 1024;
const MAX_BACKOFF_S = 10;

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

export class Relay {
    /** handlers: {onFrames(frames) async, onClosed(), onOnline(boolean)} */
    constructor(room, secret, handlers) {
        this.url = `/api/rooms/${encodeURIComponent(room)}/phone`;
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

    request(method, body) {
        return fetch(this.url, {
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
        let backoff = 1;
        while (this.running && this.queue.length > 0) {
            const { n, body } = this.nextBatch();
            let wait = backoff;
            try {
                const r = await this.request('POST', body);
                switch (r.status) {
                    case 204:
                    case 400: // malformed: a bug, retrying would block the queue
                        this.queue.splice(0, n);
                        this.setOnline(true);
                        backoff = 1;
                        continue;
                    case 404:
                        this.closed();
                        break;
                    case 429:
                        wait = Number(r.headers.get('retry-after')) || 1;
                        break;
                    default:
                        backoff = Math.min(backoff * 2, MAX_BACKOFF_S);
                }
            } catch {
                this.setOnline(false);
                backoff = Math.min(backoff * 2, MAX_BACKOFF_S);
            }
            await sleep(wait * 1000);
        }
        this.sending = false;
    }

    async pollLoop() {
        let backoff = 1;
        while (this.running) {
            try {
                const r = await this.request('GET');
                if (r.status === 200 || r.status === 204) {
                    this.setOnline(true);
                    backoff = 1;
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
            await sleep(backoff * 1000);
            backoff = Math.min(backoff * 2, MAX_BACKOFF_S);
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
