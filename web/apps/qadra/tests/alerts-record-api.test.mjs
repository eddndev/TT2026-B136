import test from 'node:test';
import assert from 'node:assert/strict';
import { alertsApi } from '../src/lib/alerts-api.mjs';
import {
  alertUserId,
  alertOtherId,
  alertOperationId,
  alertId,
  alertInstant,
  alertRecord,
  alertDetail,
  alertReadReceipt,
  clone,
} from './fixtures/alerts.mjs';

function client(value) {
  const calls = [];
  return {
    calls,
    api: alertsApi(async (path, options) => {
      calls.push({ path, options });
      return clone(value);
    }, alertUserId),
  };
}

test('all four alert reasons preserve exact captured origin and nanoseconds', async () => {
  for (const kind of ['upcoming', 'overdue_unattended', 'review_required', 'due_changed_soon']) {
    const expected = alertDetail(alertRecord(kind));
    assert.deepEqual(await client(expected).api.get(alertId), expected);
  }
  const hearing = alertDetail(alertRecord('upcoming', 'hearing'));
  assert.deepEqual(await client(hearing).api.get(alertId), hearing);
});

test('all email statuses are distinct and provider acceptance is only its captured state', async () => {
  for (const kind of [
    'disabled',
    'pending',
    'sending',
    'failed',
    'unknown',
    'cancelled',
    'accepted',
  ]) {
    const expected = alertDetail();
    expected.alert.email =
      kind === 'accepted' ? { kind, accepted_at: alertInstant(1767230000, 1) } : { kind };
    assert.deepEqual((await client(expected).api.get(alertId)).alert.email, expected.alert.email);
  }
  const invalid = alertDetail();
  invalid.alert.email = { kind: 'delivered' };
  await assert.rejects(client(invalid).api.get(alertId));
});

test('all resolution reasons remain readable history with separate reading state', async () => {
  for (const reason of [
    'superseded',
    'attention_recorded',
    'target_retired',
    'cancelled_hearing',
    'no_longer_eligible',
  ]) {
    const row = alertRecord('upcoming', reason === 'cancelled_hearing' ? 'hearing' : 'deadline');
    row.state = { kind: 'resolved', at: alertInstant(1767230000), reason };
    const expected = alertDetail(row);
    assert.deepEqual(await client(expected).api.get(alertId), expected);
    assert.equal(expected.alert.read_at, null);
  }
});

test('alert detail rejects foreign identity, invalid variants and untyped extra state', async () => {
  for (const change of [
    (v) => {
      v.alert.id = alertOtherId;
    },
    (v) => {
      v.alert.recipient_id = alertOtherId;
    },
    (v) => {
      v.alert.origin.revision = 0;
    },
    (v) => {
      v.alert.origin.evidence_digest = 'unverified';
    },
    (v) => {
      v.alert.subject.kind = 'document';
    },
    (v) => {
      v.alert.subject.case_id = 'other';
    },
    (v) => {
      delete v.alert.subject_title;
    },
    (v) => {
      v.alert.case_title = '';
    },
    (v) => {
      v.alert.case_reference = 'NUC\n123';
    },
    (v) => {
      v.alert.subject_title = ' Not trimmed ';
    },
    (v) => {
      v.alert.case_title = 'x'.repeat(201);
    },
    (v) => {
      v.alert.case_reference = 'x'.repeat(101);
    },
    (v) => {
      v.alert.kind.extra = true;
    },
    (v) => {
      v.alert.kind = { kind: 'review_required', due_at: alertInstant(0) };
    },
    (v) => {
      v.alert.kind.lead_hours = 0;
    },
    (v) => {
      v.alert.state = { kind: 'active', reason: 'superseded' };
    },
    (v) => {
      v.alert.email = { kind: 'accepted' };
    },
    (v) => {
      v.alert.email.address = 'other@example.test';
    },
    (v) => {
      v.alert.operational = { freshness: 'current' };
    },
    (v) => {
      v.checked_at.nanosecond = 1000000000;
    },
    (v) => {
      v.alert.created_at.offset_seconds = -21600;
    },
  ]) {
    const value = alertDetail();
    change(value);
    await assert.rejects(client(value).api.get(alertId));
  }
  for (const kind of ['overdue_unattended', 'review_required', 'due_changed_soon'])
    await assert.rejects(client(alertDetail(alertRecord(kind, 'hearing'))).api.get(alertId));
});

test('explicit idempotent reading sends only its operation and preserves the original read time', async () => {
  const expected = alertReadReceipt(),
    { api, calls } = client(expected);
  const command = { operation_id: alertOperationId };
  assert.deepEqual(await api.markRead(alertId, command), expected);
  assert.deepEqual(await api.markRead(alertId, command), expected);
  assert.deepEqual(
    calls,
    Array.from({ length: 2 }, () => ({
      path: `/alerts/${alertId}/read`,
      options: { method: 'POST', data: command },
    })),
  );
  assert.deepEqual(expected.alert.read_at, alertInstant(1767230000, 9));
});

test('reading does not accept a foreign operation or an unread response as confirmation', async () => {
  for (const change of [
    (v) => {
      v.operation_id = alertOtherId;
    },
    (v) => {
      v.alert.id = alertOtherId;
    },
    (v) => {
      v.alert.read_at = null;
    },
    (v) => {
      v.alert.recipient_id = alertOtherId;
    },
  ]) {
    const expected = alertReadReceipt();
    change(expected);
    await assert.rejects(
      client(expected).api.markRead(alertId, { operation_id: alertOperationId }),
    );
  }
});

test('captured temporal evidence respects exact windows without inventing a current due', async () => {
  for (const change of [
    (v) => {
      v.alert.created_at = alertInstant(v.checked_at.unix_seconds + 1);
    },
    (v) => {
      v.alert.trigger_at = alertInstant(v.alert.created_at.unix_seconds + 1);
    },
    (v) => {
      v.alert.kind.activity_at = clone(v.alert.created_at);
    },
    (v) => {
      v.alert.trigger_at.nanosecond--;
    },
    (v) => {
      v.alert.read_at = alertInstant(v.alert.created_at.unix_seconds - 1);
    },
    (v) => {
      v.alert.read_at = alertInstant(v.checked_at.unix_seconds + 1);
    },
    (v) => {
      v.alert.email = { kind: 'accepted', accepted_at: alertInstant(0) };
    },
    (v) => {
      v.alert.state = { kind: 'resolved', at: alertInstant(0), reason: 'superseded' };
    },
  ]) {
    const value = alertDetail();
    change(value);
    await assert.rejects(client(value).api.get(alertId));
  }
  const overdue = alertDetail(alertRecord('overdue_unattended'));
  overdue.alert.trigger_at.nanosecond++;
  await assert.rejects(client(overdue).api.get(alertId));
  const changed = alertDetail(alertRecord('due_changed_soon'));
  changed.alert.kind.current_due_at = alertInstant(1767000000);
  assert.deepEqual(await client(changed).api.get(alertId), changed);
  changed.alert.kind.current_due_at = clone(changed.alert.kind.previous_due_at);
  await assert.rejects(client(changed).api.get(alertId));
});
