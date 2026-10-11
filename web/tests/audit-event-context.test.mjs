import test from 'node:test';
import assert from 'node:assert/strict';
import { auditEventsApi } from '../src/lib/audit-events-api.mjs';

const account = '20000000-0000-4000-8000-000000000001';
const request = '30000000-0000-4000-8000-000000000001';
const query = {
  from: '2026-10-01T00:00:00Z',
  until: '2026-10-02T00:00:00Z',
  limit: 1,
};
const context = (change = {}) => ({
  user_id: account,
  ip_address: '192.0.2.17',
  source: 'http',
  request_id: request,
  ...change,
});
const event = (change = {}) => ({
  sequence: '9007199254740993',
  timestamp: '2026-10-01T12:00:00.123456789Z',
  actor: 'captured-owner@example.test',
  action: 'access.granted',
  resource: 'GET /api/v1/cases',
  context: context(),
  ...change,
});
const page = (row = event(), change = {}) => ({
  checked_at: '2026-10-02T01:00:00Z',
  snapshot_max_sequence: '9223372036854775807',
  events: [row],
  has_more: false,
  next_cursor: null,
  ...change,
});
const read = (value, selection = query) => auditEventsApi(async () => value).list(selection);

test('legacy context remains explicitly unavailable without resolving the captured actor', async () => {
  const value = page(event({ context: null, action: 'document.read' }));
  assert.deepEqual(await read(value), value);
  assert.equal(value.events[0].actor, 'captured-owner@example.test');
  assert.equal(value.events[0].context, null);
});

test('HTTP context preserves account, request and literal IPv4 or IPv6 provenance', async () => {
  for (const ip_address of ['192.0.2.17', '2001:db8::17', '::1', '::ffff:192.0.2.17']) {
    const value = page(event({ context: context({ ip_address }) }));
    assert.deepEqual(await read(value), value);
  }
});

test('unidentified HTTP, CLI and internal contexts do not acquire a fabricated account or IP', async () => {
  for (const source of ['http', 'cli', 'internal']) {
    const value = page(
      event({ context: context({ source, user_id: null, ip_address: null, request_id: null }) }),
    );
    assert.deepEqual(await read(value), value);
  }
});

test('context is an exact typed object or null and unknown metadata fails closed', async () => {
  const incomplete = context();
  delete incomplete.request_id;
  for (const value of [
    undefined,
    [],
    'http',
    1,
    {},
    incomplete,
    context({ secret: 'must-not-reach-the-view' }),
    context({ outcome: 'granted' }),
    context({ source: 'browser' }),
    context({ source: 'HTTP' }),
    context({ source: null }),
    context({ user_id: 'captured-owner@example.test' }),
    context({ user_id: '20000000000040008000000000000001' }),
    context({ user_id: 1 }),
    context({ request_id: 'not-a-request-uuid' }),
    context({ request_id: [] }),
    context({ ip_address: 127001 }),
    context({ ip_address: [] }),
  ])
    await assert.rejects(() => read(page(event({ context: value }))));
});

test('IP provenance rejects hostnames, forwarding chains, ports, zones and malformed literals', async () => {
  for (const ip_address of [
    '',
    'localhost',
    '192.0.2.17, 198.51.100.8',
    '192.0.2.17:443',
    '192.0.2.999',
    '192.00.2.17',
    '127.1',
    ' 192.0.2.17',
    '192.0.2.17\n',
    'https://192.0.2.17',
    '[2001:db8::17]',
    'fe80::1%eth0',
    '2001:db8:::17',
    '2001:db8::g',
    '1:2:3:4:5:6:7',
    '1:2:3:4:5:6:7:8:9',
  ])
    await assert.rejects(() => read(page(event({ context: context({ ip_address }) }))));
});

test('the existing actor filter accepts a verified UUID without adding an API parameter', async () => {
  const calls = [];
  const value = page();
  const client = auditEventsApi(async (path) => {
    calls.push(new URL(path, 'https://example.test'));
    return value;
  });
  assert.deepEqual(await client.list({ ...query, actor: account }), value);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].pathname, '/audit/events');
  assert.equal(calls[0].searchParams.get('actor'), account);
  assert.deepEqual([...calls[0].searchParams.keys()].sort(), ['actor', 'from', 'limit', 'until']);
});

test('actor matching retains exact recorded text and never invents an identity for legacy rows', async () => {
  const historical = page(event({ context: null }));
  assert.deepEqual(
    await read(historical, { ...query, actor: 'captured-owner@example.test' }),
    historical,
  );
  await assert.rejects(() => read(historical, { ...query, actor: account }));
  await assert.rejects(() =>
    read(page(event({ context: context({ user_id: null }) })), { ...query, actor: account }),
  );
  await assert.rejects(() =>
    read(page(), { ...query, actor: '20000000-0000-4000-8000-000000000002' }),
  );
  await assert.rejects(() => read(page(), { ...query, actor: 'CAPTURED-OWNER@example.test' }));
  const textualUuid = page(event({ actor: account, context: null }));
  assert.deepEqual(await read(textualUuid, { ...query, actor: account }), textualUuid);
});

test('UUID actor pagination preserves its exact filter while captured labels can differ', async () => {
  const values = [
    page(event(), { has_more: true, next_cursor: 'opaque-context-page' }),
    page(
      event({
        sequence: '9007199254740994',
        timestamp: '2026-10-01T12:00:00.123456790Z',
        actor: 'another-captured-label@example.test',
      }),
    ),
  ];
  const calls = [];
  const client = auditEventsApi(async (path) => {
    calls.push(new URL(path, 'https://example.test'));
    return values.shift();
  });
  const first = await client.list({ ...query, actor: account });
  const second = await client.list({ ...query, actor: account, cursor: first.next_cursor });
  assert.equal(second.events[0].actor, 'another-captured-label@example.test');
  assert.equal(calls[1].searchParams.get('actor'), account);
  assert.equal(calls[1].searchParams.get('cursor'), 'opaque-context-page');
});

test('typed context counts toward the page text budget without charging legacy null', async () => {
  const encoder = new TextEncoder();
  for (const metadata of [null, context()]) {
    const row = event({ context: metadata, resource: '' });
    const fixed = encoder.encode(row.actor + row.action).byteLength;
    const metadataBytes =
      metadata === null ? 0 : encoder.encode(JSON.stringify(metadata)).byteLength;
    row.resource = 'x'.repeat(262144 - fixed - metadataBytes);
    const value = page(row);
    assert.deepEqual(await read(value), value);
    await assert.rejects(() => read(page({ ...row, resource: `${row.resource}x` })));
  }
});
