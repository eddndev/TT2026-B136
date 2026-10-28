import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { parsePinnedRoots, pinnedRootsSource } from '../src/lib/evidence/pinned-roots-schema.mjs';

const defaultFile = fileURLToPath(
  new URL('../src/lib/evidence/pinned-roots.json', import.meta.url),
);
const production = '98f934f7152cd0b12ef1bf9b424a62d425a02496b531e218d2cde45a4130a4b6';
const entry = (overrides = {}) => ({
  fingerprint: production,
  purposes: ['server-seal'],
  label: 'Qadra',
  ...overrides,
});

test('the release file is valid and pins the production root for every purpose', () => {
  const roots = parsePinnedRoots(JSON.parse(readFileSync(defaultFile, 'utf8')));
  const root = roots.find((item) => item.fingerprint === production);
  assert.ok(root, 'production root is pinned');
  assert.deepEqual(root.purposes, ['server-seal', 'individual-signing', 'time-stamping']);
  assert.ok(root.label.length > 0);
  assert.ok(Object.isFrozen(roots) && Object.isFrozen(root));
});

test('malformed entries are rejected', () => {
  const invalid = [
    entry({ fingerprint: production.toUpperCase() }),
    entry({ fingerprint: production.slice(1) }),
    entry({ fingerprint: `${production.slice(1)}g` }),
    entry({ purposes: ['server'] }),
    entry({ purposes: [] }),
    entry({ purposes: ['server-seal', 'server-seal'] }),
    entry({ label: '' }),
    entry({ label: '   ' }),
    entry({ extra: true }),
  ];
  for (const value of invalid)
    assert.throws(() => parsePinnedRoots([value]), /Invalid pinned roots/, JSON.stringify(value));
  assert.throws(() => parsePinnedRoots([]), /Invalid pinned roots/);
  assert.throws(() => parsePinnedRoots({}), /Invalid pinned roots/);
  assert.throws(() => parsePinnedRoots([entry(), entry()]), /twice/);
});

test('purposes are returned in canonical order', () => {
  const [root] = parsePinnedRoots([entry({ purposes: ['time-stamping', 'server-seal'] })]);
  assert.deepEqual(root.purposes, ['server-seal', 'time-stamping']);
});

test('an override must be an absolute path to a valid file', () => {
  const files = {
    '/default.json': JSON.stringify([entry()]),
    '/ci.json': JSON.stringify([entry({ label: 'CI' })]),
    '/bad.json': JSON.stringify([entry({ purposes: ['other'] })]),
    '/broken.json': '[',
  };
  const read = (file) => {
    if (!(file in files)) throw new Error('missing');
    return files[file];
  };
  assert.equal(pinnedRootsSource(undefined, '/default.json', read), '/default.json');
  assert.equal(pinnedRootsSource('', '/default.json', read), '/default.json');
  assert.equal(pinnedRootsSource('/ci.json', '/default.json', read), '/ci.json');
  assert.throws(() => pinnedRootsSource('ci.json', '/default.json', read), /absolute/);
  assert.throws(() => pinnedRootsSource('/bad.json', '/default.json', read), /unknown purpose/);
  assert.throws(() => pinnedRootsSource('/broken.json', '/default.json', read), /JSON/);
  assert.throws(() => pinnedRootsSource('/missing.json', '/default.json', read), /JSON/);
});
