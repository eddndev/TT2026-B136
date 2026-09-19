import test from 'node:test';
import assert from 'node:assert/strict';
import { combinedAgendaApi } from '../src/lib/combined-agenda-api.mjs';
import { hearingRow } from './fixtures/hearings.mjs';
import { summary, v2Record, timedPrepared, clone } from './fixtures/deadline-v2-unit.mjs';

const query = { from: '2026-01-01T00:00:00Z', until: '2026-01-03T00:00:00Z' };
const instant = (seconds, nanosecond = 0) => ({
  unix_seconds: seconds,
  nanosecond,
  offset_seconds: 0,
});
const checked = instant(1767225602);
function page() {
  const hearing = hearingRow();
  hearing.scheduled_at = '2026-01-01T01:00:00Z';
  const deadline = summary(v2Record(timedPrepared()));
  deadline.operational = {
    freshness: 'current',
    checked_at: clone(checked),
    changed_dependencies: [],
    due_at: clone(deadline.calculation_due_at),
  };
  return {
    ...query,
    kind: 'all',
    hearing_status: 'scheduled',
    checked_at: clone(checked),
    items: [
      { kind: 'hearing', at: instant(1767229200), hearing },
      {
        kind: 'deadline',
        at: clone(deadline.operational.due_at),
        case_title: 'Authorized case',
        case_reference: 'REF-1',
        case_status: 'active',
        deadline,
      },
    ],
    complete: true,
    next_cursor: null,
  };
}
function client(value) {
  const calls = [];
  return {
    calls,
    api: combinedAgendaApi(async (path) => {
      calls.push(path);
      return clone(value);
    }),
  };
}

test('combined agenda uses one endpoint and preserves both resource families', async () => {
  const expected = page(),
    { api, calls } = client(expected);
  assert.deepEqual(await api.list(query), expected);
  const url = new URL(calls[0], 'https://local.test');
  assert.equal(url.pathname, '/agenda');
  assert.equal(url.searchParams.get('kind'), 'all');
  assert.equal(url.searchParams.get('hearing_status'), 'scheduled');
  assert.equal(url.searchParams.get('limit'), '20');
  assert.equal(calls.length, 1);
});

test('complete and partial empty pages have distinct explicit continuations', async () => {
  const value = page();
  value.items = [];
  value.complete = false;
  value.next_cursor =
    'a1:1767225600:1767398400:all:scheduled:1767312000:9:1:00000000-0000-0000-0000-000000000002';
  const { api } = client(value);
  assert.equal((await api.list(query)).complete, false);
  value.complete = true;
  value.next_cursor = null;
  assert.equal((await client(value).api.list(query)).complete, true);
});

test('query rejects invalid range, limits, filters and cursor before requesting', async () => {
  const { api, calls } = client(page());
  for (const mutation of [
    { limit: 0 },
    { limit: 101 },
    { kind: 'other' },
    { hearing_status: 'pending' },
    { kind: 'deadline', hearing_status: 'cancelled' },
    { from: '2026-02-30T00:00:00Z' },
    { until: query.from },
    { until: '2028-01-01T00:00:00Z' },
    { cursor: '' },
    { cursor: 'broken' },
  ])
    await assert.rejects(api.list({ ...query, ...mutation }));
  assert.equal(calls.length, 0);
});

test('response rejects range mismatch, unknown fields and inconsistent continuation', async () => {
  for (const change of [
    (v) => {
      v.from = '2026-01-02T00:00:00Z';
    },
    (v) => {
      v.extra = true;
    },
    (v) => {
      v.complete = false;
    },
    (v) => {
      v.next_cursor = 'broken';
    },
    (v) => {
      v.kind = 'deadline';
    },
    (v) => {
      v.items.reverse();
    },
    (v) => {
      v.items.push(clone(v.items[1]));
    },
    (v) => {
      v.items[0].at.unix_seconds = 1767398400;
    },
    (v) => {
      v.items[0].at.offset_seconds = 3600;
    },
    (v) => {
      v.items[0].at.nanosecond = -1;
    },
  ]) {
    const value = page();
    change(value);
    await assert.rejects(client(value).api.list(query));
  }
});

test('deadline agenda refuses pending, stale, retired, legacy and unavailable operational dates', async () => {
  for (const change of [
    (d) => {
      d.review_state = 'pending';
    },
    (d) => {
      d.status = 'retired';
    },
    (d) => {
      d.receipt_kind = 'v1';
    },
    (d) => {
      d.operational.freshness = 'changed';
    },
    (d) => {
      d.operational.due_at = null;
    },
    (d) => {
      d.operational.checked_at.unix_seconds++;
    },
    (d) => {
      d.operational.due_at.unix_seconds++;
    },
  ]) {
    const value = page();
    change(value.items[1].deadline);
    await assert.rejects(client(value).api.list(query));
  }
});

test('common ordering instant must be derived from its resource and respects nanoseconds', async () => {
  const value = page();
  value.items[1].at.nanosecond = 1;
  await assert.rejects(client(value).api.list(query));
  value.items[1].deadline.operational.due_at.nanosecond = 1;
  value.items[1].deadline.calculation_due_at.nanosecond = 1;
  assert.deepEqual((await client(value).api.list(query)).items[1].at, instant(1767312000, 1));
  const mismatch = page();
  mismatch.items[0].hearing.scheduled_at = '2026-01-01T02:00:00Z';
  await assert.rejects(client(mismatch).api.list(query));
});

test('same instant and UUID retain hearing before deadline without merging families', async () => {
  const value = page(),
    at = instant(1767312000);
  value.items[0].at = clone(at);
  value.items[0].hearing.scheduled_at = '2026-01-02T00:00:00Z';
  value.items[0].hearing.id = value.items[1].deadline.id;
  assert.equal((await client(value).api.list(query)).items.length, 2);
  value.items.reverse();
  await assert.rejects(client(value).api.list(query));
});

test('disposing the agenda invalidates delayed results and future requests', async () => {
  let release;
  const api = combinedAgendaApi(
    () =>
      new Promise((resolve) => {
        release = resolve;
      }),
  );
  const pending = api.list(query);
  api.dispose();
  assert.equal(typeof release, 'function');
  release(page());
  await assert.rejects(pending);
  await assert.rejects(api.list(query));
});
