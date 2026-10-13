import test from 'node:test';
import assert from 'node:assert/strict';
import { hearingResultsApi } from '../src/lib/hearing-results-api.mjs';
import {
  resultPrepared,
  resultRecord,
  resultRow,
  resultCommand,
  resultHistoryRow,
} from './fixtures/hearing-results.mjs';
import { hearingCaseId, hearingId } from './fixtures/hearings.mjs';

test('result queries separate scoped roots exact details and bounded lightweight history', async () => {
  const row = resultRecord(),
    calls = [];
  const api = hearingResultsApi(
    async (path) => {
      calls.push(path);
      if (path.includes('/history')) return { revisions: [resultHistoryRow(row)], has_more: false };
      if (path.includes('/revisions/')) return row;
      return { results: [resultRow(row)], has_more: false };
    },
    hearingCaseId,
    hearingId,
  );
  await api.list({ status: 'withdrawn', afterId: row.id });
  await api.revision(row.id, 1);
  await api.history(row.id, { beforeRevision: 3, limit: 20 });
  assert.match(calls[0], /results\?status=withdrawn&limit=20&after_id=/);
  assert.match(calls[1], /revisions\/1$/);
  assert.match(calls[2], /history\?limit=20&before_revision=3$/);
  await assert.rejects(api.history(row.id, { limit: 21 }));
});

test('prepared result binds actor operation scope target revision and explicit source selection', async () => {
  const command = resultCommand(),
    prepared = resultPrepared(command);
  for (const change of [
    { actor_id: 'other' },
    { case_id: 'other' },
    { result_revision: 9 },
    { anchor: { ...prepared.anchor, revision: 9 } },
    { continuation: { result_id: 'other', revision: 1 } },
  ]) {
    const api = hearingResultsApi(
      async () => ({ ...prepared, ...change }),
      hearingCaseId,
      hearingId,
    );
    await assert.rejects(api.prepare(command, 'user'));
  }
});

test('response scope and disposed requests cannot restore another result context', async () => {
  const record = resultRecord();
  const api = hearingResultsApi(
    async () => ({ ...record, hearing_id: 'other' }),
    hearingCaseId,
    hearingId,
  );
  await assert.rejects(api.get(record.id));
  let finish;
  const delayed = hearingResultsApi(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
    hearingCaseId,
    hearingId,
  );
  const pending = delayed.get(record.id);
  delayed.dispose();
  finish(record);
  await assert.rejects(pending, /abierto|vigente/);
});

test('submissions use exact operation routes and require the expected result receipt', async () => {
  for (const action of ['record', 'correct', 'withdraw']) {
    const command = resultCommand(action, action === 'record' ? 0 : 1),
      prepared = resultPrepared(command);
    const calls = [];
    const api = hearingResultsApi(
      async (path, options) => {
        calls.push({ path, options });
        return resultRecord(prepared);
      },
      hearingCaseId,
      hearingId,
    );
    await api.submit(prepared);
    assert.equal(calls[0].options.method, action === 'correct' ? 'PUT' : 'POST');
    assert.equal(calls[0].path.endsWith('/withdrawal'), action === 'withdraw');
    const wrong = hearingResultsApi(
      async () => ({ ...resultRecord(prepared), values_digest: 'a'.repeat(64) }),
      hearingCaseId,
      hearingId,
    );
    await assert.rejects(wrong.submit(prepared), /recibo/);
  }
});

test('JSON command byte limit is 512 KiB and includes the submission envelope', async () => {
  let calls = 0;
  const api = hearingResultsApi(
    async () => {
      calls++;
      return resultPrepared();
    },
    hearingCaseId,
    hearingId,
  );
  const command = resultCommand();
  command.change.values.summary = '\u{1f600}'.repeat(131072);
  await assert.rejects(
    api.prepare(command, 'user'),
    (error) => error.status === 413 && /512/.test(error.message),
  );
  assert.equal(calls, 0);
  const prepared = resultPrepared();
  prepared.command.change.values.summary = '';
  const overhead = new TextEncoder().encode(JSON.stringify(prepared.command)).length;
  prepared.command.change.values.summary = 'a'.repeat(524250 - overhead);
  assert.equal(new TextEncoder().encode(JSON.stringify(prepared.command)).length, 524250);
  await assert.rejects(api.submit(prepared), (error) => error.status === 413);
  assert.equal(calls, 0);
});

test('review and detail reject historical attendee projections from a different exact reference', async () => {
  const prepared = resultPrepared();
  prepared.values.attendees = [
    {
      participant_id: '11111111-1111-4111-8111-111111111111',
      revision: 1,
      capacity: 'Persona',
      observation: null,
    },
  ];
  prepared.command.change.values = prepared.values;
  prepared.attendees = [
    {
      id: '22222222-2222-4222-8222-222222222222',
      revision: 1,
      display_name: 'Otra ficha',
      values_digest: 'a'.repeat(64),
    },
  ];
  const api = hearingResultsApi(async () => prepared, hearingCaseId, hearingId);
  await assert.rejects(api.prepare(prepared.command, 'user'), /comparecencia|referencia|ficha/);
  const record = resultRecord(prepared);
  const reader = hearingResultsApi(async () => record, hearingCaseId, hearingId);
  await assert.rejects(reader.get(record.id), /comparecencia|referencia|ficha/);
});
