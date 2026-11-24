import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Backoff } from '../static/relay.js';

test('backoff doubles up to its cap and starts over after a reset', () => {
    const b = new Backoff();
    assert.deepEqual([1, 2, 3, 4, 5, 6].map(() => b.next()), [1, 2, 4, 8, 10, 10]);
    b.reset();
    assert.equal(b.next(), 1);
});
