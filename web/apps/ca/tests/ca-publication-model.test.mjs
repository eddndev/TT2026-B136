import test from 'node:test';
import assert from 'node:assert/strict';
import { createCaPublication } from '../src/lib/ca-publication-model.mjs';

const operationId = '10000000-0000-4000-8000-000000000001';
const operatorId = '20000000-0000-4000-8000-000000000001';
const rootFingerprint = 'a'.repeat(64);
const status = { deployment_id: operatorId, revision: 2, root_fingerprint: rootFingerprint,
  crl_digest: 'b'.repeat(64), crl_number: 4096, valid_from: 90, valid_until: 200,
  checked_at: 100, valid_now: true, maintenance_pending: false, revoked_serials: [],
  root_pem: 'PUBLIC ROOT', crl_pem: 'PUBLIC CRL' };
const receipt = { operation_id: operationId, operator_id: operatorId, recorded_at: 100,
  expected_revision: 2, revision: 3, crl_number: 4097, root_fingerprint: rootFingerprint,
  state: 'published' };
const history = { operation_id: operationId, operator_id: operatorId, recorded_at: 100,
  kind: 'publication', state: 'pending', expected_revision: 2, revision: null };

function fixture(respond = (command) => command.action === 'status' ? status : receipt) {
  const calls = [];
  let uuidCalls = 0;
  const model = createCaPublication({ oncommand: async (payload) => {
    calls.push(structuredClone(payload)); return respond(payload);
  }, uuid: () => { uuidCalls++; return operationId; } });
  return { model, calls, uuidCalls: () => uuidCalls };
}
const confirmation = { expected_revision: 2, acknowledge_maintenance: true };

test('status is read-only and publishing requires exact revision plus maintenance acknowledgement', async () => {
  const { model, calls, uuidCalls } = fixture();
  await model.confirm(confirmation);
  assert.equal(calls.length, 0);
  await model.refresh();
  assert.deepEqual(calls, [{ action: 'status' }]);
  assert.equal(model.snapshot().status.revision, 2);
  await model.confirm({ ...confirmation, acknowledge_maintenance: false });
  await model.confirm({ ...confirmation, expected_revision: 1 });
  assert.equal(calls.length, 1);
  await model.confirm(confirmation);
  assert.deepEqual(calls[1], { action: 'renew-crl', operation_id: operationId, ...confirmation });
  assert.equal(uuidCalls(), 1);
  assert.deepEqual(model.snapshot().receipt, receipt);
  assert.equal(model.snapshot().phase, 'completed');
});

test('existing maintenance requires recovery instead of a new publication', async () => {
  const { model, calls } = fixture(() => ({ ...status, maintenance_pending: true }));
  await model.refresh();
  await model.confirm(confirmation);
  assert.equal(calls.length, 1);
  assert.equal(model.snapshot().operation_id, null);
});

test('uncertain publication keeps UUID and explicitly resumes without another renew command', async () => {
  const { model, calls, uuidCalls } = fixture((command) => {
    if (command.action === 'status') return status;
    if (command.action === 'renew-crl') throw new Error('private diagnostic');
    return receipt;
  });
  await model.refresh();
  await model.confirm(confirmation);
  assert.equal(model.snapshot().phase, 'uncertain');
  assert.equal(model.snapshot().operation_id, operationId);
  assert.doesNotMatch(model.snapshot().error, /private diagnostic/);
  model.reset();
  await model.confirm(confirmation);
  assert.equal(calls.length, 2);
  await model.resume();
  assert.deepEqual(calls[2], { action: 'resume-crl', operation_id: operationId });
  assert.equal(uuidCalls(), 1);
  assert.equal(model.snapshot().phase, 'completed');
});

test('fresh session resumes publication from its saved history revision', async () => {
  const { model, calls, uuidCalls } = fixture();
  await model.resumeOperation(history);
  assert.deepEqual(calls, [{ action: 'resume-crl', operation_id: operationId }]);
  assert.equal(model.snapshot().receipt.expected_revision, 2);
  assert.equal(uuidCalls(), 0);
});

test('recovery cannot replace a pending publication with another operation', async () => {
  const { model, calls } = fixture((command) => {
    if (command.action === 'status') return status;
    throw new Error('unavailable');
  });
  await model.refresh();
  await model.confirm(confirmation);
  await model.resumeOperation({ ...history, operation_id: operatorId });
  assert.equal(calls.length, 2);
  assert.equal(model.snapshot().operation_id, operationId);
});

test('receipt must match operation, original revision, next revision and configured root', async () => {
  for (const change of [{ operation_id: operatorId }, { expected_revision: 1 },
    { revision: 2 }, { root_fingerprint: 'c'.repeat(64) }, { state: 'pending' }]) {
    const { model } = fixture((command) => command.action === 'status' ? status : { ...receipt, ...change });
    await model.refresh();
    await model.confirm(confirmation);
    assert.equal(model.snapshot().phase, 'uncertain');
    assert.equal(model.snapshot().receipt, null);
  }
});

test('duplicate confirmation submits one publication and session loss drops a late receipt', async () => {
  let resolve;
  const { model, calls } = fixture((command) => command.action === 'status' ? status
    : new Promise((done) => { resolve = done; }));
  await model.refresh();
  const pending = model.confirm(confirmation);
  await model.confirm(confirmation);
  assert.equal(calls.length, 2);
  model.dispose();
  resolve(receipt);
  await pending;
  assert.equal(model.snapshot().receipt, null);
  assert.equal(model.snapshot().status, null);
});

test('failed status refresh clears a stale revision and cannot trigger automatic renewal', async () => {
  let fail = false;
  const { model, calls } = fixture(() => { if (fail) throw new Error('unavailable'); return status; });
  await model.refresh();
  fail = true;
  await model.refresh();
  assert.equal(model.snapshot().status, null);
  await model.confirm(confirmation);
  assert.equal(calls.filter((call) => call.action === 'renew-crl').length, 0);
});
