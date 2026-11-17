// End-to-end crypto of the phone, mirror of src-tauri/src/remote_mic/crypto.rs
// (ECDH P-256, HKDF-SHA256, AES-256-GCM, pairing fingerprint). Both are pinned
// by protocol/test-vectors.json (see crypto.test.mjs): change them together.

const HKDF_SALT = new TextEncoder().encode('Plumed-RemoteMic-v1');
const HKDF_INFO = new TextEncoder().encode('AES-GCM-256-key');
const IV_LEN = 12;
const FINGERPRINT_BYTES = 8;
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

/** HKDF-SHA256(shared x) -> 32-byte AES key. */
export async function hkdf(shared) {
    const ikm = await subtle.importKey('raw', shared, 'HKDF', false, ['deriveBits']);
    const bits = await subtle.deriveBits({ name: 'HKDF', hash: 'SHA-256', salt: HKDF_SALT, info: HKDF_INFO }, ikm, 256);
    return new Uint8Array(bits);
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

/** SHA-256(desktop || phone), first 16 uppercase hex digits grouped by 4: 524C-F8E8-D5A4-2D82 */
export async function fingerprint(desktopRaw, phoneRaw) {
    const both = new Uint8Array(desktopRaw.length + phoneRaw.length);
    both.set(desktopRaw);
    both.set(phoneRaw, desktopRaw.length);
    const digest = new Uint8Array(await subtle.digest('SHA-256', both));
    const hex = Array.from(digest.slice(0, FINGERPRINT_BYTES), (b) => b.toString(16).padStart(2, '0'))
        .join('')
        .toUpperCase();
    return hex.match(/.{4}/g).join('-');
}
