// node --test server/static-tests: the phone's JS against protocol/test-vectors.json
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { PROTOCOL_VERSION, encode, decode, sealInner, openInner, parseRoom, parseAck } from '../static/wire.js';

const vectors = JSON.parse(readFileSync(new URL('../../protocol/test-vectors.json', import.meta.url)));
const hex = (s) => Uint8Array.from(Buffer.from(s, 'hex'));
const toHex = (b) => Buffer.from(b).toString('hex');

test('envelope matches the Rust encoding', () => {
    const v = vectors.envelope;
    const bytes = encode([{ text: v.text }, { binary: hex(v.binary) }]);
    assert.equal(toHex(bytes), v.encoded);
    const frames = decode(hex(v.encoded));
    assert.equal(frames[0].text, v.text);
    assert.equal(toHex(frames[1].binary), v.binary);
});

test('truncated envelope is rejected', () => {
    assert.throws(() => decode(hex(vectors.envelope.encoded).slice(0, -1)));
});

test('inner plaintext matches the Rust encoding', () => {
    const v = vectors.inner;
    assert.equal(toHex(sealInner(v.seq, v.op, hex(v.payload))), v.encoded);
    const inner = openInner(hex(v.encoded));
    assert.deepEqual([inner.seq, inner.op, toHex(inner.payload)], [v.seq, v.op, v.payload]);
    assert.equal(openInner(new Uint8Array(8)), null);
});

test('a BOUND payload names the private room', () => {
    const bytes = (s) => new TextEncoder().encode(s);
    assert.deepEqual(parseRoom(bytes('AbC-9.s3cr_et')), { room: 'AbC-9', secret: 's3cr_et' });
    // Old desktops sent an empty BOUND; paths and extra parts are refused
    for (const bad of ['', 'room', 'a.b.c', 'a/b.c', '.secret', 'room.']) {
        assert.equal(parseRoom(bytes(bad)), null, bad);
    }
    assert.equal(parseRoom(Uint8Array.of(0xff, 0x2e, 0x61)), null);
});

test('protocol version matches the shared vector', () => {
    assert.equal(PROTOCOL_VERSION, vectors.protocol_version);
});

test('ACK payload is a u64 BE sequence number', () => {
    assert.equal(parseAck(Uint8Array.from([0, 0, 0, 0, 0, 0, 1, 2])), 258);
    assert.equal(parseAck(Uint8Array.from([1, 2])), null);
});
