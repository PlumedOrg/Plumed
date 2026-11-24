import { test } from 'node:test';
import assert from 'node:assert/strict';
import { decodeFile, FileTooLarge, MAX_FILE_BYTES } from '../static/audio.js';

test('a huge file is refused before it is read', async () => {
    const file = {
        size: MAX_FILE_BYTES + 1,
        arrayBuffer: () => assert.fail('the file was read'),
    };
    await assert.rejects(decodeFile(file, 60), FileTooLarge);
});
