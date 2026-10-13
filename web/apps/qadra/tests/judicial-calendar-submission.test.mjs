import test from 'node:test';
import assert from 'node:assert/strict';
import {
  calendarMatches,
  readCalendarSubmission,
  calendarRequest,
} from '../src/lib/judicial-calendar-submission.mjs';
const id = '00000000-0000-0000-0000-000000000000',
  hash = 'ab'.repeat(32);
const prepared = {
  actor_id: id,
  result_revision: 2,
  values_digest: hash,
  submission_digest: hash,
  command: {
    calendar_id: id,
    operation_id: id,
    change: { action: 'replace', expected_revision: 1, reason: 'Change' },
  },
};
const record = () => ({
  id,
  revision: 2,
  status: 'published',
  values_digest: hash,
  reason: 'Change',
  recorded_by: { id },
  receipt: { operation_id: id, action: 'replace', expected_revision: 1, submission_digest: hash },
});
test('reconciliation requires every receipt identity actor action revision and values digest', () => {
  assert.equal(calendarMatches(record(), prepared), true);
  for (const [key, value] of [
    ['id', 'other'],
    ['revision', 3],
    ['status', 'retired'],
    ['values_digest', 'other'],
    ['reason', 'different'],
    ['recorded_by', { id: 'other' }],
    ['receipt', { ...record().receipt, operation_id: 'other' }],
    ['receipt', { ...record().receipt, submission_digest: 'other' }],
    ['receipt', { ...record().receipt, expected_revision: 0 }],
    ['receipt', { ...record().receipt, action: 'retire' }],
  ]) {
    const r = record();
    r[key] = value;
    assert.equal(calendarMatches(r, prepared), false, key);
  }
  assert.equal(calendarMatches(record(), { ...prepared, actor_id: undefined }), false);
  assert.equal(calendarMatches(record(), { ...prepared, result_revision: 3 }), false);
});
test('exact absence stays uncertain and only the target not-found code is reconciled', async () => {
  let called;
  const result = await readCalendarSubmission(
    {
      revision: async (...args) => {
        called = args;
        return record();
      },
    },
    prepared,
  );
  assert.equal(result.state, 'matched');
  assert.deepEqual(called, [id, 2]);
  const failure = Object.assign(new Error('not found'), {
    status: 404,
    code: 'judicial_calendar_not_found',
  });
  assert.deepEqual(
    await readCalendarSubmission(
      {
        revision: async () => {
          throw failure;
        },
      },
      prepared,
    ),
    { state: 'absent' },
  );
  for (const error of [{ status: 401 }, { status: 403 }, { status: 404, code: 'case_not_found' }])
    await assert.rejects(
      readCalendarSubmission(
        {
          revision: async () => {
            throw error;
          },
        },
        prepared,
      ),
    );
  const request = calendarRequest(prepared);
  request.command.calendar_id = 'changed';
  assert.equal(prepared.command.calendar_id, id);
});
