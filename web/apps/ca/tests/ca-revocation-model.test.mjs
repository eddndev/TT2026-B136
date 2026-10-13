import test from 'node:test';
import assert from 'node:assert/strict';
import { createCaRevocation, canRevoke } from '../src/lib/ca-revocation-model.mjs';

const operationId = '10000000-0000-4000-8000-000000000001';
const operatorId = '20000000-0000-4000-8000-000000000001';
const fingerprint = 'a'.repeat(64);
const rootFingerprint = 'b'.repeat(64);
const identity = { serial: '1000', fingerprint, root_fingerprint: rootFingerprint,
  subject: 'CN=Reviewed Holder', purpose: 'partner', local_status: 'issued',
  published_status: 'not-revoked' };
const reason = 'Holder reported loss of control';
const receipt = { operation_id: operationId, recorded_at: 100, operator_id: operatorId,
  serial: identity.serial, fingerprint, reason, local_status: 'revoked', published_status: 'pending' };

function fixture(respond = () => receipt) {
  const calls = [];
  let uuidCalls = 0;
  const model = createCaRevocation({ oncommand: async (payload) => {
    calls.push(structuredClone(payload)); return respond(payload);
  }, uuid: () => { uuidCalls++; return operationId; } });
  return { model, calls, uuidCalls: () => uuidCalls };
}

function reviewed(model) { model.choose(identity); model.setReason(reason); }

test('selecting a certificate and reason requires separate exact-identity confirmation', async () => {
  const { model, calls } = fixture();
  reviewed(model);
  assert.equal(calls.length, 0);
  assert.equal(model.snapshot().phase, 'review');
  await model.confirm({ serial: '1001', fingerprint });
  await model.confirm({ serial: identity.serial, fingerprint: 'c'.repeat(64) });
  assert.equal(calls.length, 0);
  await model.confirm(identity);
  assert.deepEqual(calls, [{ action: 'revoke', operation_id: operationId,
    serial: identity.serial, fingerprint, reason, expected_root_fingerprint: rootFingerprint }]);
  assert.equal(model.snapshot().phase, 'completed');
  assert.deepEqual(model.snapshot().receipt, receipt);
  assert.equal(model.snapshot().receipt.published_status, 'pending');
});

test('TSA and already revoked certificates never offer revocation', async () => {
  const { model, calls } = fixture();
  assert.equal(canRevoke(identity), true);
  assert.equal(canRevoke({ ...identity, purpose: 'document-signing', local_status: 'expired' }), true);
  for (const value of [{ ...identity, purpose: 'time-stamping' },
    { ...identity, local_status: 'revoked' }, { ...identity, published_status: 'revoked' }]) {
    assert.equal(canRevoke(value), false);
    model.choose(value);
    model.setReason(reason);
    await model.confirm(value);
    assert.equal(model.snapshot().certificate, null);
  }
  assert.equal(calls.length, 0);
});

test('revocation reason must be nonempty, bounded and without control characters', async () => {
  const { model, calls } = fixture();
  model.choose(identity);
  for (const value of ['', ' ', 'x'.repeat(501), 'a\nb']) {
    model.setReason(value);
    await model.confirm(identity);
    assert.ok(model.snapshot().error);
  }
  assert.equal(calls.length, 0);
});

test('uncertain response retains immutable parameters and exact UUID for explicit resume', async () => {
  let attempts = 0;
  const { model, calls, uuidCalls } = fixture(() => {
    if (++attempts === 1) throw new Error('private diagnostic');
    return receipt;
  });
  reviewed(model);
  await model.confirm(identity);
  assert.equal(model.snapshot().phase, 'uncertain');
  assert.equal(model.snapshot().operation_id, operationId);
  assert.doesNotMatch(model.snapshot().error, /private diagnostic/);
  model.setReason('Changed reason');
  model.choose({ ...identity, serial: '1001' });
  model.cancel();
  await model.confirm(identity);
  assert.equal(calls.length, 1);
  assert.equal(model.snapshot().reason, reason);
  await model.resume();
  assert.deepEqual(calls[1], calls[0]);
  assert.equal(uuidCalls(), 1);
  assert.equal(model.snapshot().phase, 'completed');
});

test('double confirmation cannot issue parallel revocations', async () => {
  let resolve;
  const pending = new Promise((done) => { resolve = done; });
  const { model, calls } = fixture(() => pending);
  reviewed(model);
  const first = model.confirm(identity);
  await model.confirm(identity);
  await model.resume();
  assert.equal(calls.length, 1);
  resolve(receipt);
  await first;
});

test('wrong identity or fabricated published receipt keeps result uncertain', async () => {
  for (const changed of [{ serial: '1001' }, { fingerprint: 'c'.repeat(64) },
    { operation_id: operatorId }, { reason: 'Different reason' }, { published_status: 'revoked' }]) {
    const { model } = fixture(() => ({ ...receipt, ...changed }));
    reviewed(model);
    await model.confirm(identity);
    assert.equal(model.snapshot().phase, 'uncertain');
    assert.equal(model.snapshot().receipt, null);
  }
});

test('cancellation before confirmation is local and permits another selection', () => {
  const { model, calls } = fixture();
  reviewed(model);
  model.cancel();
  assert.equal(model.snapshot().phase, 'idle');
  assert.equal(model.snapshot().certificate, null);
  model.choose({ ...identity, serial: '1001' });
  assert.equal(model.snapshot().certificate.serial, '1001');
  assert.equal(calls.length, 0);
});

test('session disposal clears the review and ignores delayed revocation response', async () => {
  let resolve;
  const { model } = fixture(() => new Promise((done) => { resolve = done; }));
  reviewed(model);
  const pending = model.confirm(identity);
  model.dispose();
  resolve(receipt);
  await pending;
  assert.equal(model.snapshot().certificate, null);
  assert.equal(model.snapshot().receipt, null);
  assert.equal(model.snapshot().operation_id, null);
});

test('defensive snapshots cannot replace selected certificate or confirmation reason', async () => {
  const { model, calls } = fixture();
  reviewed(model);
  const copied = model.snapshot();
  copied.certificate.root_fingerprint = 'e'.repeat(64);
  copied.reason = 'Changed';
  await model.confirm(identity);
  assert.equal(calls[0].reason, reason);
  assert.equal(calls[0].expected_root_fingerprint, rootFingerprint);
});

test('fresh session resumes a revocation from history without inventing certificate metadata', async () => {
  const { model, calls, uuidCalls } = fixture();
  await model.resumeOperation({ operation_id: operationId, operator_id: operatorId,
    recorded_at: 100, kind: 'revoke', state: 'pending', serial: identity.serial, fingerprint, reason });
  assert.deepEqual(calls, [{ action: 'resume-revoke', operation_id: operationId }]);
  assert.equal(model.snapshot().receipt.reason, reason);
  assert.equal(model.snapshot().certificate.root_fingerprint, undefined);
  assert.equal(uuidCalls(), 0);
});

test('history revocation recovery failure retains its original resume command', async () => {
  const { model, calls } = fixture(() => { throw new Error('unavailable'); });
  const record = { operation_id: operationId, kind: 'revoke', state: 'pending',
    serial: identity.serial, fingerprint, reason };
  await model.resumeOperation(record);
  await model.resumeOperation({ ...record, operation_id: operatorId });
  await model.resume();
  assert.deepEqual(calls, [
    { action: 'resume-revoke', operation_id: operationId },
    { action: 'resume-revoke', operation_id: operationId },
  ]);
  assert.equal(model.snapshot().operation_id, operationId);
});
