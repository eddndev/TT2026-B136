import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { buildPolicy, DEFAULT_ADMITTED_POLICY } from '../src/lib/evidence/policy.mjs';

const pinned = [
  { fingerprint: 'a'.repeat(64), purposes: ['server-seal', 'time-stamping'], label: 'Qadra' },
];
const now = new Date('2026-10-28T12:00:00.900Z');
const route = {
  'server-seal': {
    root_der_base64: 'ROOT',
    root_fingerprint: 'f1',
    crl_der_base64: 'U0VBTA==',
    crl_digest: 'd1',
  },
  'time-stamping': {
    roots: [
      { root_der_base64: 'ROOT', root_fingerprint: 'f1' },
      { root_der_base64: 'VFNBUg==', root_fingerprint: 'f2' },
    ],
  },
  'individual-signing': {
    deployment_id: 'x',
    revision: 2,
    root_der_base64: 'ROOT',
    root_fingerprint: 'f1',
    crl_der_base64: 'SU5E',
    crl_digest: 'd2',
  },
};

test('route roots with identical DER merge their purposes', () => {
  const policy = buildPolicy({ pinned, trust: route, now });
  assert.deepEqual(policy.served.roots, [
    { der_base64: 'ROOT', purposes: ['server-seal', 'individual-signing', 'time-stamping'] },
    { der_base64: 'VFNBUg==', purposes: ['time-stamping'] },
  ]);
});

test('the seal CRL comes before the individual signing CRL', () => {
  assert.deepEqual(buildPolicy({ pinned, trust: route, now }).served.crls_base64, [
    'U0VBTA==',
    'SU5E',
  ]);
  const sealOnly = { ...route, 'individual-signing': null };
  const policy = buildPolicy({ pinned, trust: sealOnly, now });
  assert.deepEqual(policy.served.crls_base64, ['U0VBTA==']);
  assert.deepEqual(policy.served.roots, [
    { der_base64: 'ROOT', purposes: ['server-seal', 'time-stamping'] },
    { der_base64: 'VFNBUg==', purposes: ['time-stamping'] },
  ]);
});

test('an unavailable or malformed route gives no served trust', () => {
  for (const trust of [null, undefined, {}, { 'server-seal': {} }])
    assert.equal(buildPolicy({ pinned, trust, now }).served, null);
});

test('route material that is not canonical base64 gives no served trust', () => {
  const variants = [
    (value) => (value['server-seal'].root_der_base64 = 'Uk9PVA'),
    (value) => (value['server-seal'].crl_der_base64 = 'U0VBTA=!'),
    (value) => (value['time-stamping'].roots[1].root_der_base64 = 'VFNBUg==='),
    (value) => (value['individual-signing'].root_der_base64 = 'Uk9 PVA=='),
    (value) => (value['individual-signing'].crl_der_base64 = 'QR=='),
    (value) => (value['individual-signing'].crl_der_base64 = 42),
  ];
  for (const change of variants) {
    const trust = structuredClone(route);
    change(trust);
    assert.equal(buildPolicy({ pinned, trust, now }).served, null);
  }
});

test('pinned roots, admitted policy and the client instant are passed through', () => {
  const policy = buildPolicy({ pinned, trust: null, now });
  assert.deepEqual(policy.pinned, pinned);
  assert.notEqual(policy.pinned[0].purposes, pinned[0].purposes);
  assert.deepEqual(policy.admitted_policies, [DEFAULT_ADMITTED_POLICY]);
  assert.equal(policy.current_instant, 1793188800);
  assert.equal(buildPolicy({ pinned, trust: null, now: 1999 }).current_instant, 1);
  assert.throws(() => buildPolicy({ pinned, trust: null, now: Number.NaN }), TypeError);
});

test('the admitted policy matches the engine default', () => {
  const source = readFileSync(
    new URL('../../../../crates/evidence-verify/src/policy.rs', import.meta.url),
    'utf8',
  );
  const match = /DEFAULT_ADMITTED_POLICY: &str = "([0-9.]+)"/.exec(source);
  assert.equal(match?.[1], DEFAULT_ADMITTED_POLICY);
});
