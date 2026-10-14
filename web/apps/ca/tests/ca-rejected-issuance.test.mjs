import test from 'node:test';
import assert from 'node:assert/strict';
import { createCaOperations } from '../src/lib/ca-operations-model.mjs';
import { receiptValue, pageValue } from '../src/lib/ca-operations-values.mjs';

const id = '10000000-0000-4000-8000-000000000001';
const operator = '10000000-0000-4000-8000-000000000002';
const rejection = { state: 'rejected', reason: 'csr_subject_policy', operation_id: id,
  operator_id: operator, recorded_at: 100, rejected_at: 110, csr_sha256: 'a'.repeat(64),
  root_fingerprint: 'b'.repeat(64), reserved_serial: '1002', holder: 'Reviewed Holder',
  reference: 'Saved request', criterion: 'Original review', purpose: 'partner' };
const history = { operation_id: id, kind: 'issue', state: 'rejected', recorded_at: 100,
  operator_id: operator, holder: rejection.holder, reference: rejection.reference,
  criterion: rejection.criterion, serial: null, fingerprint: null, reason: 'csr_subject_policy' };
const pending = { ...history, state: 'pending' };
delete pending.reason;

test('a rejected saved request ends recovery without a certificate or another issuance', async () => {
  const calls = [];
  const model = createCaOperations({ oncommand: async (command) => {
    calls.push(command);
    return command.action === 'history'
      ? { items: [pending], total: 1, offset: 0, limit: 20 } : rejection;
  } });
  await model.loadHistory();
  await model.resume(id);
  assert.equal(model.snapshot().phase, 'rejected');
  assert.deepEqual(model.snapshot().receipt, rejection);
  assert.equal(model.snapshot().receipt.certificate_pem, undefined);
  model.startNew();
  assert.equal(model.snapshot().phase, 'draft');
  assert.equal(model.snapshot().operation_id, null);
  assert.deepEqual(calls.map((value) => value.action), ['history', 'resume-issue']);
});

test('rejected history preserves review and never offers a fictitious certificate', () => {
  assert.deepEqual(pageValue({ items: [history], total: 1, offset: 0, limit: 20 },
    'history', 0).items, [history]);
  for (const patch of [{ serial: '1002' }, { fingerprint: 'c'.repeat(64) },
    { kind: 'revoke' }, { reason: 'temporary_outage' }]) {
    assert.throws(() => pageValue({ items: [{ ...history, ...patch }], total: 1,
      offset: 0, limit: 20 }, 'history', 0));
  }
});

test('terminal rejection must match the saved operation and exact declared request', () => {
  assert.deepEqual(receiptValue(rejection, id), rejection);
  for (const patch of [{ operation_id: operator }, { reason: 'temporary_outage' },
    { rejected_at: 99 }, { csr_sha256: 'invalid' }, { root_fingerprint: 'invalid' },
    { reserved_serial: '0' }, { certificate_pem: 'not an issued certificate' }]) {
    assert.throws(() => receiptValue({ ...rejection, ...patch }, id));
  }
  assert.throws(() => receiptValue(rejection, id, { ...rejection, holder: 'Another holder' }));
});
