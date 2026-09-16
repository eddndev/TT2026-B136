import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { factDraft, factValues, factCommand } from '../src/lib/procedural-fact-values.mjs';
import {
  factValuesFixture,
  factRecord,
  factPrepared,
  factCommandFixture,
  factOperationId,
  resolutionId,
  notificationId,
} from './fixtures/procedural-fact-unit.mjs';
const vectors = JSON.parse(
  readFileSync(
    new URL('../../crates/domain/tests/fixtures/procedural_fact_vectors.json', import.meta.url),
  ),
);

test('all 28 independent domain inputs normalize identically without mutating originals', () => {
  assert.equal(vectors.length, 28);
  for (const v of vectors) {
    const input = structuredClone(v.input);
    assert.deepEqual(factValues(v.family, input), v.normalized, v.name);
    assert.deepEqual(input, v.input, v.name);
  }
});
test('draft copies historical values and requires explicit new classifications and time', () => {
  for (const family of ['resolution', 'notification']) {
    const row = factRecord(factPrepared(factCommandFixture(family)));
    const draft = factDraft(family, row);
    assert.deepEqual(draft, { values: row.values, reason: '' });
    draft.values.summary = 'Edited';
    assert.notEqual(row.values.summary, draft.values.summary);
    assert.throws(() => factValues(family, factDraft(family).values));
  }
  const parent = factRecord();
  assert.deepEqual(factDraft('notification', null, parent).values.resolution, {
    id: parent.id,
    revision: parent.revision,
  });
});
test('commands preserve fixed family parent and revisions while withdrawal omits values', () => {
  for (const family of ['resolution', 'notification'])
    for (const action of ['record', 'correct', 'withdraw']) {
      const base =
        action === 'record' ? null : factRecord(factPrepared(factCommandFixture(family)));
      const draft = { values: factValuesFixture(family), reason: ' Motivo\r\nExpreso ' };
      const command = factCommand(draft, {
        family,
        action,
        base,
        operationId: factOperationId,
        id: family === 'resolution' ? resolutionId : notificationId,
        resolutionId,
      });
      assert.equal(command.change.expected_revision, action === 'record' ? 0 : 1);
      assert.equal('values' in command.change, action !== 'withdraw');
      assert.equal('reason' in command.change, action !== 'record');
      if (action !== 'record') assert.equal(command.change.reason, 'Motivo\nExpreso');
      if (family === 'notification') assert.equal(command.resolution_id, resolutionId);
    }
});
test('correction may change parent revision but not its root and may retain archived sources', () => {
  const base = factRecord(factPrepared(factCommandFixture('notification')));
  const draft = factDraft('notification', base);
  draft.reason = 'Otra revision';
  draft.values.resolution.revision = 9;
  const options = {
    family: 'notification',
    action: 'correct',
    base,
    operationId: factOperationId,
    id: notificationId,
    resolutionId,
  };
  assert.equal(factCommand(draft, options).change.values.resolution.revision, 9);
  draft.values.resolution.id = notificationId;
  assert.throws(() => factCommand(draft, options));
  for (const changed of [
    { status: 'withdrawn' },
    { revision: 4294967295 },
    { id: resolutionId },
    { family: 'resolution' },
  ])
    assert.throws(() =>
      factCommand(factDraft('notification', base), { ...options, base: { ...base, ...changed } }),
    );
});
test('unknown and absent values stay distinct and invalid text is never silently discarded', () => {
  const value = factValuesFixture('notification');
  assert.equal(factValues('notification', value).received_at, null);
  value.received_at = { precision: 'unknown' };
  assert.deepEqual(factValues('notification', value).received_at, { precision: 'unknown' });
  for (const text of ['', 'x'.repeat(1001), '\tx', '\ud800', 'bad\rline']) {
    value.summary = text;
    assert.throws(() => factValues('notification', value));
  }
  value.summary = 'x';
  value.intended_recipient = { kind: 'unknown', reason: '' };
  assert.throws(() => factValues('notification', value));
});
test('support dedup preserves distinct locators and rejects contradictory digest expectations', () => {
  const vector = vectors.find((v) => v.name === 'notification_shared_support_distinct_locators');
  const value = structuredClone(vector.normalized);
  const normalized = factValues('notification', value);
  assert.notEqual(
    normalized.provenance.support.locator,
    normalized.representation.provenance.support.locator,
  );
  value.representation.provenance.support.digest = 'f'.repeat(64);
  assert.throws(() => factValues('notification', value));
  value.representation.provenance.support.version = 2;
  assert.equal(factValues('notification', value).representation.provenance.support.version, 2);
});
test('values reject extra fields and positional objects rather than dropping unsupported data', () => {
  const base = factValuesFixture();
  for (const value of [
    { ...base, extra: null },
    { ...base, issued_at: { precision: 'unknown', hour: 0 } },
    { ...base, class: { kind: 'known', value: { kind: 'order', label: 'discarded' } } },
    { ...base, issuer: ['known', 'issuer'] },
    { ...base, provenance: { kind: 'operator_note', note: 'x', support: null } },
  ])
    assert.throws(() => factValues('resolution', value));
});
