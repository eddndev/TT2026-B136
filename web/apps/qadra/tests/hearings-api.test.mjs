import test from 'node:test';
import assert from 'node:assert/strict';
import { createApi } from '../src/lib/api.mjs';
import {
  hearingCaseId,
  hearingId,
  hearingCommand,
  hearingPrepared,
  hearingRecord,
  hearingRow,
  hearingContext,
} from './fixtures/hearings.mjs';

test('hearing adapter separates preparation from action-specific submission and checks the receipt', async () => {
  for (const action of ['schedule', 'replace', 'cancel']) {
    const command = hearingCommand(action, action === 'schedule' ? 0 : 1);
    const prepared = hearingPrepared(command),
      calls = [];
    const api = createApi(async (path, options) => {
      calls.push({ path, method: options.method, body: JSON.parse(options.body) });
      return new Response(
        JSON.stringify(path.endsWith('/prepare') ? prepared : hearingRecord(prepared)),
      );
    }).caseHearings(hearingCaseId);
    await api.prepare(command, 'user');
    await api.submit(prepared);
    assert.equal(calls[0].path, `/api/v1/cases/${hearingCaseId}/hearings/prepare`);
    assert.deepEqual(calls[0].body, command);
    assert.equal(calls[1].method, action === 'replace' ? 'PUT' : 'POST');
    assert.equal(
      calls[1].path,
      `/api/v1/cases/${hearingCaseId}/hearings${action === 'schedule' ? '' : `/${hearingId}${action === 'cancel' ? '/cancellation' : ''}`}`,
    );
    assert.deepEqual(calls[1].body, {
      command,
      expected_submission_digest: prepared.submission_digest,
    });
  }
});

test('hearing reads validate exact identity and keep list and history cursor contracts', async () => {
  const paths = [];
  const api = createApi(async (path) => {
    paths.push(path);
    return new Response(
      JSON.stringify(
        path.includes('/context')
          ? hearingContext
          : path.includes('/history')
            ? { revisions: [hearingRecord()], has_more: false, next_before_revision: null }
            : path.includes('after_id')
              ? { hearings: [hearingRow()], has_more: false, next_after_id: null }
              : hearingRecord(),
      ),
    );
  }).caseHearings(hearingCaseId);
  await api.context();
  await api.list({ status: 'all', afterId: hearingId });
  await api.history(hearingId, { beforeRevision: 2 });
  await api.revision(hearingId, 1);
  await assert.rejects(api.revision(hearingId, 2), /revisi/i);
  assert.match(paths[1], /status=all.*after_id=/);
  assert.match(paths[2], /before_revision=2/);
});

test('foreign prepared actor, wrong operation and disposed responses are rejected', async () => {
  let result = hearingPrepared(),
    release;
  const api = createApi(async () => new Response(JSON.stringify(result))).caseHearings(
    hearingCaseId,
  );
  await assert.rejects(api.prepare(hearingCommand(), 'different'), /actor|sesi/i);
  result = hearingPrepared({ ...hearingCommand(), operation_id: 'other' });
  await assert.rejects(api.prepare(hearingCommand(), 'user'), /operaci|env/i);
  const scoped = createApi(async () => {
    await new Promise((resolve) => {
      release = resolve;
    });
    return new Response(JSON.stringify(hearingRecord()));
  }).caseHearings(hearingCaseId);
  const pending = scoped.get(hearingId);
  scoped.dispose();
  release();
  await assert.rejects(pending, /expediente/i);
});

test('agenda is one authorized endpoint with a paired UTC cursor and no case fanout', async () => {
  const calls = [];
  const api = createApi(async (path) => {
    calls.push(path);
    return new Response(
      JSON.stringify({ hearings: [hearingRow()], has_more: false, next_after: null }),
    );
  }).hearingAgenda();
  await api.list({
    from: '2026-10-01T00:00:00Z',
    until: '2026-10-02T00:00:00Z',
    status: 'scheduled',
    after: { at: '2026-10-01T15:02:03Z', id: hearingId },
  });
  assert.equal(calls.length, 1);
  assert.ok(calls[0].startsWith('/api/v1/hearings?'));
  assert.match(calls[0], /after_time=2026-10-01T15%3A02%3A03Z/);
  assert.match(calls[0], /after_id=/);
});
