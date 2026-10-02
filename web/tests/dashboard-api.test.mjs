import test from 'node:test';
import assert from 'node:assert/strict';
import { dashboardApi } from '../src/lib/dashboard-api.mjs';
import { createApi } from '../src/lib/api.mjs';

const userId = '11111111-1111-4111-8111-111111111111';
const summary = (change = {}) => ({
  checked_at: '2026-09-27T10:15:30.123456789Z',
  scope: 'office',
  active_cases: 9,
  pending_contracts: 4,
  deadlines_overdue: 2,
  deadlines_due_48h: 3,
  deadlines_due_7d: 7,
  deadlines_unresolved: 1,
  workload: [{ user_id: userId, email: 'lawyer@example.test', active_cases: 6 }],
  ...change,
});
const scoped = (value) => dashboardApi(async () => value);

test('dashboard requests authorized complete aggregates without pagination or local estimates', async () => {
  const calls = [];
  const client = createApi(async (url, options) => {
    calls.push({ url, ...options });
    return Response.json(summary());
  }).dashboard();
  assert.deepEqual(await client.get(), summary());
  assert.equal(calls.length, 1);
  assert.equal(calls[0].url, '/api/v1/dashboard');
  assert.equal(calls[0].method, 'GET');
  assert.equal(calls[0].cache, 'no-store');
});

test('dashboard retains exact timestamps, assigned scope and legitimate empty aggregates', async () => {
  for (const checked_at of ['2026-09-27T10:15:30Z', '2026-09-27T04:15:30.12-06:00']) {
    const value = summary({ checked_at, scope: 'assigned_cases' });
    assert.deepEqual(await scoped(value).get(), value);
  }
  const empty = summary({
    active_cases: 0,
    pending_contracts: 0,
    deadlines_overdue: 0,
    deadlines_due_48h: 0,
    deadlines_due_7d: 0,
    deadlines_unresolved: 0,
    workload: [],
  });
  assert.deepEqual(await scoped(empty).get(), empty);
});

test('dashboard rejects impossible or imprecise counts and an inconsistent inclusive window', async () => {
  const keys = [
    'active_cases',
    'pending_contracts',
    'deadlines_overdue',
    'deadlines_due_48h',
    'deadlines_due_7d',
    'deadlines_unresolved',
  ];
  for (const key of keys)
    for (const value of [-1, 1.5, '3', null, Number.MAX_SAFE_INTEGER + 1])
      await assert.rejects(() => scoped(summary({ [key]: value })).get());
  await assert.rejects(() => scoped(summary({ deadlines_due_48h: 8 })).get());
});

test('dashboard rejects malformed envelopes, hidden fields, invalid calendar instants and identities', async () => {
  for (const change of [
    { scope: 'all' },
    { total: 9 },
    { checked_at: '2026-02-30T10:00:00Z' },
    { checked_at: '2026-09-27' },
    { checked_at: '2026-09-27T25:00:00Z' },
    { workload: null },
    { workload: [{ user_id: 'other', email: 'lawyer@example.test', active_cases: 1 }] },
    { workload: [{ user_id: userId, email: 'missing-at', active_cases: 1 }] },
    { workload: [{ user_id: userId, email: 'lawyer@example.test', active_cases: 10 }] },
    {
      workload: [
        { user_id: userId, email: 'lawyer@example.test', active_cases: 1, password_hash: 'secret' },
      ],
    },
  ])
    await assert.rejects(() => scoped(summary(change)).get());
  const duplicate = summary();
  duplicate.workload.push({ ...duplicate.workload[0] });
  await assert.rejects(() => scoped(duplicate).get());
  const missing = summary();
  delete missing.deadlines_unresolved;
  await assert.rejects(() => scoped(missing).get());
});

test('dashboard preserves request errors without retrying or inventing empty metrics', async () => {
  const failure = Object.assign(new Error('Permission denied'), { status: 403 });
  let calls = 0;
  const client = dashboardApi(async () => {
    calls++;
    throw failure;
  });
  await assert.rejects(
    () => client.get(),
    (error) => error === failure,
  );
  assert.equal(calls, 1);
});

test('disposed dashboard scopes reject both late responses and additional transport', async () => {
  let release,
    calls = 0;
  const client = dashboardApi(() => {
    calls++;
    return new Promise((resolve) => {
      release = resolve;
    });
  });
  const pending = assert.rejects(client.get());
  client.dispose();
  release(summary());
  await pending;
  await assert.rejects(() => client.get());
  assert.equal(calls, 1);
});

test('a dashboard response from a replaced session cannot reach the new account', async () => {
  let release;
  const api = createApi(async (url) => {
    if (url.endsWith('/totp'))
      return Response.json({ access_token: 'new', user: { id: 'new-user' } });
    return new Promise((resolve) => {
      release = resolve;
    });
  });
  const pending = assert.rejects(api.dashboard().get());
  await api.mfa('challenge', '123456', 'totp');
  release(Response.json(summary()));
  await pending;
});
