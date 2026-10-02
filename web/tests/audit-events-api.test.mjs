import test from 'node:test';
import assert from 'node:assert/strict';
import { auditEventsApi } from '../src/lib/audit-events-api.mjs';
import { createApi } from '../src/lib/api.mjs';
const query = {
  from: '2026-10-01T00:00:00Z',
  until: '2026-10-02T00:00:00Z',
  limit: 1,
};
const row = (change = {}) => ({
  sequence: '9007199254740993',
  timestamp: '2026-10-01T01:02:03.123456789Z',
  actor: 'system',
  action: 'read',
  resource: 'record',
  ...change,
});
const page = (change = {}) => ({
  checked_at: '2026-10-02T01:00:00Z',
  snapshot_max_sequence: '9223372036854775807',
  events: [row()],
  has_more: false,
  next_cursor: null,
  ...change,
});

test('audit scope uses common current bearer transport and exact UTF8 filters', async () => {
  const calls = [];
  const api = createApi(async (url, options) => {
    calls.push({ url, options });
    if (url.endsWith('/totp'))
      return Response.json({
        access_token: 'owner-token',
        user: { id: 'owner' },
      });
    return Response.json(page({ events: [row({ actor: ' Owner\u00e9 ', resource: 'a:b/x' })] }));
  });
  await api.mfa('challenge', '123456', 'totp');
  const value = await api
    .auditEvents()
    .list({ ...query, actor: ' Owner\u00e9 ', resource: 'a:b/x' });
  assert.equal(value.events[0].sequence, '9007199254740993');
  const call = calls[1],
    url = new URL(call.url, 'https://example.test');
  assert.equal(url.pathname, '/api/v1/audit/events');
  assert.equal(url.searchParams.get('actor'), ' Owner\u00e9 ');
  assert.equal(url.searchParams.get('resource'), 'a:b/x');
  assert.equal(call.options.headers.Authorization, 'Bearer owner-token');
  assert.equal(call.options.cache, 'no-store');
});
test('audit input validation prevents transport for malformed or unbounded queries', async () => {
  let calls = 0;
  const api = auditEventsApi(async () => {
    calls++;
    return page();
  });
  for (const bad of [
    { ...query, unknown: 1 },
    { ...query, from: '2026-02-30T00:00:00Z' },
    { ...query, from: '2026-10-01' },
    { ...query, until: query.from },
    { ...query, until: '2027-10-03T00:00:00Z' },
    { ...query, from: '2026-10-01T00:00:60Z' },
    { ...query, limit: 0 },
    { ...query, limit: 101 },
    { ...query, limit: '1' },
    { ...query, actor: '' },
    { ...query, actor: 'x\n' },
    { ...query, actor: '\u00e9'.repeat(128) },
    { ...query, action: 'a'.repeat(129) },
    { ...query, resource: 'r'.repeat(1025) },
    { ...query, cursor: 'x'.repeat(4097) },
  ])
    await assert.rejects(() => api.list(bad));
  assert.equal(calls, 0);
});
test('audit page keeps exact timestamp and integer strings beyond Number precision', async () => {
  const value = page({ events: [row({ sequence: '9223372036854775807' })] });
  assert.deepEqual(await auditEventsApi(async () => value).list(query), value);
  const empty = page({ events: [], snapshot_max_sequence: null });
  assert.deepEqual(await auditEventsApi(async () => empty).list(query), empty);
});
test('next page preserves snapshot and exact nanosecond chronological ordering', async () => {
  const values = [
    page({ has_more: true, next_cursor: 'opaque-first' }),
    page({
      events: [row({ sequence: '0', timestamp: '2026-10-01T01:02:03.123456790Z' })],
    }),
  ];
  const paths = [];
  const client = auditEventsApi(async (path) => {
    paths.push(path);
    return values.shift();
  });
  const first = await client.list(query);
  const second = await client.list({ ...query, cursor: first.next_cursor });
  assert.equal(second.events[0].sequence, '0');
  assert.equal(
    new URL(paths[1], 'https://example.test').searchParams.get('cursor'),
    'opaque-first',
  );
});
test('client rejects changed filters and unknown cursors before transport', async () => {
  let calls = 0;
  const client = auditEventsApi(async () => {
    calls++;
    return page({ has_more: true, next_cursor: 'opaque' });
  });
  await client.list(query);
  await assert.rejects(() => client.list({ ...query, actor: 'other', cursor: 'opaque' }));
  await assert.rejects(() => client.list({ ...query, cursor: 'invented' }));
  assert.equal(calls, 1);
});
test('snapshot replacement, replayed chronology and duplicate sequences are rejected', async () => {
  for (const change of [
    { snapshot_max_sequence: '9223372036854775806' },
    { events: [row()] },
    {
      events: [row({ sequence: '0', timestamp: '2026-10-01T01:02:03.123456788Z' })],
    },
  ]) {
    let count = 0;
    const client = auditEventsApi(async () =>
      ++count === 1 ? page({ has_more: true, next_cursor: 'next' }) : page(change),
    );
    await client.list(query);
    await assert.rejects(() => client.list({ ...query, cursor: 'next' }));
  }
  await assert.rejects(() =>
    auditEventsApi(async () =>
      page({
        events: [row(), row({ timestamp: '2026-10-01T01:02:03.123456790Z' })],
      }),
    ).list({ ...query, limit: 2 }),
  );
});
test('malformed response fields, hidden fields, integer coercion and invalid dates fail closed', async () => {
  for (const value of [
    page({ secret: 'hidden' }),
    page({ checked_at: '2026-02-30T00:00:00Z' }),
    page({ snapshot_max_sequence: 42 }),
    page({ snapshot_max_sequence: '9223372036854775808' }),
    page({ events: [row({ sequence: '01' })] }),
    page({ events: [row({ sequence: 9007199254740992 })] }),
    page({ events: [row({ ip: 'invented' })] }),
    page({ events: [row({ timestamp: '2026-10-02T00:00:00Z' })] }),
    page({ has_more: true, next_cursor: null }),
    page({ events: [], has_more: true, next_cursor: 'next' }),
    page({ next_cursor: 'unexpected' }),
    page({ events: [row(), row({ sequence: '2' })] }),
  ])
    await assert.rejects(() => auditEventsApi(async () => value).list(query));
});
test('historical strings exceed filter limits but aggregate text remains bounded', async () => {
  const value = page({
    events: [
      row({
        actor: 'a'.repeat(255),
        action: 'b'.repeat(129),
        resource: '<script>\u00e9\n'.repeat(200),
      }),
    ],
  });
  assert.deepEqual(await auditEventsApi(async () => value).list(query), value);
  await assert.rejects(() =>
    auditEventsApi(async () => page({ events: [row({ resource: 'x'.repeat(262144) })] })).list(
      query,
    ),
  );
});
test('disposed scopes reject late results and perform no further requests', async () => {
  let release,
    calls = 0;
  const client = auditEventsApi(() => {
    calls++;
    return new Promise((resolve) => (release = resolve));
  });
  const pending = assert.rejects(client.list(query));
  client.dispose();
  release(page());
  await pending;
  await assert.rejects(() => client.list(query));
  assert.equal(calls, 1);
});
test('new session prevents delivery from previous account', async () => {
  let release;
  const api = createApi(async (url) =>
    url.endsWith('/totp')
      ? Response.json({ access_token: 'new', user: { id: 'new' } })
      : new Promise((resolve) => (release = resolve)),
  );
  const pending = assert.rejects(api.auditEvents().list(query));
  await api.mfa('challenge', '123456', 'totp');
  release(Response.json(page()));
  await pending;
});
test('capacity uses activity guidance and preserves status without automatic retry', async () => {
  let calls = 0;
  const api = createApi(async () => {
    calls++;
    return Response.json({ error: { code: 'audit_query_capacity_exceeded' } }, { status: 413 });
  });
  await assert.rejects(
    () => api.auditEvents().list(query),
    (error) =>
      error.status === 413 &&
      error.message.includes('actividad') &&
      !error.message.includes('16 MiB'),
  );
  assert.equal(calls, 1);
});
test('caller mutations during transport cannot rewrite page validation expectations', async () => {
  let release;
  const input = { ...query };
  const client = auditEventsApi(() => new Promise((resolve) => (release = resolve)));
  const pending = client.list(input);
  input.from = '2027-01-01T00:00:00Z';
  input.actor = 'other';
  release(page());
  assert.deepEqual(await pending, page());
});
