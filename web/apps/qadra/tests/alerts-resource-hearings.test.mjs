import test from 'node:test';
import assert from 'node:assert/strict';
import { alertsApi } from '../src/lib/alerts-api.mjs';
import { resourceHearingsApi } from '../src/lib/resource-hearing-api.mjs';
import {
  alertUserId,
  alertId,
  alertOperationId,
  alertOtherId,
  alertRecord,
  alertDetail,
  alertPage,
  alertPreferences,
  alertInstant,
  clone,
} from './fixtures/alerts.mjs';
import { resourceHearingCreation } from './fixtures/resource-hearing-unit.mjs';
import { resourceHearingAlert } from './fixtures/resource-hearing-alert.mjs';

const client = (value) => alertsApi(async () => clone(value), alertUserId);
function ownClient(value = resourceHearingCreation()) {
  const calls = [];
  const { case_id, resource_id } = value.hearing;
  return {
    calls,
    api: resourceHearingsApi(
      async (path, options) => {
        calls.push({ path, options });
        return clone(value);
      },
      case_id,
      resource_id,
    ),
  };
}

test('a resource hearing alert keeps its parent and captured origin beside equal UUID families', async () => {
  const own = resourceHearingAlert();
  const rows = ['hearing', 'deadline'].map((kind, index) => {
    const row = alertRecord('upcoming', kind);
    row.id = `40000000-0000-4000-8000-00000000000${index + 1}`;
    row.subject.id = own.subject.id;
    row.subject.case_id = own.subject.case_id;
    return row;
  });
  rows.push(own);
  rows.sort(
    (a, b) => b.created_at.unix_seconds - a.created_at.unix_seconds || b.id.localeCompare(a.id),
  );
  const expected = alertPage(rows);
  assert.deepEqual(await client(expected).list(), expected);
  assert.deepEqual(await client(alertDetail(own)).get(alertId), alertDetail(own));
});

test('resource hearing alert parsing rejects noninitial origins, deadline reasons and loose scope', async () => {
  for (const change of [
    (row) => {
      row.origin.revision = 0;
    },
    (row) => {
      row.origin.revision = 2;
    },
    (row) => {
      row.origin.evidence_digest = 'capture';
    },
    (row) => {
      delete row.subject.resource_id;
    },
    (row) => {
      row.subject.resource_id = 'foreign';
    },
    (row) => {
      row.subject.association_id = alertOtherId;
    },
    (row) => {
      row.kind = { kind: 'review_required' };
    },
    (row) => {
      row.kind = { kind: 'overdue_unattended', due_at: clone(row.trigger_at) };
    },
    (row) => {
      row.kind = {
        kind: 'due_changed_soon',
        previous_due_at: alertInstant(1767400000),
        current_due_at: clone(row.kind.activity_at),
      };
    },
  ]) {
    const row = resourceHearingAlert();
    change(row);
    await assert.rejects(client(alertDetail(row)).get(alertId));
  }
});

test('reading a resource hearing alert preserves its capture and existing hearing preferences', async () => {
  const row = resourceHearingAlert();
  row.read_at = alertInstant(row.created_at.unix_seconds + 1, 123456789);
  const receipt = { operation_id: alertOperationId, ...alertDetail(row) };
  assert.deepEqual(
    await client(receipt).markRead(alertId, { operation_id: alertOperationId }),
    receipt,
  );
  const preferences = alertPreferences(true);
  preferences.preferences.values.hearing_upcoming.lead_hours = [72, 24];
  preferences.preferences.values.deadline_upcoming.lead_hours = [48];
  assert.deepEqual(await client(preferences).preferences(), preferences);
  assert.equal(preferences.preferences.email_transport, 'disabled');
  assert.equal(Object.hasOwn(preferences.preferences.values, 'resource_hearing_upcoming'), false);
});

test('resource hearing alert opening uses only the exact own route and its original capture', async () => {
  const creation = resourceHearingCreation({ withAct: true });
  const row = resourceHearingAlert(creation);
  const { api, calls } = ownClient(creation);
  assert.deepEqual(await api.fromAlert(row), creation);
  assert.deepEqual(calls, [
    {
      path:
        `/cases/${row.subject.case_id}/procedural-resources/${row.subject.resource_id}` +
        `/activities/resource-hearings/${row.subject.id}/revisions/1`,
      options: undefined,
    },
  ]);
  assert.equal(creation.hearing.values.scheduled_at, '2026-01-01T18:00:00-06:00');
  assert.equal(row.origin.evidence_digest, creation.hearing.capture_digest);
  assert.notEqual(row.origin.evidence_digest, creation.hearing.submission_digest);
});

test('ordinary families and foreign parents cannot dispatch through the own alert reader', async () => {
  for (const change of [
    (row) => {
      row.subject.kind = 'hearing';
      delete row.subject.resource_id;
    },
    (row) => {
      row.subject.kind = 'deadline';
      delete row.subject.resource_id;
    },
    (row) => {
      row.subject.case_id = alertOtherId;
    },
    (row) => {
      row.subject.resource_id = alertOtherId;
    },
    (row) => {
      row.origin.revision = 2;
    },
    (row) => {
      row.kind = { kind: 'review_required' };
    },
  ]) {
    const row = resourceHearingAlert();
    change(row);
    const { api, calls } = ownClient();
    await assert.rejects(api.fromAlert(row));
    assert.equal(calls.length, 0);
  }
});

test('exact own opening rejects a different capture or activity instant without fallback requests', async () => {
  for (const change of [
    (row) => {
      row.origin.evidence_digest = '1'.repeat(64);
    },
    (row) => {
      row.kind.activity_at.unix_seconds++;
      row.trigger_at.unix_seconds++;
    },
    (row) => {
      row.kind.activity_at.nanosecond++;
      row.trigger_at.nanosecond++;
    },
  ]) {
    const row = resourceHearingAlert();
    change(row);
    const { api, calls } = ownClient();
    await assert.rejects(api.fromAlert(row));
    assert.equal(calls.length, 1);
    assert.match(calls[0].path, /\/activities\/resource-hearings\/[^/]+\/revisions\/1$/);
  }
});
