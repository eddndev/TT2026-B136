import test from 'node:test';
import assert from 'node:assert/strict';
import { combinedAgendaApi } from '../src/lib/combined-agenda-api.mjs';
import { agendaSelection, mergeAgendaItems } from '../src/components/agenda-presentation.mjs';
import { hearingRow } from './fixtures/hearings.mjs';

const query = { from: '2026-01-01T00:00:00Z', until: '2026-01-02T00:00:00Z' };
const id = '00000000-0000-0000-0000-000000000002';
const clone = (value) => structuredClone(value);
const instant = (seconds) => ({ unix_seconds: seconds, nanosecond: 0, offset_seconds: 0 });
function item(status = 'scheduled') {
  return {
    kind: 'precautionary_hearing',
    at: instant(1767225601),
    case_title: 'Authorized precautionary case',
    case_reference: 'PRECAUTIONARY-1',
    case_status: 'closed',
    precautionary_hearing: {
      case_id: '00000000-0000-0000-0000-000000000001',
      id,
      revision: 3,
      purpose: 'review',
      scheduled_at: '2025-12-31T18:00:01-06:00',
      modality: 'in_person',
      status,
      participant_count: 2,
      capture_digest: '08'.repeat(32),
    },
  };
}
function page(kind = 'precautionary_hearing', hearing_status = 'scheduled') {
  return {
    ...query,
    kind,
    hearing_status,
    checked_at: instant(1767225602),
    items: [item(hearing_status === 'cancelled' ? 'cancelled' : 'scheduled')],
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

test('precautionary Agenda retains exact current captures and declared cancellation', async () => {
  for (const kind of ['all', 'precautionary_hearing']) {
    for (const hearing_status of ['scheduled', 'cancelled', 'all']) {
      for (const purpose of ['imposition', 'review']) {
        const value = page(kind, hearing_status);
        value.items[0].precautionary_hearing.purpose = purpose;
        const { api, calls } = client(value);
        assert.deepEqual(await api.list({ ...query, kind, hearing_status }), value);
        const url = new URL(calls[0], 'https://local.test');
        assert.equal(url.pathname, '/agenda');
        assert.equal(url.searchParams.get('kind'), kind);
        assert.equal(url.searchParams.get('hearing_status'), hearing_status);
        assert.equal(calls.length, 1);
      }
    }
  }
  const selected = agendaSelection({
    view: 'custom',
    from: '2026-01-01',
    until: '2026-01-02',
    offset: '+00:00',
    kind: 'precautionary_hearing',
    hearing_status: 'cancelled',
  });
  assert.equal(selected.query.hearing_status, 'cancelled');
  assert.equal(selected.filters.kind, 'precautionary_hearing');
});

test('precautionary cursor uses rank three and remains bound to range family and status', async () => {
  const value = page('precautionary_hearing', 'all');
  value.complete = false;
  value.next_cursor = `a1:1767225600:1767312000:precautionary_hearing:all:1767225601:0:3:${id}`;
  const input = { ...query, kind: value.kind, hearing_status: value.hearing_status, limit: 1 };
  assert.deepEqual(await client(value).api.list(input), value);
  const empty = { ...clone(value), items: [], complete: true, next_cursor: null };
  const { api, calls } = client(empty);
  assert.deepEqual(await api.list({ ...input, limit: 2, cursor: value.next_cursor }), empty);
  assert.equal(
    new URL(calls[0], 'https://local.test').searchParams.get('cursor'),
    value.next_cursor,
  );
  for (const mutation of [
    { kind: 'all' },
    { hearing_status: 'cancelled' },
    { until: '2026-01-03T00:00:00Z' },
    { cursor: value.next_cursor.replace(':0:3:', ':0:03:') },
    { cursor: value.next_cursor.replace(':0:3:', ':0:4:') },
    { cursor: value.next_cursor.replace(':0:3:', ':0:2:') },
  ]) {
    const { api, calls } = client(empty);
    await assert.rejects(api.list({ ...input, cursor: value.next_cursor, ...mutation }));
    assert.equal(calls.length, 0);
  }
});

test('precautionary overview rejects malformed shape time revision and status mismatches', async () => {
  for (const mutate of [
    (v) => {
      v.items[0].precautionary_hearing.case_id = 'foreign';
    },
    (v) => {
      v.items[0].precautionary_hearing.id = 'bad';
    },
    (v) => {
      v.items[0].precautionary_hearing.revision = 0;
    },
    (v) => {
      v.items[0].precautionary_hearing.revision = 1.5;
    },
    (v) => {
      v.items[0].precautionary_hearing.purpose = 'initial';
    },
    (v) => {
      v.items[0].precautionary_hearing.status = 'expired';
    },
    (v) => {
      v.items[0].precautionary_hearing.status = 'cancelled';
    },
    (v) => {
      v.items[0].precautionary_hearing.modality = 'automatic';
    },
    (v) => {
      v.items[0].precautionary_hearing.participant_count = 33;
    },
    (v) => {
      v.items[0].precautionary_hearing.capture_digest = 'bad';
    },
    (v) => {
      v.items[0].precautionary_hearing.capture_digest = 'AB'.repeat(32);
    },
    (v) => {
      v.items[0].precautionary_hearing.scheduled_at = '2025-12-31T18:00:02-06:00';
    },
    (v) => {
      v.items[0].precautionary_hearing.scheduled_at = '2025-12-31T18:00:01';
    },
    (v) => {
      v.items[0].precautionary_hearing.measure_id = id;
    },
    (v) => {
      v.items[0].at.nanosecond = 1;
    },
    (v) => {
      v.items[0].case_status = 'unknown';
    },
    (v) => {
      v.items.push(clone(v.items[0]));
    },
  ]) {
    const value = page();
    mutate(value);
    await assert.rejects(client(value).api.list({ ...query, kind: 'precautionary_hearing' }));
  }
  const cancelled = page('precautionary_hearing', 'cancelled');
  cancelled.items[0].precautionary_hearing.status = 'scheduled';
  await assert.rejects(
    client(cancelled).api.list({
      ...query,
      kind: 'precautionary_hearing',
      hearing_status: 'cancelled',
    }),
  );
  for (const kind of ['hearing', 'deadline', 'resource_hearing']) {
    const wrongFamily = page(kind);
    await assert.rejects(client(wrongFamily).api.list({ ...query, kind }));
  }
});

test('Agenda merges a newer cancelled precautionary head without colliding with an ordinary hearing', () => {
  const earlier = item(),
    later = item('cancelled');
  earlier.precautionary_hearing.revision = 2;
  earlier.precautionary_hearing.capture_digest = '07'.repeat(32);
  const ordinary = hearingRow();
  ordinary.id = id;
  ordinary.scheduled_at = earlier.precautionary_hearing.scheduled_at;
  const other = { kind: 'hearing', at: clone(earlier.at), hearing: ordinary };
  assert.deepEqual(mergeAgendaItems([earlier, other], [later]), [other, later]);
  assert.deepEqual(mergeAgendaItems([later, other], [earlier]), [other, later]);
});
