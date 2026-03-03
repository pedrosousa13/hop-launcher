import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const overlayPath = path.resolve(__dirname, '../ui/launcherOverlay.js');

test('launcher overlay does not pass unsupported orientation property to St.BoxLayout', () => {
    const source = fs.readFileSync(overlayPath, 'utf8');
    assert.equal(source.includes('orientation: Clutter.Orientation.VERTICAL'), false);
});
