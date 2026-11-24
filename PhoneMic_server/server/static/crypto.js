// End-to-end crypto of the phone, mirror of src-tauri/src/remote_mic/crypto.rs
// (ECDH P-256, HKDF-SHA256, AES-256-GCM, hello proof). Both are pinned
// by protocol/test-vectors.json (see crypto.test.mjs): change them together.
// Keep PhoneMic_server/protocol_details.md in sync with any change here.
// One key per direction, bound to both public keys: a packet cannot be
// reflected back to its sender, nor replayed into another pairing. The HKDF
// input is the ECDH x coordinate followed by the QR code's pairing secret, so
// the relay (which never sees the URL fragment) can derive nothing.

import { PROTOCOL_VERSION } from './wire.js';

const HKDF_SALT = new TextEncoder().encode(`Plumed-RemoteMic-${PROTOCOL_VERSION}`);
const PHONE_TO_DESKTOP = new TextEncoder().encode('phone-to-desktop');
const DESKTOP_TO_PHONE = new TextEncoder().encode('desktop-to-phone');
const HELLO_PROOF = new TextEncoder().encode('hello-proof');
const IV_LEN = 12;
const subtle = globalThis.crypto.subtle;

export function toB64(bytes) {
    let s = '';
    for (const b of bytes) s += String.fromCharCode(b);
    return btoa(s);
}

export function fromB64(s) {
    const bin = atob(s);
    return Uint8Array.from(bin, (c) => c.charCodeAt(0));
}

/** bytes -> base64url without padding. */
export function toB64url(bytes) {
    return toB64(bytes).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

/** base64url without padding (the QR link) -> bytes. */
export function fromB64url(s) {
    const std = s.replace(/-/g, '+').replace(/_/g, '/');
    return fromB64(std + '='.repeat((4 - (std.length % 4)) % 4));
}

/** -> {privateKey, publicRaw (65 bytes)} */
export async function generateKeyPair() {
    const pair = await subtle.generateKey({ name: 'ECDH', namedCurve: 'P-256' }, false, ['deriveBits']);
    const publicRaw = new Uint8Array(await subtle.exportKey('raw', pair.publicKey));
    return { privateKey: pair.privateKey, publicRaw };
}

export async function sharedSecret(privateKey, peerRaw) {
    const peer = await subtle.importKey('raw', peerRaw, { name: 'ECDH', namedCurve: 'P-256' }, false, []);
    return new Uint8Array(await subtle.deriveBits({ name: 'ECDH', public: peer }, privateKey, 256));
}

export function concat(...parts) {
    const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
    let at = 0;
    for (const p of parts) {
        out.set(p, at);
        at += p.length;
    }
    return out;
}

/** HKDF-SHA256(ikm = shared x || pairing secret, info = label || desktop || phone) -> 32 bytes. */
export async function hkdf(ikmBytes, label, desktopRaw, phoneRaw) {
    const info = concat(label, desktopRaw, phoneRaw);
    const ikm = await subtle.importKey('raw', ikmBytes, 'HKDF', false, ['deriveBits']);
    const bits = await subtle.deriveBits({ name: 'HKDF', hash: 'SHA-256', salt: HKDF_SALT, info }, ikm, 256);
    return new Uint8Array(bits);
}

/** The phone's keys: `send` seals phone -> desktop, `recv` opens desktop -> phone. */
export async function phoneKeys(ikm, desktopRaw, phoneRaw) {
    const [send, recv] = await Promise.all([
        hkdf(ikm, PHONE_TO_DESKTOP, desktopRaw, phoneRaw).then(aesKey),
        hkdf(ikm, DESKTOP_TO_PHONE, desktopRaw, phoneRaw).then(aesKey),
    ]);
    return { send, recv };
}

/** What the hello carries to prove the phone read the QR code's pairing secret. */
export function helloProof(ikm, desktopRaw, phoneRaw) {
    return hkdf(ikm, HELLO_PROOF, desktopRaw, phoneRaw);
}

export function aesKey(raw) {
    return subtle.importKey('raw', raw, 'AES-GCM', false, ['encrypt', 'decrypt']);
}

/** -> IV(12) || ciphertext || tag(16) */
export async function seal(key, plain) {
    const iv = globalThis.crypto.getRandomValues(new Uint8Array(IV_LEN));
    const sealed = new Uint8Array(await subtle.encrypt({ name: 'AES-GCM', iv }, key, plain));
    const out = new Uint8Array(IV_LEN + sealed.length);
    out.set(iv);
    out.set(sealed, IV_LEN);
    return out;
}

/** Throws when the packet does not authenticate. */
export async function open(key, packet) {
    const iv = packet.slice(0, IV_LEN);
    return new Uint8Array(await subtle.decrypt({ name: 'AES-GCM', iv }, key, packet.slice(IV_LEN)));
}
