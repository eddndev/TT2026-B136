import test from 'node:test';
import assert from 'node:assert/strict';
import { fetchEvidenceTrust } from '../src/lib/evidence/trust-api.mjs';
import { versionApi } from '../src/lib/version-api.mjs';

const trust = {
  'server-seal': {
    root_der_base64: 'Uk9PVA==',
    root_fingerprint: 'a'.repeat(64),
    crl_der_base64: 'Q1JM',
    crl_digest: 'b'.repeat(64),
  },
  'time-stamping': { roots: [{ root_der_base64: 'Uk9PVA==', root_fingerprint: 'a'.repeat(64) }] },
  'individual-signing': null,
};

test('the trust route is read with a plain GET', async () => {
  const calls = [];
  const value = await fetchEvidenceTrust(async (path, options) => {
    calls.push([path, options]);
    return trust;
  });
  assert.deepEqual(value, trust);
  assert.deepEqual(calls, [['/evidence/trust', undefined]]);
});

test('any failure of the trust route gives null', async () => {
  assert.equal(
    await fetchEvidenceTrust(async () => {
      throw new Error('offline');
    }),
    null,
  );
  for (const answer of [null, [], 'text', {}, { 'server-seal': null }])
    assert.equal(await fetchEvidenceTrust(async () => answer), null);
});

test('the selected version reads the trust route through the account request', async () => {
  const paths = [];
  const api = versionApi(
    async () => {
      throw new Error('not scoped');
    },
    'case',
    async (path) => {
      paths.push(path);
      return trust;
    },
  );
  const selected = api.version('doc', 1);
  assert.deepEqual(await selected.evidenceTrust(), trust);
  assert.deepEqual(paths, ['/evidence/trust']);
  selected.dispose();
  assert.equal(await selected.evidenceTrust(), null);
});
