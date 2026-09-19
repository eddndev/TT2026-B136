import test from 'node:test';
import assert from 'node:assert/strict';
import { alertPreferenceCommand, alertReadCommand } from '../src/lib/alerts-preference-values.mjs';
import { alertOperationId, alertPreferenceRequest } from './fixtures/alerts.mjs';

test('personal preferences preserve explicit hours and channels with revision zero', () => {
  const request = alertPreferenceRequest();
  assert.deepEqual(alertPreferenceCommand(request), request);
  request.expected_revision = 7;
  request.values.hearing_upcoming.lead_hours = [];
  request.values.deadline_upcoming.lead_hours = [720, 1];
  request.values.review_required = { internal: false, email: false };
  assert.deepEqual(alertPreferenceCommand(request), request);
});

test('preferences require all five policies and reject ownership or address overrides', () => {
  for (const change of [
    (v) => {
      delete v.values.hearing_upcoming;
    },
    (v) => {
      delete v.values.deadline_upcoming;
    },
    (v) => {
      delete v.values.overdue_unattended;
    },
    (v) => {
      delete v.values.review_required;
    },
    (v) => {
      delete v.values.due_changed_soon;
    },
    (v) => {
      v.user_id = alertOperationId;
    },
    (v) => {
      v.values.email = 'other@example.test';
    },
    (v) => {
      v.values.hearing_upcoming.subscriptions = [];
    },
    (v) => {
      v.values.due_changed_soon.sms = true;
    },
  ]) {
    const request = alertPreferenceRequest();
    change(request);
    assert.throws(() => alertPreferenceCommand(request));
  }
});

test('advance hours are bounded distinct integer lists without implicit values', () => {
  for (const hours of [
    null,
    '48,24',
    [0],
    [-1],
    [721],
    [24.5],
    ['24'],
    [48, 48],
    [9, 8, 7, 6, 5, 4, 3, 2, 1],
  ]) {
    const request = alertPreferenceRequest();
    request.values.hearing_upcoming.lead_hours = hours;
    assert.throws(() => alertPreferenceCommand(request));
  }
  const missing = alertPreferenceRequest();
  delete missing.values.deadline_upcoming.lead_hours;
  assert.throws(() => alertPreferenceCommand(missing));
});

test('channels and optimistic revisions are strictly typed', () => {
  for (const change of [
    (v) => {
      v.values.hearing_upcoming.channels = null;
    },
    (v) => {
      delete v.values.overdue_unattended.email;
    },
    (v) => {
      v.values.review_required.internal = 1;
    },
    (v) => {
      v.values.due_changed_soon.email = 'true';
    },
    (v) => {
      v.expected_revision = -1;
    },
    (v) => {
      v.expected_revision = 0.5;
    },
    (v) => {
      v.expected_revision = '0';
    },
    (v) => {
      v.expected_revision = 4294967296;
    },
    (v) => {
      v.operation_id = 'unbound';
    },
  ]) {
    const request = alertPreferenceRequest();
    change(request);
    assert.throws(() => alertPreferenceCommand(request));
  }
});

test('reading sends one exact operation identity without changing resource attention', () => {
  assert.deepEqual(alertReadCommand({ operation_id: alertOperationId }), {
    operation_id: alertOperationId,
  });
  for (const request of [
    {},
    { operation_id: null },
    { operation_id: 'invalid' },
    { operation_id: alertOperationId, attention: 'recorded' },
    { operation_id: alertOperationId, user_id: alertOperationId },
    { operation_id: alertOperationId, read: false },
  ])
    assert.throws(() => alertReadCommand(request));
});
