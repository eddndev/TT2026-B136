import test from 'node:test';
import assert from 'node:assert/strict';
import {
  hearingResultRequest,
  hearingResultMatches,
  readHearingResultSubmission,
} from '../src/lib/hearing-result-submission.mjs';

function fixture(action = 'record', continuation = null) {
  const expected = action === 'record' ? 0 : 1;
  const prepared = {
    case_id: 'case',
    actor_id: 'actor',
    result_revision: expected + 1,
    values_digest: 'v'.repeat(64),
    submission_digest: 's'.repeat(64),
    anchor: {
      hearing_id: 'hearing',
      revision: 7,
      values_digest: 'a'.repeat(64),
      submission_digest: 'b'.repeat(64),
      status: 'cancelled',
    },
    continuation,
    observed_administration: { revision: 1, status: 'active' },
    command: {
      operation_id: 'operation',
      hearing_id: 'hearing',
      result_id: 'result',
      change: {
        action,
        expected_revision: expected,
        ...(action === 'record' ? {} : { reason: 'Motivo' }),
      },
    },
  };
  const record = {
    id: 'result',
    hearing_id: 'hearing',
    case_id: 'case',
    revision: expected + 1,
    status: action === 'withdraw' ? 'withdrawn' : 'recorded',
    reason: action === 'record' ? null : 'Motivo',
    values_digest: prepared.values_digest,
    anchor: structuredClone(prepared.anchor),
    continuation: structuredClone(continuation),
    recorded_by: { id: 'actor', email: 'captured@example.test' },
    recorded_administration_revision: 99,
    receipt: {
      operation_id: 'operation',
      action,
      expected_revision: expected,
      submission_digest: prepared.submission_digest,
    },
  };
  return { prepared, record };
}

test('exact result reconciliation binds family scope actor operation action revision and value receipt', () => {
  const { prepared, record } = fixture();
  assert.equal(hearingResultMatches(record, prepared), true);
  for (const change of [
    { id: 'other' },
    { hearing_id: 'other' },
    { case_id: 'other' },
    { revision: 3 },
    { status: 'scheduled' },
    { values_digest: 'different' },
    { recorded_by: { id: 'other' } },
  ])
    assert.equal(hearingResultMatches({ ...record, ...change }, prepared), false);
  for (const change of [
    { operation_id: 'other' },
    { action: 'correct' },
    { expected_revision: 7 },
    { submission_digest: 'different' },
  ])
    assert.equal(
      hearingResultMatches({ ...record, receipt: { ...record.receipt, ...change } }, prepared),
      false,
    );
});

test('exact historical anchor and continuation identities and both digests must match', () => {
  const continuation = {
    hearing_id: 'previous-hearing',
    result_id: 'previous-result',
    revision: 4,
    values_digest: 'c'.repeat(64),
    submission_digest: 'd'.repeat(64),
    status: 'withdrawn',
  };
  const { prepared, record } = fixture('correct', continuation);
  assert.equal(hearingResultMatches(record, prepared), true);
  for (const key of ['hearing_id', 'revision', 'values_digest', 'submission_digest'])
    assert.equal(
      hearingResultMatches({ ...record, anchor: { ...record.anchor, [key]: 'other' } }, prepared),
      false,
    );
  for (const key of ['hearing_id', 'result_id', 'revision', 'values_digest', 'submission_digest'])
    assert.equal(
      hearingResultMatches(
        { ...record, continuation: { ...continuation, [key]: 'other' } },
        prepared,
      ),
      false,
    );
  assert.equal(hearingResultMatches({ ...record, continuation: null }, prepared), false);
  assert.equal(hearingResultMatches({ ...record, anchor: null }, prepared), false);
});

test('withdraw matches its copied values and sources without binding current captured administration', () => {
  const { prepared, record } = fixture('withdraw');
  assert.equal(hearingResultMatches(record, prepared), true);
  assert.equal(
    hearingResultMatches({ ...record, recorded_administration_revision: 100 }, prepared),
    true,
  );
  assert.equal(hearingResultMatches({ ...record, status: 'recorded' }, prepared), false);
  assert.equal(hearingResultMatches({ ...record, reason: 'Another reason' }, prepared), false);
});

test('submission copies the normalized command and never sends observed administration as a CAS', () => {
  const { prepared } = fixture();
  prepared.command.change.values = { summary: 'Before' };
  const request = hearingResultRequest(prepared);
  prepared.command.change.values.summary = 'Edited';
  assert.equal(request.command.change.values.summary, 'Before');
  assert.deepEqual(Object.keys(request).sort(), ['command', 'expected_submission_digest']);
});

test('reconciliation queries only the exact result revision and returns matched or different', async () => {
  const { prepared, record } = fixture();
  const calls = [];
  const api = {
    revision: async (...args) => {
      calls.push(args);
      return record;
    },
  };
  assert.deepEqual(await readHearingResultSubmission(api, prepared), { state: 'matched', record });
  assert.deepEqual(calls, [['result', 1]]);
  record.receipt.operation_id = 'another-attempt';
  assert.deepEqual(await readHearingResultSubmission(api, prepared), {
    state: 'different',
    record,
  });
});

test('an absent exact revision remains uncertain while denied scope or reference failures propagate', async () => {
  const { prepared } = fixture();
  const api = {
    revision: async () => {
      throw { status: 404, code: 'hearing_result_not_found' };
    },
  };
  assert.deepEqual(await readHearingResultSubmission(api, prepared), { state: 'absent' });
  for (const failure of [
    { status: 404, code: 'case_not_found' },
    { status: 404, code: 'hearing_result_reference_not_found' },
    { status: 403, code: 'forbidden' },
    { status: 401 },
    { status: 503 },
  ])
    await assert.rejects(
      readHearingResultSubmission(
        {
          revision: async () => {
            throw failure;
          },
        },
        prepared,
      ),
      (actual) => actual === failure,
    );
});

test('missing or incomplete receipts never reconcile as success', () => {
  const { prepared, record } = fixture();
  for (const value of [
    null,
    {},
    { ...record, receipt: null },
    { ...record, hearing_id: undefined },
  ])
    assert.equal(hearingResultMatches(value, prepared), false);
});

test('missing fields on both the attempt and response do not count as matching evidence', () => {
  for (const [expectedPath, actualPath] of [
    [['case_id'], ['case_id']],
    [['actor_id'], ['recorded_by', 'id']],
    [['values_digest'], ['values_digest']],
    [['submission_digest'], ['receipt', 'submission_digest']],
    [['command', 'hearing_id'], ['hearing_id']],
    [['command', 'result_id'], ['id']],
    [
      ['command', 'operation_id'],
      ['receipt', 'operation_id'],
    ],
  ]) {
    const { prepared, record } = fixture();
    const remove = (value, path) => {
      const parent = path.slice(0, -1).reduce((part, key) => part[key], value);
      delete parent[path.at(-1)];
    };
    remove(prepared, expectedPath);
    remove(record, actualPath);
    assert.equal(hearingResultMatches(record, prepared), false, expectedPath.join('.'));
  }
});
