// Wire format of the Plumed remote microphone, mirror of the Rust crate
// PhoneMic_server/protocol (src/lib.rs). Both are pinned by
// protocol/test-vectors.json (see wire.test.mjs): change them together.
// Keep PhoneMic_server/protocol_details.md in sync with any change here.

// Mirror of PROTOCOL_VERSION (protocol/src/lib.rs), pinned by test-vectors.json.
// Bump it with any wire, crypto or pairing change.
export const PROTOCOL_VERSION = '4.0.0';

export const OP = Object.freeze({
    START: 1,
    AUDIO: 2,
    END: 3,
    CANCEL: 4,
    BOUND: 0x10,
    RECEIVED: 0x11,
    DENIED: 0x12,
    REPLACED: 0x13,
});

export const MAX_FRAME = 256 * 1024;
const KIND_TEXT = 0;
const KIND_BINARY = 1;
const HEADER_LEN = 5;
const SEQ_LEN = 8;

const utf8 = new TextEncoder();
const fromUtf8 = new TextDecoder('utf-8', { fatal: true });

/** frames: [{text: string} | {binary: Uint8Array}] -> Uint8Array */
export function encode(frames) {
    const parts = frames.map((f) => (f.text !== undefined ? [KIND_TEXT, utf8.encode(f.text)] : [KIND_BINARY, f.binary]));
    const total = parts.reduce((n, [, bytes]) => n + HEADER_LEN + bytes.length, 0);
    const out = new Uint8Array(total);
    const view = new DataView(out.buffer);
    let at = 0;
    for (const [kind, bytes] of parts) {
        out[at] = kind;
        view.setUint32(at + 1, bytes.length);
        out.set(bytes, at + HEADER_LEN);
        at += HEADER_LEN + bytes.length;
    }
    return out;
}

/** Uint8Array -> [{text} | {binary}], throws on a malformed envelope. */
export function decode(bytes) {
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    const frames = [];
    let at = 0;
    while (at < bytes.length) {
        if (bytes.length - at < HEADER_LEN) throw new Error('truncated envelope');
        const kind = bytes[at];
        const len = view.getUint32(at + 1);
        const start = at + HEADER_LEN;
        if (len > MAX_FRAME || start + len > bytes.length) throw new Error('bad frame length');
        const body = bytes.slice(start, start + len);
        if (kind === KIND_TEXT) frames.push({ text: fromUtf8.decode(body) });
        else if (kind === KIND_BINARY) frames.push({ binary: body });
        else throw new Error(`unknown frame kind ${kind}`);
        at = start + len;
    }
    return frames;
}

/** Plaintext of an encrypted frame: seq(u64 BE) || op(u8) || payload. */
export function sealInner(seq, op, payload = new Uint8Array(0)) {
    const out = new Uint8Array(SEQ_LEN + 1 + payload.length);
    new DataView(out.buffer).setBigUint64(0, BigInt(seq));
    out[SEQ_LEN] = op;
    out.set(payload, SEQ_LEN + 1);
    return out;
}

/** -> {seq, op, payload} or null when too short. */
export function openInner(plain) {
    if (plain.length < SEQ_LEN + 1) return null;
    const view = new DataView(plain.buffer, plain.byteOffset, plain.byteLength);
    return {
        seq: Number(view.getBigUint64(0)),
        op: plain[SEQ_LEN],
        payload: plain.slice(SEQ_LEN + 1),
    };
}

/** Payload of OP.START. */
export function u32be(n) {
    const out = new Uint8Array(4);
    new DataView(out.buffer).setUint32(0, n);
    return out;
}

const TOKEN = /^[A-Za-z0-9_-]+$/;

/**
 * The room a BOUND payload names, `<room id>.<phone secret>` in ASCII (the
 * private room the desktop opened for this phone), or null when malformed.
 * Example: bytes of "AbC9.s3cr_et" -> {room: 'AbC9', secret: 's3cr_et'}.
 */
export function parseRoom(payload) {
    let text;
    try {
        text = fromUtf8.decode(payload);
    } catch {
        return null;
    }
    const parts = text.split('.');
    if (parts.length !== 2 || !parts.every((p) => TOKEN.test(p))) return null;
    return { room: parts[0], secret: parts[1] };
}
