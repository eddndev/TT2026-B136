import test from 'node:test';
import assert from 'node:assert/strict';
import { pageValue } from '../src/lib/ca-operations-values.mjs';
import { createCaOperations } from '../src/lib/ca-operations-model.mjs';

const common = { operation_id: '10000000-0000-4000-8000-000000000001',
  operator_id: '20000000-0000-4000-8000-000000000001', recorded_at: 100, state: 'pending' };
const revoke = { ...common, kind: 'revoke', serial: '1000', fingerprint: 'a'.repeat(64), reason: 'Key lost' };
const publication = { ...common, kind: 'publication', expected_revision: 2 };
const page = (items) => ({ items, total: items.length, offset: 0, limit: 20 });

test('history preserves distinct issuance, revocation and publication records', () => {
  const issue = { ...common, kind: 'issue', holder: 'Holder', reference: 'Reference', criterion: 'Review',
    serial: null, fingerprint: null };
  const result = pageValue(page([issue, revoke, publication]), 'history', 0).items;
  assert.deepEqual(result[0], issue);
  assert.deepEqual(result[1], revoke);
  assert.deepEqual(result[2], { ...publication, revision: null });
  assert.equal(pageValue(page([{ ...publication, state: 'completed', revision: 3 }]), 'history', 0)
    .items[0].revision, 3);
});

test('malformed or mismatched history kinds cannot fabricate a completion', () => {
  for (const record of [{ ...common, kind: 'unknown' }, { ...revoke, fingerprint: null },
    { ...publication, expected_revision: '2' }, { ...publication, state: 'completed', revision: null }]) {
    assert.throws(() => pageValue(page([record]), 'history', 0));
  }
});

test('issuance recovery cannot send a revocation or publication as resume-issue', async () => {
  const calls = [];
  const model = createCaOperations({ oncommand: async (command) => {
    calls.push(command); return page([revoke]);
  } });
  await model.loadHistory();
  assert.equal(model.snapshot().history.items[0].kind, 'revoke');
  await model.resume(revoke.operation_id);
  assert.deepEqual(calls, [{ action: 'history', offset: 0, limit: 20 }]);
});
