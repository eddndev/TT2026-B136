import test from 'node:test';
import assert from 'node:assert/strict';
import { proceduralFactsApi } from '../src/lib/procedural-facts-api.mjs';
import {
  factCaseId,
  factActorId,
  resolutionId,
  factPrepared,
  factRecord,
  factRow,
  factHistoryRow,
  factCommandFixture,
  richFactPrepared,
} from './fixtures/procedural-fact-unit.mjs';
const apiFor = (request, family = 'resolution') =>
  proceduralFactsApi(
    request,
    factCaseId,
    family,
    family === 'notification' ? resolutionId : undefined,
  );

test('both scoped families expose heads exact revisions and bounded lightweight history', async () => {
  for (const family of ['resolution', 'notification']) {
    const row = factRecord(factPrepared(factCommandFixture(family))),
      calls = [];
    const api = apiFor(async (path) => {
      calls.push(path);
      if (path.includes('/history'))
        return { revisions: [factHistoryRow(row)], has_more: false, next_before_revision: null };
      if (path.includes('/revisions/')) return row;
      return {
        [family === 'resolution' ? 'resolutions' : 'notifications']: [factRow(row)],
        has_more: false,
        next_after_id: null,
      };
    }, family);
    await api.list({ status: 'all', limit: 1 });
    await api.revision(row.id, 1);
    await api.history(row.id, { beforeRevision: 2 });
    assert.match(
      calls[0],
      family === 'resolution'
        ? /\/cases\/[^/]+\/resolutions\?/
        : /\/resolutions\/[^/]+\/notifications\?/,
    );
    assert.match(calls[1], /\/revisions\/1$/);
    assert.match(calls[2], /before_revision=2/);
    await assert.rejects(api.list({ limit: 101 }));
    await assert.rejects(api.history(row.id, { limit: 21 }));
  }
});
test('prepare binds actor normalized command values exact sources and all three digests', async () => {
  for (const family of ['resolution', 'notification']) {
    const prepared = factPrepared(factCommandFixture(family));
    assert.deepEqual(
      await apiFor(async () => prepared, family).prepare(prepared.command, factActorId),
      prepared,
    );
    for (const edit of [
      (p) => (p.actor_id = 'other'),
      (p) => (p.case_id = 'other'),
      (p) => (p.result_revision = 7),
      (p) => (p.command.operation_id = 'other'),
      (p) => (p.command.change.expected_revision = 4),
      (p) => (p.command.change.values.summary = 'Other'),
      (p) => (p.values.summary = 'Other'),
      (p) => (p.sources_digest = 'bad'),
      (p) => (p.values_digest = 'bad'),
      (p) => (p.submission_digest = 'bad'),
    ]) {
      const changed = structuredClone(prepared);
      edit(changed);
      await assert.rejects(
        apiFor(async () => changed, family).prepare(prepared.command, factActorId),
      );
    }
  }
});
test('exact source union rejects missing extra foreign revised and contradictory projections', async () => {
  const prepared = richFactPrepared();
  assert.deepEqual(
    await apiFor(async () => prepared, 'notification').prepare(prepared.command, factActorId),
    prepared,
  );
  for (const edit of [
    (p) => (p.sources.participants = []),
    (p) => p.sources.participants.push(p.sources.participants[0]),
    (p) => (p.sources.participants[0].case_id = 'other'),
    (p) => (p.sources.participants[0].revision = 1),
    (p) => (p.sources.hearing_results[0].agreement_id = '00000000-0000-0000-0000-000000000000'),
    (p) =>
      (p.sources.hearing_results[0].agreement = {
        id: '00000000-0000-0000-0000-000000000000',
        text: 'Invented',
      }),
    (p) => (p.sources.resolution.revision = 9),
    (p) => (p.sources.direct_supports[0].digest = 'f'.repeat(64)),
    (p) => p.sources.direct_supports.push(p.sources.direct_supports[0]),
  ]) {
    const changed = structuredClone(prepared);
    edit(changed);
    await assert.rejects(
      apiFor(async () => changed, 'notification').prepare(prepared.command, factActorId),
    );
  }
});
test('responses enforce case family immutable parent and requested exact revision', async () => {
  const row = factRecord(factPrepared(factCommandFixture('notification')));
  for (const edit of [
    { case_id: 'other' },
    { family: 'resolution' },
    { resolution_id: 'other' },
    { id: resolutionId },
    { revision: 7 },
  ])
    await assert.rejects(
      apiFor(async () => ({ ...row, ...edit }), 'notification').revision(row.id, 1),
    );
});
test('late success and rejection after dispose cannot reactivate a prior context', async () => {
  for (const reject of [false, true]) {
    let finish;
    const row = factRecord();
    const api = apiFor(
      () =>
        new Promise((yes, no) => {
          finish = reject ? no : yes;
        }),
    );
    const pending = api.get(row.id);
    api.dispose();
    finish(reject ? new Error('late') : row);
    await assert.rejects(pending);
    await assert.rejects(api.get(row.id));
  }
});
test('preparation captures the submitted command before asynchronous UI edits', async () => {
  let finish;
  const original = factCommandFixture(),
    prepared = factPrepared(original);
  const api = apiFor(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const pending = api.prepare(original, factActorId);
  original.change.values.summary = 'Local edit';
  finish(prepared);
  assert.deepEqual(await pending, prepared);
});
test('submit routes six actions with exact receipts and no administration CAS', async () => {
  for (const family of ['resolution', 'notification'])
    for (const action of ['record', 'correct', 'withdraw']) {
      const prepared = factPrepared(factCommandFixture(family, action)),
        calls = [];
      const api = apiFor(async (path, options) => {
        calls.push({ path, options });
        return factRecord(prepared);
      }, family);
      await api.submit(prepared);
      assert.equal(calls.length, 1);
      assert.equal(calls[0].options.method, action === 'correct' ? 'PUT' : 'POST');
      assert.equal(calls[0].path.endsWith('/withdrawal'), action === 'withdraw');
      assert.deepEqual(Object.keys(calls[0].options.data).sort(), [
        'command',
        'expected_submission_digest',
      ]);
      const changed = factRecord(prepared);
      changed.receipt.operation_id = 'other';
      await assert.rejects(apiFor(async () => changed, family).submit(prepared));
    }
});
test('outgoing route scope and finite body budget reject before any request', async () => {
  let calls = 0;
  const api = apiFor(async () => {
    calls++;
    return factPrepared();
  });
  await assert.rejects(api.prepare(factCommandFixture('notification'), factActorId));
  const command = factCommandFixture();
  command.change.values.summary = 'x'.repeat(524288);
  await assert.rejects(api.prepare(command, factActorId), (error) => error.status === 413);
  assert.equal(calls, 0);
});
test('lists and history reject contradictory pagination or foreign lightweight rows', async () => {
  const row = factRecord();
  for (const page of [
    { resolutions: [factRow(row), factRow(row)], has_more: false, next_after_id: null },
    { resolutions: [{ ...factRow(row), case_id: 'other' }], has_more: false, next_after_id: null },
    { resolutions: [], has_more: true, next_after_id: row.id },
    { resolutions: [factRow(row)], has_more: true, next_after_id: 'other' },
  ])
    await assert.rejects(apiFor(async () => page).list());
  for (const page of [
    {
      revisions: [{ ...factHistoryRow(row), id: resolutionId + 'x' }],
      has_more: false,
      next_before_revision: null,
    },
    {
      revisions: [factHistoryRow(row), factHistoryRow(row)],
      has_more: false,
      next_before_revision: null,
    },
  ])
    await assert.rejects(apiFor(async () => page).history(row.id));
});
