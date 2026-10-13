import test from 'node:test';
import assert from 'node:assert/strict';
import { alertsApi } from '../src/lib/alerts-api.mjs';
import {
  alertUserId,
  alertId,
  alertOperationId,
  alertOtherId,
  alertRecord,
  alertDetail,
  alertPage,
  alertInstant,
  clone,
} from './fixtures/alerts.mjs';

const client = (value) => alertsApi(async () => clone(value), alertUserId);
function caution() {
  const row = alertRecord('upcoming', 'precautionary_hearing');
  row.subject_title = 'Audiencia cautelar declarada';
  row.origin.revision = 2;
  return row;
}

test('precautionary alerts retain positive revisions and distinct subjects beside equal UUID families', async () => {
  const own = caution();
  const rows = ['hearing', 'deadline', 'resource_hearing', 'precautionary_hearing'].map(
    (kind, index) => {
      const row = clone(own);
      row.id = `40000000-0000-4000-8000-00000000000${index + 1}`;
      row.subject.kind = kind;
      if (kind === 'resource_hearing') {
        row.subject.resource_id = alertOtherId;
        row.origin.revision = 1;
      }
      return row;
    },
  );
  rows.sort((a, b) => b.id.localeCompare(a.id));
  const expected = alertPage(rows);
  assert.deepEqual(await client(expected).list(), expected);
  for (const revision of [1, 2, 3, 4294967295]) {
    own.origin.revision = revision;
    assert.deepEqual(await client(alertDetail(own)).get(alertId), alertDetail(own));
  }
  assert.equal(Object.hasOwn(own.subject, 'resource_id'), false);
});

test('precautionary alert validation rejects foreign shape and deadline notification kinds', async () => {
  for (const change of [
    (row) => {
      row.origin.revision = 0;
    },
    (row) => {
      row.origin.evidence_digest = 'capture';
    },
    (row) => {
      row.subject.resource_id = alertOtherId;
    },
    (row) => {
      row.subject.case_id = 'foreign';
    },
    (row) => {
      delete row.subject.id;
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
    const row = caution();
    change(row);
    await assert.rejects(client(alertDetail(row)).get(alertId));
  }
});

test('repeated precautionary read validation preserves historical revision and cancellation state', async () => {
  const row = caution();
  row.state = {
    kind: 'resolved',
    at: alertInstant(row.created_at.unix_seconds + 1, row.created_at.nanosecond),
    reason: 'cancelled_hearing',
  };
  row.read_at = alertInstant(row.created_at.unix_seconds + 2, row.created_at.nanosecond);
  const receipt = { operation_id: alertOperationId, ...alertDetail(row) };
  for (let attempt = 0; attempt < 2; attempt++) {
    assert.deepEqual(
      await client(receipt).markRead(alertId, { operation_id: alertOperationId }),
      receipt,
    );
  }
  assert.equal(receipt.alert.origin.revision, 2);
  assert.deepEqual(receipt.alert.origin, caution().origin);
  const wrong = clone(receipt);
  wrong.operation_id = alertOtherId;
  await assert.rejects(client(wrong).markRead(alertId, { operation_id: alertOperationId }));
});
