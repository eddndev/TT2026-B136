import test from 'node:test';
import assert from 'node:assert/strict';
import { deadlinesApi } from '../src/lib/deadline-api.mjs';
import {
  clone,
  v2Prepared,
  v2Record,
  v1Record,
  technicalRecord,
  historyRow,
  ids,
  digest,
} from './fixtures/deadline-v2-unit.mjs';

const principal = () => ({ id: ids(4), email: 'staff@example.test', role: 'owner' });

test('preparation checks captured user id and email against the authenticated principal', async () => {
  const p = v2Prepared();
  const calls = [];
  const api = deadlinesApi(async (path, options) => {
    calls.push([path, options]);
    return clone(p);
  }, ids(1));
  assert.deepEqual(await api.prepare(p.command, principal()), p);
  assert.equal(calls[0][0], `/cases/${ids(1)}/deadlines/prepare`);
  assert.deepEqual(calls[0][1].data, p.command);
  for (const author of [
    { kind: 'user', id: ids(4), email: 'old@example.test' },
    { kind: 'user', id: ids(9), email: 'staff@example.test' },
    { kind: 'technical', service: 'deadline_reevaluator', policy_version: 1 },
  ]) {
    const response = clone(p);
    response.author = author;
    const client = deadlinesApi(async () => response, ids(1));
    await assert.rejects(() => client.prepare(p.command, principal()));
  }
});

test('mixed history retains V1 and technical V2 receipts and validates visible predecessor links', async () => {
  const legacy = v1Record();
  const technical = technicalRecord();
  const page = {
    case_id: ids(1),
    id: ids(6),
    revisions: [historyRow(technical), historyRow(legacy)],
    has_more: false,
    next_before_revision: null,
  };
  const api = deadlinesApi(async () => clone(page), ids(1));
  assert.deepEqual(await api.history(ids(6)), page);
  for (const mutate of [
    (p) => {
      p.revisions[0].receipt.version.predecessor.capture_digest = digest('1');
    },
    (p) => {
      p.revisions[0].receipt.version.predecessor.submission_digest = digest('1');
    },
    (p) => {
      p.revisions[0].receipt.operation_id = p.revisions[1].receipt.operation_id;
    },
    (p) => {
      p.revisions[0].state_digest = digest('1');
    },
    (p) => {
      p.revisions[1].receipt.version = { kind: 'v2' };
    },
  ]) {
    const next = clone(page);
    mutate(next);
    await assert.rejects(() => deadlinesApi(async () => next, ids(1)).history(ids(6)));
  }
  const newerLegacy = v2Record(v2Prepared('correct'));
  newerLegacy.receipt.version = { kind: 'v1' };
  newerLegacy.tracking = null;
  newerLegacy.receipt.operation_id = ids(7);
  const downgraded = clone(page);
  downgraded.revisions = [historyRow(newerLegacy), historyRow(v2Record())];
  await assert.rejects(() => deadlinesApi(async () => downgraded, ids(1)).history(ids(6)));
});

test('mutation paths remain unchanged and never send captured authorship as intent', async () => {
  for (const [action, tail, method] of [
    ['register', '', 'POST'],
    ['correct', `/${ids(6)}`, 'PUT'],
    ['set_attention', `/${ids(6)}/attention`, 'POST'],
    ['retire', `/${ids(6)}/retirement`, 'POST'],
  ]) {
    const p = v2Prepared(action);
    const calls = [];
    const api = deadlinesApi(async (path, options) => {
      calls.push([path, options]);
      return v2Record(p);
    }, ids(1));
    assert.deepEqual(await api.submit(p), v2Record(p));
    assert.equal(calls[0][0], `/cases/${ids(1)}/deadlines${tail}`);
    assert.equal(calls[0][1].method, method);
    assert.deepEqual(calls[0][1].data, {
      command: p.command,
      expected_submission_digest: p.submission_digest,
    });
    assert.equal(calls.length, 1);
  }
});

test('disposal rejects a late V2 preparation before its data becomes reusable', async () => {
  const p = v2Prepared();
  let resolve;
  let calls = 0;
  const api = deadlinesApi(() => {
    calls++;
    return new Promise((done) => {
      resolve = done;
    });
  }, ids(1));
  const waiting = api.prepare(p.command, principal());
  api.dispose();
  resolve(clone(p));
  await assert.rejects(() => waiting);
  await assert.rejects(() => api.submit(p));
  assert.equal(calls, 1);
});
