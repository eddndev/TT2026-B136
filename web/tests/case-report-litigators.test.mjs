import test from 'node:test';
import assert from 'node:assert/strict';
import { caseReportsApi } from '../src/lib/case-reports-api.mjs';
import { createApi } from '../src/lib/api.mjs';
const first = '83000000-0000-4000-8000-000000000001';
const second = '83000000-0000-4000-8000-000000000002';
const nil = '00000000-0000-0000-0000-000000000000';
const row = (user_id = first) => ({ user_id, email: 'closed.colleague@example.test' });
const page = (change = {}) => ({
  scope: 'assigned_cases',
  checked_at: '2026-09-27T12:00:00Z',
  litigators: [row()],
  has_more: false,
  next_after_id: null,
  ...change,
});
const client = (value) => caseReportsApi(async () => value);

test('report litigators use their own bounded directory and exact exclusive cursor', async () => {
  const calls = [];
  const value = page({ litigators: [row(second)] });
  const api = caseReportsApi(async (...args) => {
    calls.push(args);
    return value;
  });
  assert.deepEqual(await api.litigators(), value);
  assert.deepEqual(await api.litigators({ limit: 1, after_id: first }), value);
  assert.deepEqual(calls, [
    ['/case-reports/litigators?limit=20'],
    [`/case-reports/litigators?limit=1&after_id=${first}`],
  ]);
});

test('report litigator input rejects invalid limits cursors and unknown filters before transport', async () => {
  let calls = 0;
  const api = caseReportsApi(async () => {
    calls++;
    return page();
  });
  for (const query of [
    null,
    [],
    { limit: 0 },
    { limit: 101 },
    { limit: 1.5 },
    { limit: '1' },
    { after_id: nil },
    { after_id: 'bad' },
    { scope: 'office' },
    { unread_only: true },
  ])
    await assert.rejects(() => api.litigators(query));
  assert.equal(calls, 0);
});

test('report litigator pages reject secret fields invalid identities emails scope and clocks', async () => {
  const malformed = [
    page({ requester: 'private' }),
    page({ scope: 'private' }),
    page({ checked_at: '2026-02-30T12:00:00Z' }),
    ...[
      { user_id: nil },
      { email: 'UPPER@example.test' },
      { email: 'no-address' },
      { email: ' space@example.test' },
      { email: 'a\n@example.test' },
      { auth_generation: 2 },
      { active_cases: 0 },
    ].map((value) => page({ litigators: [{ ...row(), ...value }] })),
  ];
  for (const value of malformed) await assert.rejects(() => client(value).litigators());
});

test('report litigator pages preserve order bounds and exact continuation without implied totals', async () => {
  const value = page({ has_more: true, next_after_id: first });
  assert.deepEqual(await client(value).litigators({ limit: 1 }), value);
  assert.deepEqual(await client(page({ litigators: [] })).litigators(), page({ litigators: [] }));
  for (const bad of [
    page({ litigators: [row(), row()] }),
    page({ litigators: [row(second), row()] }),
    page({ has_more: 'true' }),
    page({ has_more: true }),
    page({ next_after_id: first }),
    page({ litigators: [], has_more: true, next_after_id: first }),
    page({ has_more: true, next_after_id: second }),
  ])
    await assert.rejects(() => client(bad).litigators());
  await assert.rejects(() =>
    client(page({ litigators: [row(), row(second)] })).litigators({ limit: 1 }),
  );
  await assert.rejects(() => client(page()).litigators({ after_id: first }));
});

test('disposed report litigator clients reject late pages and make no further requests', async () => {
  let release,
    calls = 0;
  const api = caseReportsApi(() => {
    calls++;
    return new Promise((resolve) => {
      release = resolve;
    });
  });
  const result = assert.rejects(api.litigators());
  api.dispose();
  release(page());
  await result;
  await assert.rejects(() => api.litigators());
  assert.equal(calls, 1);
});

test('report litigator factory rejects pages from a replaced authenticated session', async () => {
  let release;
  const calls = [];
  const api = createApi(async (url, options) => {
    calls.push({ url, ...options });
    if (url.endsWith('/totp')) return Response.json({ access_token: 'new', user: { id: first } });
    return new Promise((resolve) => {
      release = resolve;
    });
  });
  const result = assert.rejects(api.reports().litigators());
  await api.mfa('challenge', '123456', 'totp');
  release(Response.json(page()));
  await result;
  assert.equal(calls[0].cache, 'no-store');
});
