import test from 'node:test';
import assert from 'node:assert/strict';
import { handleValidation } from '../src/lib/evidence-verify-run.mjs';

const report = { engine: { name: 'evidence-verify', version: '0.1.0' }, overall: 'checked' };
async function exchange(data, overrides = {}) {
  const posted = [];
  const calls = [];
  await handleValidation(data, {
    load: async () => {},
    validate: (zip, policy) => {
      calls.push([zip, policy]);
      return JSON.stringify(report);
    },
    sourceDigest: 'f'.repeat(64),
    post: (message) => posted.push(message),
    ...overrides,
  });
  return { posted, calls };
}
const request = () => ({ op: 7, zip: new Uint8Array([1, 2, 3]).buffer, policy: { pinned: [] } });

test('a validation posts its stages and the report with the source digest', async () => {
  const { posted, calls } = await exchange(request());
  assert.deepEqual(posted, [
    { op: 7, stage: 'module' },
    { op: 7, stage: 'validate' },
    {
      op: 7,
      ok: true,
      report: {
        engine: { name: 'evidence-verify', version: '0.1.0', source_digest: 'f'.repeat(64) },
        overall: 'checked',
      },
    },
  ]);
  const [[zip, policy]] = calls;
  assert.ok(zip instanceof Uint8Array);
  assert.deepEqual([...zip], [1, 2, 3]);
  assert.equal(policy, '{"pinned":[]}');
});

test('a module that cannot load is reported as unavailable', async () => {
  const { posted, calls } = await exchange(request(), {
    load: async () => {
      throw new Error('compile');
    },
  });
  assert.deepEqual(posted, [
    { op: 7, stage: 'module' },
    { op: 7, ok: false, error: 'module_unavailable' },
  ]);
  assert.equal(calls.length, 0);
});

test('engine errors are reduced to fixed codes', async () => {
  const cases = [
    [
      () => {
        throw 'policy.fingerprint';
      },
      'policy_invalid',
    ],
    [
      () => {
        throw 'policy.malformed';
      },
      'policy_invalid',
    ],
    [
      () => {
        throw new Error('unreachable executed');
      },
      'engine_failed',
    ],
    [() => 'not json', 'engine_failed'],
    [() => '[]', 'engine_failed'],
    [() => '{"overall":"checked"}', 'engine_failed'],
  ];
  for (const [validate, error] of cases) {
    const { posted } = await exchange(request(), { validate });
    assert.deepEqual(posted.at(-1), { op: 7, ok: false, error });
  }
});

test('a request without a ZIP buffer or a policy object fails before loading', async () => {
  for (const data of [
    { op: 1, zip: new Uint8Array(1), policy: {} },
    { op: 1, zip: new ArrayBuffer(1), policy: null },
    null,
  ]) {
    let loaded = false;
    const { posted } = await exchange(data, {
      load: async () => {
        loaded = true;
      },
    });
    assert.equal(loaded, false);
    assert.equal(posted.length, 1);
    assert.equal(posted[0].ok, false);
    assert.equal(posted[0].error, 'engine_failed');
  }
});
