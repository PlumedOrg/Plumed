import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Vad, frameDb } from '../static/vad.js';

const RATE = 16000;
const ms = (n) => (RATE * n) / 1000;

/** Sine at the given dBFS RMS level (silence below -100). */
function tone(durationMs, db) {
    const amp = db <= -100 ? 0 : Math.SQRT2 * 10 ** (db / 20);
    return Float32Array.from({ length: ms(durationMs) }, (_, i) => amp * Math.sin((2 * Math.PI * 440 * i) / RATE));
}

function run(vad, ...parts) {
    return parts.flatMap((p) => vad.push(p));
}

const types = (events) => events.map((e) => e.type).filter((t) => t !== 'audio');

test('frameDb of a -20 dBFS sine', () => {
    assert.ok(Math.abs(frameDb(tone(20, -20)) + 20) < 0.1);
});

test('speech between silences is one utterance with pre-roll', () => {
    const events = run(new Vad(RATE), tone(1000, -70), tone(1000, -20), tone(1500, -70));
    assert.deepEqual(types(events), ['start', 'end']);
    // 400 ms of pre-roll, which includes the 3 onset frames
    assert.equal(events[0].samples.length, ms(400));
    const sent = events.filter((e) => e.type !== 'end').reduce((n, e) => n + e.samples.length, 0);
    // 1000 ms of speech + the pre-roll before the onset + 900 ms of hangover
    assert.equal(sent, ms(1000 - 60 + 400 + 900));
});

test('a short blip is cancelled', () => {
    assert.deepEqual(types(run(new Vad(RATE), tone(1000, -70), tone(100, -20), tone(1500, -70))), ['start', 'cancel']);
});

test('silence and quiet noise send nothing', () => {
    assert.deepEqual(run(new Vad(RATE), tone(3000, -100), tone(3000, -55)), []);
});

test('a long monologue is split every 60 s', () => {
    const events = run(new Vad(RATE), tone(500, -70), tone(130_000, -20), tone(1500, -70));
    assert.deepEqual(types(events), ['start', 'end', 'start', 'end', 'start', 'end']);
});

test('flush ends an utterance in progress', () => {
    const vad = new Vad(RATE);
    run(vad, tone(500, -70), tone(500, -20));
    assert.deepEqual(vad.flush(), [{ type: 'end' }]);
    assert.deepEqual(vad.flush(), []);
});
