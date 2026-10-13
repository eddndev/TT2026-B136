import test from 'node:test';
import assert from 'node:assert/strict';
import {
  factRequest,
  factMatches,
  readFactSubmission,
} from '../src/lib/procedural-fact-submission.mjs';
import {
  factPrepared,
  factRecord,
  factCommandFixture,
  richFactPrepared,
  factDigest,
} from './fixtures/procedural-fact-unit.mjs';

test('receipt reconciliation binds both families all actions actor scope values and source digest', () => {
  for (const family of ['resolution', 'notification'])
    for (const action of ['record', 'correct', 'withdraw']) {
      const prepared = factPrepared(factCommandFixture(family, action)),
        record = factRecord(prepared);
      assert.equal(factMatches(record, prepared), true);
      for (const change of [
        { family: 'other' },
        { id: 'other' },
        { case_id: 'other' },
        { revision: 77 },
        { values_digest: 'f'.repeat(64) },
        { recorded_by: { id: 'other' } },
        { status: 'other' },
        { reason: 'Other' },
      ])
        assert.equal(factMatches({ ...record, ...change }, prepared), false);
      for (const change of [
        { operation_id: 'other' },
        { action: 'other' },
        { expected_revision: 77 },
        { sources_digest: 'f'.repeat(64) },
        { submission_digest: 'f'.repeat(64) },
      ])
        assert.equal(
          factMatches({ ...record, receipt: { ...record.receipt, ...change } }, prepared),
          false,
        );
      if (family === 'notification')
        assert.equal(factMatches({ ...record, resolution_id: 'other' }, prepared), false);
    }
});
test('copied values and readable exact sources cannot differ behind unchanged digests', () => {
  const prepared = richFactPrepared(),
    record = factRecord(prepared);
  assert.equal(factMatches(record, prepared), true);
  for (const pointer of [
    ['values', 'summary'],
    ['sources', 'resolution', 'summary'],
    ['sources', 'participants', 0, 'display_name'],
    ['sources', 'hearing_results', 0, 'summary'],
    ['sources', 'direct_supports', 0, 'name'],
  ]) {
    const changed = structuredClone(record);
    let node = changed;
    for (const key of pointer.slice(0, -1)) node = node[key];
    node[pointer.at(-1)] = 'Different';
    assert.equal(factMatches(changed, prepared), false, pointer.join('.'));
  }
});
test('administration may advance without becoming a CAS and R0 is not fabricated', () => {
  const prepared = factPrepared(),
    record = factRecord(prepared);
  record.recorded_administration = {
    kind: 'recorded',
    case_id: prepared.case_id,
    revision: 3,
    title: 'Updated title',
    reference: null,
    status: 'active',
    values_digest: factDigest,
    changed_at: '2026-09-16T11:00:00Z',
    changed_by: record.recorded_by,
  };
  assert.equal(factMatches(record, prepared), true);
  prepared.observed_administration = structuredClone(record.recorded_administration);
  record.recorded_administration.revision = 4;
  assert.equal(factMatches(record, prepared), true);
  record.recorded_administration.revision = 2;
  assert.equal(factMatches(record, prepared), false);
});
test('missing or malformed evidence on both sides is never a matching receipt', () => {
  for (const [expectedPath, actualPath] of [
    [['actor_id'], ['recorded_by', 'id']],
    [['sources_digest'], ['receipt', 'sources_digest']],
    [['values_digest'], ['values_digest']],
    [
      ['command', 'operation_id'],
      ['receipt', 'operation_id'],
    ],
  ]) {
    const prepared = factPrepared(),
      record = factRecord(prepared);
    for (const [value, path] of [
      [prepared, expectedPath],
      [record, actualPath],
    ]) {
      let node = value;
      for (const key of path.slice(0, -1)) node = node[key];
      delete node[path.at(-1)];
    }
    assert.equal(factMatches(record, prepared), false);
  }
  for (const record of [null, {}, { receipt: null }])
    assert.equal(factMatches(record, factPrepared()), false);
});
test('submission copies only the command and expected digest', () => {
  const prepared = factPrepared(),
    request = factRequest(prepared);
  prepared.command.change.values.summary = 'Edit';
  assert.notEqual(request.command.change.values.summary, prepared.command.change.values.summary);
  assert.deepEqual(Object.keys(request).sort(), ['command', 'expected_submission_digest']);
});
test('reconciliation reads only the target revision and never resends writes', async () => {
  const prepared = factPrepared(),
    record = factRecord(prepared),
    calls = [];
  const api = {
    revision: async (...args) => {
      calls.push(args);
      return record;
    },
    submit: () => assert.fail('No automatic retry'),
  };
  assert.deepEqual(await readFactSubmission(api, prepared), { state: 'matched', record });
  assert.deepEqual(calls, [[prepared.command.id, prepared.result_revision]]);
  record.receipt.operation_id = 'other';
  assert.equal((await readFactSubmission(api, prepared)).state, 'different');
  api.revision = async () => {
    throw { status: 404, code: 'procedural_fact_not_found' };
  };
  assert.deepEqual(await readFactSubmission(api, prepared), { state: 'absent' });
  for (const failure of [
    { status: 404, code: 'case_not_found' },
    { status: 404, code: 'procedural_fact_reference_not_found' },
    { status: 403 },
    { status: 401 },
    { status: 503 },
    new Error('Network'),
  ]) {
    api.revision = async () => {
      throw failure;
    };
    await assert.rejects(readFactSubmission(api, prepared), (error) => error === failure);
  }
});
