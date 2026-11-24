import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Outbox, RESEND_AFTER_MS, MAX_BYTES } from '../static/outbox.js';

const p = (n) => new Uint8Array([n]);

test('acknowledged packets are forgotten, the rest is resent once overdue', () => {
    const box = new Outbox();
    [1, 2, 3].forEach((seq) => box.add(seq, p(seq), 0));
    box.ack(1);
    assert.deepEqual(box.due(RESEND_AFTER_MS - 1), []);
    assert.deepEqual(box.due(RESEND_AFTER_MS), [p(2), p(3)]);
    // Just resent: not again right away
    assert.deepEqual(box.due(RESEND_AFTER_MS + 1), []);
    box.ack(3);
    assert.equal(box.bytes, 0);
    assert.deepEqual(box.due(10 * RESEND_AFTER_MS), []);
});

test('an unanswered outbox is dropped past its cap', () => {
    const box = new Outbox();
    box.add(1, new Uint8Array(MAX_BYTES), 0);
    assert.equal(box.items.length, 1);
    box.add(2, p(2), 0);
    assert.deepEqual([box.items.length, box.bytes], [0, 0]);
});
