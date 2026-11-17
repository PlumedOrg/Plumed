// Phone page of the Plumed remote microphone.
//
// The QR code opens {origin}/#{room}.{secret}.{desktop public key}. The phone
// sends its own public key in a "hello", the desktop user checks the pairing
// code on both screens and confirms, then every frame is AES-GCM encrypted:
// IV || AES-GCM(seq || op || payload). The relay never sees the key.

import { OP, sealInner, openInner, u32be } from './wire.js';
import * as crypto from './crypto.js';
import { Relay } from './relay.js';
import { Capture, decodeFile, toPcm16, TARGET_RATE } from './audio.js';
import { Vad } from './vad.js';
import { t, setLang, applyStatic } from './i18n.js';

const PUSH_TO_TALK_MS = 500;
// The desktop keeps at most 19.2M samples per recording (20 min at 16 kHz)
const MAX_UTTERANCE_SECONDS = 20 * 60;
const FILE_CHUNK_SAMPLES = TARGET_RATE;
const MODE_KEY = 'plumed-mode';

const $ = (id) => document.getElementById(id);

/** Everything the page shows, rendered again on a language change. */
const ui = { view: 'loading', error: null, state: 'state_idle', notice: null };

const session = {
    relay: null,
    storeKey: null,
    key: null,
    rawKey: null,
    bound: false,
    seqOut: 0,
    seqIn: 0,
    chain: Promise.resolve(),
};

const rec = {
    mode: localStorage.getItem(MODE_KEY) === 'handsfree' ? 'handsfree' : 'manual',
    capture: null,
    opening: false,
    // manual: an utterance is open; hands-free: the mic is listening
    active: false,
    paused: false,
    vad: null,
    samples: 0,
    sendingFile: false,
    pressedAt: 0,
    stopOnRelease: false,
};

// ---------------------------------------------------------------- rendering

function render() {
    applyStatic();
    for (const view of ['loading', 'error', 'pairing', 'connected']) $(`view-${view}`).hidden = ui.view !== view;
    $('error-text').textContent = ui.error === null ? '' : t(ui.error.key, ui.error.vars);
    $('mode-manual').setAttribute('aria-pressed', String(rec.mode === 'manual'));
    $('mode-handsfree').setAttribute('aria-pressed', String(rec.mode === 'handsfree'));
    $('mode-hint').textContent = t(rec.mode === 'manual' ? 'hint_manual' : 'hint_handsfree');
    const labels = rec.mode === 'manual' ? ['main_start', 'main_stop'] : ['main_listen', 'main_stop_listening'];
    $('main').textContent = t(labels[rec.active ? 1 : 0]);
    $('main').classList.toggle('on', rec.active && !rec.paused);
    $('pause').hidden = !rec.active;
    $('pause').textContent = t(rec.paused ? 'resume' : 'pause');
    $('state').textContent = t(ui.state);
    $('notice').textContent = ui.notice === null ? '' : t(ui.notice.key, ui.notice.vars);
    $('notice').hidden = ui.notice === null;
}

function show(view) {
    ui.view = view;
    render();
}

function setState(state) {
    ui.state = state;
    render();
}

function notice(key, vars = {}) {
    ui.notice = { key, vars };
    render();
}

/** Terminal error: everything stops, the user has to scan a new QR code. */
function fail(key, vars = {}) {
    stopMic();
    session.relay?.stop();
    if (session.storeKey !== null) sessionStorage.removeItem(session.storeKey);
    ui.error = { key, vars };
    show('error');
}

function showLevel(db) {
    const percent = Math.max(0, Math.min(100, ((db + 60) / 60) * 100));
    $('level-fill').style.width = `${percent}%`;
}

// ----------------------------------------------------------------- session

/** Survives a reload (sessionStorage, this tab only) so the phone reconnects without a new pairing. */
function save() {
    if (!session.bound) return;
    const saved = { key: crypto.toB64(session.rawKey), seqOut: session.seqOut, seqIn: session.seqIn };
    sessionStorage.setItem(session.storeKey, JSON.stringify(saved));
}

/** Encrypts and queues one frame. Chained so the sequence numbers leave in order. */
function sendOp(op, payload) {
    session.chain = session.chain.then(async () => {
        session.seqOut += 1;
        const packet = await crypto.seal(session.key, sealInner(session.seqOut, op, payload));
        save();
        session.relay.send([{ binary: packet }]);
    });
    return session.chain;
}

async function onFrames(frames) {
    for (const frame of frames) {
        if (frame.text !== undefined) {
            onText(frame.text);
            continue;
        }
        let plain;
        try {
            plain = await crypto.open(session.key, frame.binary);
        } catch {
            continue; // for another phone of the same room, or forged
        }
        const inner = openInner(plain);
        if (inner === null || inner.seq <= session.seqIn) continue; // replayed
        session.seqIn = inner.seq;
        switch (inner.op) {
            case OP.BOUND:
                if (!session.bound) {
                    session.bound = true;
                    showConnected();
                }
                break;
            case OP.RECEIVED:
                setState('state_received');
                break;
            default:
                break;
        }
        save();
    }
}

/** Unencrypted answers to the hello, meaningless once paired. */
function onText(text) {
    if (session.bound) return;
    let msg;
    try {
        msg = JSON.parse(text);
    } catch {
        return;
    }
    switch (msg.type) {
        case 'denied':
            fail('denied');
            break;
        case 'stale':
            fail('stale');
            break;
        default:
            break;
    }
}

function parseLink() {
    const parts = location.hash.replace(/^#/, '').split('.');
    if (parts.length !== 3 || parts.some((p) => p.length === 0)) return null;
    try {
        return { room: parts[0], secret: parts[1], desktopRaw: crypto.fromB64url(parts[2]) };
    } catch {
        return null;
    }
}

async function restore(storeKey) {
    const raw = sessionStorage.getItem(storeKey);
    if (raw === null) return false;
    try {
        const saved = JSON.parse(raw);
        session.rawKey = crypto.fromB64(saved.key);
        session.key = await crypto.aesKey(session.rawKey);
        session.seqOut = saved.seqOut;
        session.seqIn = saved.seqIn;
        session.bound = true;
        return true;
    } catch {
        sessionStorage.removeItem(storeKey);
        return false;
    }
}

async function pair(desktopRaw) {
    const mine = await crypto.generateKeyPair();
    session.rawKey = await crypto.hkdf(await crypto.sharedSecret(mine.privateKey, desktopRaw));
    session.key = await crypto.aesKey(session.rawKey);
    $('fingerprint').textContent = await crypto.fingerprint(desktopRaw, mine.publicRaw);
    const hello = { type: 'hello', key: crypto.toB64(mine.publicRaw), to: crypto.toB64(desktopRaw) };
    session.relay.send([{ text: JSON.stringify(hello) }]);
    show('pairing');
}

function showConnected() {
    show('connected');
    keepAwake();
}

async function start() {
    render();
    const link = parseLink();
    if (link === null) return fail('no_link');
    if (globalThis.crypto?.subtle === undefined) return fail('unsupported');
    session.storeKey = `plumed-mic:${link.room}`;
    session.relay = new Relay(link.room, link.secret, {
        onFrames,
        onClosed: () => fail('closed'),
        onOnline: (online) => {
            $('banner').hidden = online;
        },
    });
    const restored = await restore(session.storeKey);
    session.relay.start();
    if (restored) showConnected();
    else await pair(link.desktopRaw);
}

// --------------------------------------------------------------- recording

function sendAudio(samples) {
    if (samples.length === 0) return;
    rec.samples += samples.length;
    sendOp(OP.AUDIO, toPcm16(samples));
}

function beginUtterance(rate) {
    rec.samples = 0;
    sendOp(OP.START, u32be(rate));
}

function onChunk(samples) {
    if (rec.paused || !rec.active) return;
    if (rec.mode === 'manual') {
        rec.vad.push(samples);
        showLevel(rec.vad.levelDb);
        sendAudio(samples);
        if (rec.samples >= MAX_UTTERANCE_SECONDS * rec.capture.rate) stopManual();
        return;
    }
    for (const event of rec.vad.push(samples)) onVadEvent(event);
    showLevel(rec.vad.levelDb);
}

function onVadEvent(event) {
    switch (event.type) {
        case 'start':
            beginUtterance(rec.capture.rate);
            sendAudio(event.samples);
            setState('state_speech');
            break;
        case 'audio':
            sendAudio(event.samples);
            break;
        case 'end':
            sendOp(OP.END);
            setState('state_listening');
            break;
        case 'cancel':
            sendOp(OP.CANCEL);
            setState('state_listening');
            break;
        default:
            break;
    }
}

async function openMic() {
    if (rec.capture !== null) return true;
    rec.opening = true;
    try {
        rec.capture = await Capture.open(onChunk);
        // In manual mode the VAD only measures the level for the meter
        rec.vad = new Vad(rec.capture.rate);
        return true;
    } catch (e) {
        notice('mic_error', { error: e instanceof Error ? e.message : String(e) });
        return false;
    } finally {
        rec.opening = false;
    }
}

function closeMic() {
    if (rec.mode === 'handsfree' && rec.vad !== null) rec.vad.flush().forEach(onVadEvent);
    rec.capture?.close();
    rec.capture = null;
    showLevel(-100);
}

function stopMic() {
    closeMic();
    rec.active = false;
    rec.paused = false;
}

async function startManual() {
    if (!(await openMic())) return;
    beginUtterance(rec.capture.rate);
    rec.active = true;
    setState('state_recording');
}

function stopManual() {
    if (!rec.active) return;
    sendOp(OP.END);
    stopMic();
    setState('state_idle');
}

async function startListening() {
    if (!(await openMic())) return;
    rec.active = true;
    setState('state_listening');
}

function stopListening() {
    stopMic();
    setState('state_idle');
}

function onPress() {
    if (rec.sendingFile || rec.opening) return;
    rec.pressedAt = performance.now();
    rec.stopOnRelease = rec.active;
    if (rec.active) return;
    if (rec.mode === 'manual') startManual();
    else startListening();
}

function onRelease() {
    if (!rec.active && !rec.opening) return;
    const held = performance.now() - rec.pressedAt >= PUSH_TO_TALK_MS;
    // A tap toggles; holding the button is push-to-talk (manual mode only)
    if (rec.stopOnRelease || (held && rec.mode === 'manual')) {
        if (rec.mode === 'manual') stopManual();
        else stopListening();
    }
}

async function togglePause() {
    if (!rec.active) return;
    if (rec.paused) {
        rec.paused = false;
        if (!(await openMic())) rec.paused = true;
    } else {
        // Hands-free: closing the mic also ends the utterance in progress
        closeMic();
        rec.paused = true;
    }
    setState(rec.paused ? 'state_paused' : rec.mode === 'manual' ? 'state_recording' : 'state_listening');
}

function setMode(mode) {
    if (rec.mode === mode) return;
    if (rec.active) {
        if (rec.mode === 'manual') stopManual();
        else stopListening();
    }
    rec.mode = mode;
    localStorage.setItem(MODE_KEY, mode);
    render();
}

async function sendFile(file) {
    if (rec.active || rec.sendingFile) return notice('busy');
    rec.sendingFile = true;
    setState('state_sending_file');
    try {
        const samples = await decodeFile(file, MAX_UTTERANCE_SECONDS);
        beginUtterance(TARGET_RATE);
        for (let at = 0; at < samples.length; at += FILE_CHUNK_SAMPLES) {
            sendAudio(samples.subarray(at, at + FILE_CHUNK_SAMPLES));
        }
        await sendOp(OP.END);
        setState('state_idle');
    } catch (e) {
        setState('state_idle');
        notice(e instanceof RangeError ? 'file_too_long' : 'file_error');
    } finally {
        rec.sendingFile = false;
    }
}

// ---------------------------------------------------------- screen wake lock

let wakeLock = null;

/** Keeps the screen on while connected: a sleeping phone stops the mic. */
async function keepAwake() {
    if (!('wakeLock' in navigator)) return notice('no_wakelock');
    if (document.visibilityState !== 'visible' || (wakeLock !== null && !wakeLock.released)) return;
    try {
        wakeLock = await navigator.wakeLock.request('screen');
    } catch {
        // Refused (battery saver): the page still works while the screen is on
    }
}

document.addEventListener('visibilitychange', () => {
    // The lock is released when the page is hidden, take it again on return
    if (ui.view === 'connected') keepAwake();
});

// ------------------------------------------------------------------ wiring

$('main').addEventListener('pointerdown', (e) => {
    e.preventDefault();
    onPress();
});
$('main').addEventListener('pointerup', onRelease);
$('main').addEventListener('pointercancel', onRelease);
$('main').addEventListener('contextmenu', (e) => e.preventDefault());
$('pause').addEventListener('click', togglePause);
$('mode-manual').addEventListener('click', () => setMode('manual'));
$('mode-handsfree').addEventListener('click', () => setMode('handsfree'));
$('file').addEventListener('change', (e) => {
    const file = e.target.files?.[0];
    e.target.value = '';
    if (file !== undefined) sendFile(file);
});
for (const button of document.querySelectorAll('[data-lang]')) {
    button.addEventListener('click', () => {
        setLang(button.dataset.lang);
        render();
    });
}

start();
