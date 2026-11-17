import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import * as c from '../static/crypto.js';

const v = JSON.parse(readFileSync(new URL('../../protocol/test-vectors.json', import.meta.url))).crypto;
const hex = (s) => Uint8Array.from(Buffer.from(s, 'hex'));
const toHex = (b) => Buffer.from(b).toString('hex');
const b64url = (b) => Buffer.from(b).toString('base64url');

/** The vectors use the private scalars 0x01 * 32 (desktop) and 0x02 * 32 (phone). */
async function phonePrivateKey() {
    const raw = c.fromB64(v.phone_public);
    const jwk = {
        kty: 'EC',
        crv: 'P-256',
        d: b64url(new Uint8Array(32).fill(2)),
        x: b64url(raw.slice(1, 33)),
        y: b64url(raw.slice(33)),
    };
    return crypto.subtle.importKey('jwk', jwk, { name: 'ECDH', namedCurve: 'P-256' }, false, ['deriveBits']);
}

test('ECDH and HKDF match the vectors', async () => {
    const shared = await c.sharedSecret(await phonePrivateKey(), c.fromB64(v.desktop_public));
    assert.equal(toHex(shared), v.shared_x);
    const desktop = c.fromB64(v.desktop_public);
    const phone = c.fromB64(v.phone_public);
    const label = (s) => new TextEncoder().encode(s);
    assert.equal(toHex(await c.hkdf(shared, label('phone-to-desktop'), desktop, phone)), v.key_phone_to_desktop);
    assert.equal(toHex(await c.hkdf(shared, label('desktop-to-phone'), desktop, phone)), v.key_desktop_to_phone);
});

test('phone keys: we seal with send, a reflected packet does not open', async () => {
    const shared = await c.sharedSecret(await phonePrivateKey(), c.fromB64(v.desktop_public));
    const keys = await c.phoneKeys(shared, c.fromB64(v.desktop_public), c.fromB64(v.phone_public));
    assert.equal(toHex(await c.open(keys.send, hex(v.packet))), v.packet_inner);
    await assert.rejects(c.open(keys.recv, hex(v.packet)));
});

test('opens the reference packet and round-trips', async () => {
    const key = await c.aesKey(hex(v.key_phone_to_desktop));
    assert.equal(toHex(await c.open(key, hex(v.packet))), v.packet_inner);
    const sealed = await c.seal(key, hex(v.packet_inner));
    assert.equal(toHex(await c.open(key, sealed)), v.packet_inner);
    const forged = hex(v.packet);
    forged[20] ^= 1;
    await assert.rejects(c.open(key, forged));
});

test('fingerprint matches the vectors', async () => {
    assert.equal(await c.fingerprint(c.fromB64(v.desktop_public), c.fromB64(v.phone_public)), v.fingerprint);
});

test('base64url of the QR link', () => {
    const raw = c.fromB64(v.desktop_public);
    assert.deepEqual(c.fromB64url(b64url(raw)), raw);
});

test('base64url round trip, no padding', () => {
    for (const len of [1, 2, 3, 16, 65]) {
        const bytes = Uint8Array.from({ length: len }, (_, i) => (i * 37 + 251) % 256);
        assert.equal(c.toB64url(bytes), b64url(bytes));
        assert.deepEqual(c.fromB64url(c.toB64url(bytes)), bytes);
    }
});
