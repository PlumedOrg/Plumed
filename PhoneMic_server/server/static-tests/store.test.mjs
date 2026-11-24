// Built with Claude Code.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { resumable } from '../static/store.js';

const NOW = 1_000_000;
const record = {
    room: 'R00M',
    secret: 'S3CRET',
    inbox: 'inbox',
    keys: { send: {}, recv: {} },
    seqOut: 2 ** 20,
    seqIn: 3,
    expiresAt: NOW + 1,
};

test('a stored session resumes until it expires', () => {
    assert.equal(resumable(record, NOW), true);
    assert.equal(resumable(record, NOW + 1), false);
});

test('a missing or damaged record does not resume', () => {
    assert.equal(resumable(null, NOW), false);
    assert.equal(resumable(undefined, NOW), false);
    assert.equal(resumable({ ...record, keys: { send: {} } }, NOW), false);
    assert.equal(resumable({ ...record, seqOut: '5' }, NOW), false);
    assert.equal(resumable({ ...record, room: undefined }, NOW), false);
});
