// Energy based voice activity detection for the hands-free mode: the phone
// listens continuously and only sends the utterances, silence never leaves it.
//
// 20 ms frames, RMS level in dBFS compared to an adaptive noise floor. Example
// with the defaults: in a room at -55 dBFS a frame is speech above -45 dBFS;
// an utterance starts after 3 speech frames (60 ms) and carries the 400 ms
// heard before it, it ends after 900 ms of silence, and it is cancelled when
// it held less than 250 ms of speech (a cough, a door).

export const VAD_DEFAULTS = Object.freeze({
    frameMs: 20,
    initialFloorDb: -60,
    floorDownAlpha: 0.2,
    floorUpAlpha: 0.005,
    marginDb: 10,
    minSpeechDb: -50,
    onsetFrames: 3,
    prerollMs: 400,
    hangoverMs: 900,
    minSpeechMs: 250,
    maxUtteranceMs: 60_000,
});

const SILENCE_DB = -100;

export function frameDb(frame) {
    let sum = 0;
    for (const s of frame) sum += s * s;
    const rms = Math.sqrt(sum / frame.length);
    return rms > 0 ? Math.max(SILENCE_DB, 20 * Math.log10(rms)) : SILENCE_DB;
}

function concat(chunks) {
    const out = new Float32Array(chunks.reduce((n, c) => n + c.length, 0));
    let at = 0;
    for (const c of chunks) {
        out.set(c, at);
        at += c.length;
    }
    return out;
}

/**
 * push(samples) returns events, in order:
 *   {type: 'start', samples}  utterance begins, samples = pre-roll + onset
 *   {type: 'audio', samples}  more of the utterance
 *   {type: 'end'}             utterance complete, transcribe it
 *   {type: 'cancel'}          utterance was noise, drop it
 */
export class Vad {
    constructor(sampleRate, options = {}) {
        this.o = { ...VAD_DEFAULTS, ...options };
        this.frameLen = Math.round((sampleRate * this.o.frameMs) / 1000);
        this.pending = new Float32Array(this.frameLen);
        this.pendingLen = 0;
        this.floorDb = this.o.initialFloorDb;
        this.levelDb = SILENCE_DB;
        this.ring = [];
        this.ringMax = Math.round(this.o.prerollMs / this.o.frameMs);
        this.streak = 0;
        this.active = false;
    }

    get speaking() {
        return this.active;
    }

    push(samples) {
        const events = [];
        let i = 0;
        while (i < samples.length) {
            const n = Math.min(samples.length - i, this.frameLen - this.pendingLen);
            this.pending.set(samples.subarray(i, i + n), this.pendingLen);
            this.pendingLen += n;
            i += n;
            if (this.pendingLen === this.frameLen) {
                this.frame(this.pending.slice(), events);
                this.pendingLen = 0;
            }
        }
        return events;
    }

    /** Ends the current utterance (mic closed, mode changed). */
    flush() {
        if (!this.active) return [];
        const events = [];
        this.finish(events);
        return events;
    }

    frame(frame, events) {
        const o = this.o;
        const db = frameDb(frame);
        this.levelDb = db;
        const speech = db > Math.max(this.floorDb + o.marginDb, o.minSpeechDb);
        // The floor follows quiet frames quickly and loud ones slowly, and
        // never learns from speech (a long dictation would raise it).
        if (db < this.floorDb) this.floorDb += o.floorDownAlpha * (db - this.floorDb);
        else if (!speech) this.floorDb += o.floorUpAlpha * (db - this.floorDb);

        if (this.active) {
            events.push({ type: 'audio', samples: frame });
            this.durationMs += o.frameMs;
            if (speech) {
                this.speechMs += o.frameMs;
                this.silenceMs = 0;
            } else {
                this.silenceMs += o.frameMs;
            }
            if (this.silenceMs >= o.hangoverMs) this.finish(events);
            else if (this.durationMs >= o.maxUtteranceMs) this.split(events);
            return;
        }

        this.ring.push(frame);
        if (this.ring.length > this.ringMax) this.ring.shift();
        this.streak = speech ? this.streak + 1 : 0;
        if (this.streak >= o.onsetFrames) {
            events.push({ type: 'start', samples: concat(this.ring) });
            this.begin(this.streak * o.frameMs, this.ring.length * o.frameMs);
            this.ring = [];
        }
    }

    begin(speechMs, durationMs) {
        this.active = true;
        this.speechMs = speechMs;
        this.silenceMs = 0;
        this.durationMs = durationMs;
    }

    finish(events) {
        events.push({ type: this.speechMs >= this.o.minSpeechMs ? 'end' : 'cancel' });
        this.active = false;
        this.streak = 0;
    }

    /** A very long utterance is cut so the desktop transcribes as it goes. */
    split(events) {
        events.push({ type: 'end' });
        events.push({ type: 'start', samples: new Float32Array(0) });
        this.begin(0, 0);
    }
}
