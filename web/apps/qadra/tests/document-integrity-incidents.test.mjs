import test from 'node:test';
import assert from 'node:assert/strict';
import { createApi } from '../src/lib/api.mjs';
import {
  incidentId,
  incidentRecord,
  incidentPage,
} from './fixtures/document-integrity-incidents.mjs';

const base = '/api/v1/document-integrity-incidents';
const client = (value) => createApi(async () => Response.json(value)).integrityIncidents();

test('incident listing binds explicit pagination and keeps UUID order independently of time', async () => {
  const calls = [],
    first = incidentRecord(1),
    second = incidentRecord(2);
  first.detected_at = '2026-09-20T10:00:00Z';
  first.recorded_at = first.detected_at;
  const api = createApi(async (url, options) => {
    calls.push({ url, ...options });
    return Response.json(incidentPage([first, second], true));
  }).integrityIncidents();
  const result = await api.list();
  assert.deepEqual(result.incidents, [first, second]);
  assert.equal(result.next_after_id, second.id);
  assert.equal(calls[0].url, `${base}?limit=50`);
  assert.equal(calls[0].method, 'GET');
  assert.equal(calls[0].cache, 'no-store');
  const after = createApi(async (url) => {
    assert.equal(url, `${base}?limit=1&after_id=${second.id}`);
    return Response.json(incidentPage([incidentRecord(3)]));
  }).integrityIncidents();
  assert.equal(
    (await after.list({ limit: 1, after_id: second.id })).incidents[0].id,
    incidentId(3),
  );
});

test('incident detail is a direct exact record and accepts the four typed failures', async () => {
  for (const failure of [
    'malformed_vault',
    'authentication_failed',
    'digest_mismatch',
    'snapshot_changed',
  ]) {
    const row = incidentRecord(1, failure);
    const api = createApi(async (url) => {
      assert.equal(url, `${base}/${row.id}`);
      return Response.json(row);
    }).integrityIncidents();
    assert.deepEqual(await api.get(row.id), row);
  }
  await assert.rejects(() => client(incidentRecord(2)).get(incidentId(1)), /incidente|respuesta/i);
  await assert.rejects(
    () => client({ incident: incidentRecord() }).get(incidentId(1)),
    /incidente|respuesta/i,
  );
});

test('incident queries reject invalid limits, UUIDs and unknown filters without requests', async () => {
  let calls = 0;
  const api = createApi(async () => {
    calls++;
    return Response.json(incidentPage());
  }).integrityIncidents();
  for (const query of [
    { limit: 0 },
    { limit: 101 },
    { limit: 1.5 },
    { limit: '1' },
    { after_id: 'not-a-uuid' },
    { after_id: 'AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA' },
    { state: 'active' },
    { read: 'unread' },
  ])
    await assert.rejects(() => api.list(query));
  await assert.rejects(() => api.get('../documents'));
  assert.equal(calls, 0);
});

test('incident pages enforce exclusive cursors, complete pagination and a bounded sorted collection', async () => {
  const a = incidentRecord(1),
    b = incidentRecord(2);
  const invalid = [
    incidentPage([b, a]),
    incidentPage([a, a]),
    { ...incidentPage([a]), next_after_id: a.id },
    { ...incidentPage([a], true), next_after_id: b.id },
    { incidents: [], has_more: true, next_after_id: a.id },
    { ...incidentPage(), has_more: 'false' },
    { ...incidentPage(), total: 0 },
  ];
  for (const value of invalid)
    await assert.rejects(() => client(value).list(), /incidente|respuesta/i);
  await assert.rejects(
    () => client(incidentPage([a])).list({ after_id: a.id }),
    /incidente|respuesta/i,
  );
  await assert.rejects(
    () => client(incidentPage([a, b])).list({ limit: 1 }),
    /incidente|respuesta/i,
  );
  assert.deepEqual(await client(incidentPage()).list(), incidentPage());
});

test('incident fields retain exact identities and valid digests without fabricating a cause', async () => {
  const mutations = [
    (v) => delete v.observation_id,
    (v) => {
      v.id = 'INVALID';
    },
    (v) => {
      v.case_id = 'AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA';
    },
    (v) => {
      v.requester_id = null;
    },
    (v) => {
      v.document_id = 'foreign';
    },
    (v) => {
      v.document_version = 0;
    },
    (v) => {
      v.document_version = 4294967296;
    },
    (v) => {
      v.document_version = '1';
    },
    (v) => {
      v.failure = 'attack';
    },
    (v) => {
      v.expected_digest = 'A'.repeat(64);
    },
    (v) => {
      v.observed_snapshot_digest = 'short';
    },
    (v) => {
      v.responsible_person = v.requester_id;
    },
  ];
  for (const mutate of mutations) {
    const row = incidentRecord();
    mutate(row);
    await assert.rejects(() => client(row).get(incidentId(1)), /incidente|respuesta/i);
  }
  const row = incidentRecord();
  row.document_version = 4294967295;
  row.observed_snapshot_digest = row.expected_digest;
  assert.deepEqual(await client(row).get(row.id), row);
});

test('incident times are valid UTC RFC3339 and preserve ordering below one millisecond', async () => {
  for (const value of [
    '2026-02-30T10:00:00Z',
    '2026-09-19T10:00:00+01:00',
    '2026-09-19T10:00:00-00:00',
    '0000-01-01T00:00:00Z',
    '2026-09-19T10:00:00.1234567891Z',
    '2026-09-19',
  ]) {
    const row = incidentRecord();
    row.detected_at = value;
    await assert.rejects(() => client(row).get(row.id), /incidente|respuesta/i);
  }
  const reversed = incidentRecord();
  reversed.detected_at = '2026-09-19T10:00:00.123456790Z';
  await assert.rejects(() => client(reversed).get(reversed.id), /incidente|respuesta/i);
  const valid = incidentRecord();
  valid.detected_at = '2024-02-29T00:00:00+00:00';
  assert.deepEqual(await client(valid).get(valid.id), valid);
});

test('a disposed incident scope rejects late lists and prevents further requests', async () => {
  let release,
    calls = 0;
  const api = createApi(() => {
    calls++;
    return new Promise((resolve) => {
      release = resolve;
    });
  }).integrityIncidents();
  const pending = api.list();
  const rejected = assert.rejects(pending, /consulta|incidente|abierta/i);
  api.dispose();
  release(Response.json(incidentPage([incidentRecord()])));
  await rejected;
  await assert.rejects(() => api.get(incidentId(1)));
  assert.equal(calls, 1);
});

test('an expired session cannot deliver incidents to a subsequent screen', async () => {
  let release;
  const api = createApi((url) =>
    url.endsWith('/logout')
      ? new Response(null, { status: 204 })
      : new Promise((resolve) => {
          release = resolve;
        }),
  );
  const pending = api.integrityIncidents().list();
  const rejected = assert.rejects(pending, /sesi\u00f3n/i);
  await api.logout();
  release(Response.json(incidentPage([incidentRecord()])));
  await rejected;
});

test('incident authorization and unavailable service remain visible errors instead of an empty inbox', async () => {
  for (const [status, code] of [
    [403, 'permission_denied'],
    [503, 'server_busy'],
  ]) {
    let calls = 0;
    const api = createApi(async () => {
      calls++;
      return Response.json({ error: { code } }, { status });
    }).integrityIncidents();
    await assert.rejects(
      () => api.list(),
      (error) => error.status === status && error.code === code,
    );
    assert.equal(calls, 1);
  }
});
