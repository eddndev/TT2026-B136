import test from 'node:test';
import assert from 'node:assert/strict';
import { alertsApi } from '../src/lib/alerts-api.mjs';
import {
  alertUserId,
  alertOtherId,
  alertId,
  alertInstant,
  alertPage,
  alertRecord,
  alertCursor,
  clone,
} from './fixtures/alerts.mjs';

function client(value) {
  const calls = [];
  return {
    calls,
    api: alertsApi(async (path) => {
      calls.push(path);
      return clone(value);
    }, alertUserId),
  };
}

test('the personal inbox uses its endpoint and every supported server filter', async () => {
  for (const read of ['all', 'unread'])
    for (const state of ['active', 'all']) {
      const expected = alertPage(),
        { api, calls } = client(expected);
      assert.deepEqual(await api.list({ read, state, limit: 7 }), expected);
      const url = new URL(calls[0], 'https://local.test');
      assert.equal(url.pathname, '/alerts');
      assert.equal(url.searchParams.get('read'), read);
      assert.equal(url.searchParams.get('state'), state);
      assert.equal(url.searchParams.get('limit'), '7');
    }
  const { api, calls } = client(alertPage());
  await api.list();
  const query = new URL(calls[0], 'https://local.test').searchParams;
  assert.equal(query.get('read'), 'all');
  assert.equal(query.get('state'), 'active');
  assert.equal(query.get('limit'), '20');
});

test('an empty bounded scan preserves its cursor and does not become a final empty inbox', async () => {
  const expected = alertPage([]);
  expected.has_more = true;
  expected.next_cursor = alertCursor(alertRecord());
  assert.deepEqual(await client(expected).api.list(), expected);
  const { api, calls } = client(alertPage([]));
  await api.list({ cursor: expected.next_cursor });
  assert.equal(
    new URL(calls[0], 'https://local.test').searchParams.get('cursor'),
    expected.next_cursor,
  );
});

test('invalid limits filters and noncanonical or rebound cursors never reach the network', async () => {
  const { api, calls } = client(alertPage());
  const token = alertCursor(alertRecord());
  for (const query of [
    { limit: 0 },
    { limit: 101 },
    { limit: '20' },
    { limit: 2.5 },
    { read: 'read' },
    { state: 'resolved' },
    { kind: 'hearing' },
    { recipient_id: alertOtherId },
    { cursor: '' },
    { cursor: 'opaque' },
    { cursor: 'a'.repeat(129) },
    { cursor: token.replace(':123456789:', ':12345678:') },
    { cursor: token.replace(':1767229200:', ':01767229200:') },
    { cursor: token, read: 'unread' },
    { cursor: token, state: 'all' },
    { cursor: token.replace(alertId, alertId.toUpperCase().replace('4000', '40AA')) },
  ])
    await assert.rejects(api.list(query));
  assert.equal(calls.length, 0);
});

test('inbox rows preserve exact descending time and id order without collapsing subjects', async () => {
  const first = alertRecord('review_required'),
    second = clone(first);
  first.trigger_at = second.trigger_at = alertInstant(1767229199);
  first.created_at = alertInstant(1767229200, 999999999);
  second.created_at = alertInstant(1767229200, 1);
  second.id = alertOtherId;
  second.occurrence_id = alertOtherId;
  const expected = alertPage([first, second]);
  assert.equal((await client(expected).api.list()).alerts.length, 2);
  expected.alerts.reverse();
  await assert.rejects(client(expected).api.list());
  first.created_at = clone(second.created_at);
  assert.equal((await client(alertPage([first, second])).api.list()).alerts.length, 2);
  await assert.rejects(client(alertPage([second, first])).api.list());
});

test('page validation rejects filter leaks repeated alerts and broken continuation', async () => {
  for (const change of [
    (v) => {
      v.alerts.push(clone(v.alerts[0]));
    },
    (v) => {
      v.extra = true;
    },
    (v) => {
      v.has_more = true;
    },
    (v) => {
      v.next_cursor = alertCursor(v.alerts[0]);
    },
    (v) => {
      v.has_more = true;
      const later = clone(v.alerts[0]);
      later.created_at.unix_seconds++;
      v.next_cursor = alertCursor(later);
    },
  ]) {
    const value = alertPage();
    change(value);
    await assert.rejects(client(value).api.list());
  }
  const read = alertRecord();
  read.read_at = alertInstant(1767230000);
  await assert.rejects(client(alertPage([read])).api.list({ read: 'unread' }));
  const resolved = alertRecord();
  resolved.state = { kind: 'resolved', at: alertInstant(1767230000), reason: 'superseded' };
  await assert.rejects(client(alertPage([resolved])).api.list({ state: 'active' }));
  const repeated = alertPage();
  await assert.rejects(client(repeated).api.list({ cursor: alertCursor(repeated.alerts[0]) }));
});

test('disposing a personal inbox invalidates pending results and all subsequent calls', async () => {
  let release;
  const api = alertsApi(
    () =>
      new Promise((resolve) => {
        release = resolve;
      }),
    alertUserId,
  );
  const pending = api.list().then(
    (result) => ({ result }),
    (error) => ({ error }),
  );
  api.dispose();
  if (release) release(alertPage());
  const outcome = await pending;
  assert.equal(typeof release, 'function');
  assert.ok(outcome.error);
  await assert.rejects(api.list());
  await assert.rejects(api.get(alertId));
  await assert.rejects(api.preferences());
});

test('an empty partial scan cannot continue from a creation time after its observation', async () => {
  const value = alertPage([]),
    row = alertRecord();
  value.has_more = true;
  row.created_at = clone(value.checked_at);
  value.next_cursor = alertCursor(row);
  assert.deepEqual(await client(value).api.list(), value);
  row.created_at.unix_seconds++;
  value.next_cursor = alertCursor(row);
  await assert.rejects(client(value).api.list());
});
