import test from 'node:test';
import assert from 'node:assert/strict';
import { confirmSessionWindow, remainingSessionMilliseconds } from '../src/lib/session-clock.mjs';

const sample = (change = {}) => ({
  policy: { absolute_ttl_seconds: 86400, idle_ttl_seconds: 900 },
  server_now_unix_ms: 1700000000000,
  absolute_expires_at_unix_ms: 1700086400000,
  idle_expires_at_unix_ms: 1700000900000,
  ...change,
});
const receipt = { startedAt: 1000, receivedAt: 1250 };

test('the earliest server deadline loses the complete observed round trip', () => {
  const result = confirmSessionWindow(sample(), receipt);
  assert.equal(result.remainingMsAtReceipt, 899750);
  assert.equal(result.expiresAtMonotonicMs, 901000);
  assert.equal(remainingSessionMilliseconds(result, 2250), 898750);
  assert.equal(remainingSessionMilliseconds(result, 901000), 0);
  assert.equal(remainingSessionMilliseconds(result, 999000), 0);
  assert.equal(Object.isFrozen(result), true);
});

test('absolute-only sessions retain their fixed maximum without inventing activity', () => {
  const result = confirmSessionWindow(
    sample({
      policy: { absolute_ttl_seconds: 86400, idle_ttl_seconds: null },
      idle_expires_at_unix_ms: null,
    }),
    receipt,
  );
  assert.equal(result.remainingMsAtReceipt, 86399750);
  assert.equal(result.idleExpiresAtMs, null);
  assert.equal(result.absoluteExpiresAtMs, 1700086400000);
});

test('activity close to absolute expiry never creates another full idle window', () => {
  const result = confirmSessionWindow(
    sample({
      absolute_expires_at_unix_ms: 1700000001000,
      idle_expires_at_unix_ms: 1700000001000,
    }),
    receipt,
  );
  assert.equal(result.remainingMsAtReceipt, 750);
});

test('a valid response exhausted in transit grants no local time', () => {
  const result = confirmSessionWindow(sample({ idle_expires_at_unix_ms: 1700000000100 }), receipt);
  assert.equal(result.remainingMsAtReceipt, 0);
  assert.equal(remainingSessionMilliseconds(result, receipt.receivedAt), 0);
});

test('client wall-clock changes never participate in the calculation', () => {
  const before = Date.now;
  try {
    Date.now = () => {
      throw new Error('wall clock must not be read');
    };
    assert.equal(confirmSessionWindow(sample(), receipt).remainingMsAtReceipt, 899750);
  } finally {
    Date.now = before;
  }
});

test('invalid policy and inconsistent deadlines are rejected instead of extended', () => {
  const cases = [
    null,
    {},
    sample({ policy: { absolute_ttl_seconds: 0, idle_ttl_seconds: null } }),
    sample({ policy: { absolute_ttl_seconds: 86401, idle_ttl_seconds: 900 } }),
    sample({ policy: { absolute_ttl_seconds: 86400, idle_ttl_seconds: 0 } }),
    sample({ policy: { absolute_ttl_seconds: 60, idle_ttl_seconds: 61 } }),
    sample({ policy: { absolute_ttl_seconds: 86400, idle_ttl_seconds: null } }),
    sample({ idle_expires_at_unix_ms: null }),
    sample({ idle_expires_at_unix_ms: 1700086400001 }),
    sample({ idle_expires_at_unix_ms: 1700000900001 }),
    sample({ absolute_expires_at_unix_ms: 1700086400001 }),
    sample({ server_now_unix_ms: -1 }),
    sample({ server_now_unix_ms: 1.5 }),
    sample({ absolute_expires_at_unix_ms: Infinity }),
    sample({ idle_expires_at_unix_ms: '1700000900000' }),
    sample({ absolute_expires_at_unix_ms: 1700000000000 }),
  ];
  for (const value of cases) assert.throws(() => confirmSessionWindow(value, receipt));
});

test('non-monotonic or invalid receipt times never create a usable window', () => {
  for (const times of [
    { startedAt: 2, receivedAt: 1 },
    { startedAt: -1, receivedAt: 1 },
    { startedAt: NaN, receivedAt: 1 },
    { startedAt: 1, receivedAt: Infinity },
    { startedAt: '1', receivedAt: 2 },
  ])
    assert.throws(() => confirmSessionWindow(sample(), times));
  const result = confirmSessionWindow(sample(), receipt);
  assert.throws(() => remainingSessionMilliseconds(result, 1249));
  assert.throws(() => remainingSessionMilliseconds(result, NaN));
});
