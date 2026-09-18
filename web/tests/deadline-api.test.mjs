import test from 'node:test';
import assert from 'node:assert/strict';
import { deadlinesApi } from '../src/lib/deadline-api.mjs';
import { deadlineDenied, canDeadlines, deadlineFailure } from '../src/lib/deadline-errors.mjs';
import {
  prepared,
  detail,
  summary,
  history,
  id,
  administration,
  hash,
} from './fixtures/deadline-unit.mjs';

test('prepare normalizes command, scopes actor and sends verified submission once', async () => {
  const p = prepared(),
    calls = [],
    api = deadlinesApi(async (path, options) => {
      calls.push({ path, options });
      return path.endsWith('/prepare') ? structuredClone(p) : detail(p);
    }, id(1));
  const raw = structuredClone(p.command);
  raw.change.definition.title = '  Respuesta declarada  ';
  const ready = await api.prepare(raw, id(4));
  assert.deepEqual(await api.submit(ready), detail(p));
  assert.equal(calls.length, 2);
  assert.equal(calls[1].path, `/cases/${id(1)}/deadlines`);
  assert.deepEqual(calls[1].options.data, {
    command: p.command,
    expected_submission_digest: p.submission_digest,
  });
});
test('all mutation routes use their expected method and preserve precise history reads', async () => {
  for (const [action, tail, method] of [
    ['correct', '', 'PUT'],
    ['set_attention', '/attention', 'POST'],
    ['retire', '/retirement', 'POST'],
  ]) {
    const p = prepared(action),
      calls = [],
      api = deadlinesApi(async (path, options) => {
        calls.push([path, options]);
        return detail(p);
      }, id(1));
    await api.submit(p);
    assert.equal(calls[0][0], `/cases/${id(1)}/deadlines/${id(6)}${tail}`);
    assert.equal(calls[0][1].method, method);
    await api.revision(id(6), 2);
    assert.equal(calls[1][0], `/cases/${id(1)}/deadlines/${id(6)}/revisions/2`);
  }
});
test('wrong scopes, commands, result bodies and disposed async completions are rejected', async () => {
  const p = prepared();
  for (const mutate of [
    (p) => {
      p.actor_id = id(9);
    },
    (p) => {
      p.case_id = id(9);
    },
    (p) => {
      p.result_revision = 3;
    },
    (p) => {
      p.calculation.result.blocks = {};
    },
    (p) => {
      p.calculation.result.rule = { kind: 'elapsed_hours', quantity: 0 };
    },
  ]) {
    const response = structuredClone(p);
    mutate(response);
    const api = deadlinesApi(async () => response, id(1));
    await assert.rejects(() => api.prepare(p.command, id(4)));
  }
  let resolve;
  const api = deadlinesApi(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
    id(1),
  );
  const waiting = api.get(id(6));
  api.dispose();
  resolve(detail());
  await assert.rejects(() => waiting);
  await assert.rejects(() => api.submit(p));
});
test('pagination validates scope, order, cursor, role and query bounds', async () => {
  const page = { case_id: id(1), deadlines: [summary()], has_more: true, next_after_id: id(6) };
  const calls = [],
    api = deadlinesApi(async (path) => {
      calls.push(path);
      return structuredClone(page);
    }, id(1));
  await api.list({ limit: 1, afterId: id(0) });
  assert.match(calls[0], /status=active&limit=1&after_id=/);
  for (const mutate of [
    (p) => {
      p.case_id = id(9);
    },
    (p) => {
      p.next_after_id = null;
    },
    (p) => {
      p.deadlines[0].responsible.role = 'client';
    },
    (p) => {
      p.deadlines[0].blocked = false;
    },
    (p) => {
      p.deadlines[0].case_id = id(9);
    },
  ]) {
    const response = structuredClone(page);
    mutate(response);
    await assert.rejects(() => deadlinesApi(async () => response, id(1)).list({ limit: 1 }));
  }
  await assert.rejects(() => api.list({ limit: 101 }));
  await assert.rejects(() => api.list({ status: 'active', afterId: id(6), limit: 1 }));
  const people = {
    case_id: id(1),
    responsibles: [prepared().responsible],
    has_more: false,
    next_after_id: null,
  };
  assert.deepEqual(await deadlinesApi(async () => people, id(1)).responsibles(), people);
});
test('history validates immutable receipt headers and body budget before network', async () => {
  const row = history(),
    page = {
      case_id: id(1),
      id: id(6),
      revisions: [row],
      has_more: false,
      next_before_revision: null,
    };
  const api = deadlinesApi(async () => structuredClone(page), id(1));
  assert.deepEqual(await api.history(id(6)), page);
  page.revisions[0].state_digest = hash('1');
  await assert.rejects(() => api.history(id(6)));
  let calls = 0;
  const big = prepared().command;
  big.change.definition.title = 'x'.repeat(1048577);
  await assert.rejects(
    () =>
      deadlinesApi(async () => {
        calls++;
      }, id(1)).prepare(big, id(4)),
    { status: 413 },
  );
  assert.equal(calls, 0);
});
test('role and permission helpers distinguish absent records from lost case access', () => {
  assert.equal(canDeadlines('paralegal'), true);
  assert.equal(canDeadlines('client'), false);
  assert.equal(canDeadlines('litigator', 'manage'), true);
  assert.equal(canDeadlines('paralegal', 'manage'), false);
  assert.equal(deadlineDenied({ status: 404, code: 'deadline_not_found' }), false);
  assert.equal(deadlineDenied({ status: 404, code: 'case_not_found' }), true);
  assert.match(deadlineFailure({ code: 'deadline_revision_conflict' }), /borrador/);
});
