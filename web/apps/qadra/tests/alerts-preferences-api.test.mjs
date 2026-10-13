import test from 'node:test';
import assert from 'node:assert/strict';
import { alertsApi } from '../src/lib/alerts-api.mjs';
import { createApi } from '../src/lib/api.mjs';
import {
  alertUserId,
  alertOtherId,
  alertPreferences,
  alertPreferenceRequest,
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

test('personal preference defaults come from the server without an implicit save', async () => {
  const expected = alertPreferences(),
    { api, calls } = client(expected);
  assert.deepEqual(await api.preferences(), expected);
  assert.deepEqual(calls, [{ path: '/alert-preferences', options: undefined }]);
  assert.equal(expected.preferences.email_transport, 'disabled');
});

test('preference writes bind the receipt, revision and values to the exact command', async () => {
  const request = alertPreferenceRequest(),
    expected = alertPreferences(true);
  request.values.hearing_upcoming.lead_hours = [96, 12];
  expected.preferences.values = clone(request.values);
  const { api, calls } = client(expected);
  assert.deepEqual(await api.savePreferences(request), expected);
  assert.deepEqual(calls, [
    {
      path: '/alert-preferences',
      options: { method: 'PUT', data: request },
    },
  ]);
});

test('default and persisted preference responses reject false provenance and foreign accounts', async () => {
  for (const change of [
    (v) => {
      v.preferences.user_id = alertOtherId;
    },
    (v) => {
      v.preferences.extra = true;
    },
    (v) => {
      v.preferences.email_transport = 'sent';
    },
    (v) => {
      v.preferences.receipt = { operation_id: alertOtherId, expected_revision: 0 };
    },
    (v) => {
      v.preferences.values.hearing_upcoming.lead_hours = [24];
    },
    (v) => {
      v.preferences.updated_at = { unix_seconds: 0, nanosecond: 0, offset_seconds: 0 };
    },
  ]) {
    const value = alertPreferences();
    change(value);
    await assert.rejects(client(value).api.preferences());
  }
  for (const change of [
    (v) => {
      v.preferences.receipt = null;
    },
    (v) => {
      v.preferences.updated_at = null;
    },
    (v) => {
      v.preferences.revision = 2;
    },
    (v) => {
      v.preferences.values.hearing_upcoming.lead_hours = [24, 48];
    },
  ]) {
    const value = alertPreferences(true);
    change(value);
    await assert.rejects(client(value).api.preferences());
  }
});

test('a structurally valid preference receipt from another operation is not confirmation', async () => {
  for (const change of [
    (v) => {
      v.preferences.receipt.operation_id = alertOtherId;
    },
    (v) => {
      v.preferences.receipt.expected_revision = 1;
      v.preferences.revision = 2;
    },
    (v) => {
      v.preferences.values.review_required.email = false;
    },
  ]) {
    const value = alertPreferences(true);
    change(value);
    await assert.rejects(client(value).api.savePreferences(alertPreferenceRequest()));
  }
});

test('revision conflicts and uncertain failures are returned without automatic writes or reads', async () => {
  for (const failure of [
    Object.assign(new Error('Changed'), { status: 409, code: 'alert_revision_conflict' }),
    new Error('Connection lost'),
  ]) {
    const calls = [];
    const api = alertsApi(async (...args) => {
      calls.push(args);
      throw failure;
    }, alertUserId);
    await assert.rejects(api.savePreferences(alertPreferenceRequest()), (error) => {
      assert.equal(error.code, failure.code);
      assert.equal(error.status, failure.status);
      return true;
    });
    assert.equal(calls.length, 1);
    assert.equal(calls[0][0], '/alert-preferences');
  }
});

test('the authenticated API exposes personal alerts through the same bearer session', async () => {
  const calls = [],
    expected = alertPreferences();
  const api = createApi(async (path, options) => {
    calls.push({ path, options });
    return Response.json(
      path.includes('/mfa/')
        ? { access_token: 'alert-session', user: { id: alertUserId, role: 'owner' } }
        : expected,
    );
  });
  await api.mfa('challenge', '123456', 'totp');
  assert.deepEqual(await api.alerts(alertUserId).preferences(), expected);
  assert.equal(calls.at(-1).path, '/api/v1/alert-preferences');
  assert.equal(
    new Headers(calls.at(-1).options.headers).get('Authorization'),
    'Bearer alert-session',
  );
  assert.equal(calls.at(-1).options.cache, 'no-store');
});
