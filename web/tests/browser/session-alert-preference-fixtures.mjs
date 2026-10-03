import { expect } from '@playwright/test';
import { sessionSetup, checkSessionRequests } from './session-inactivity-helpers.mjs';
import { alertPreferences, alertInstant } from '../fixtures/alerts.mjs';

export const preferencesPath = '/api/v1/alert-preferences';
export const alertsPath = '/api/v1/alerts';
export const principalPath = '/api/v1/auth/me';
const states = new WeakMap();
const clone = (value) => structuredClone(value);

export function holdPreferenceRequest(state, method, path) {
  const gate = { method, path, entered: false, release() {} };
  gate.promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.preferenceGates.push(gate);
  return gate;
}
export async function checkAlertPreferenceRequests(page) {
  const state = states.get(page);
  state?.preferenceGates.forEach((gate) => gate.release());
  await checkSessionRequests(page);
  if (state)
    expect(state.nextPreferenceWrite, 'Only explicitly armed saves may be sent').toBeNull();
}

export async function alertPreferenceDraftSetup(page) {
  const state = await sessionSetup(page);
  Object.assign(state, {
    preferenceGates: [],
    preferenceWrites: [],
    nextPreferenceWrite: null,
    preferenceRecords: new Map(),
    preferenceReceipts: new Map(),
    preferenceFailures: new Map(),
    identityOverride: null,
  });
  states.set(page, state);
  state.preferenceRecord = (actor = state.current.user.id) => {
    if (!state.preferenceRecords.has(actor)) {
      const row = alertPreferences().preferences;
      row.user_id = actor;
      state.preferenceRecords.set(actor, row);
    }
    return state.preferenceRecords.get(actor);
  };
  state.commitPreferences = (command, actor = state.current.user.id) => {
    const current = state.preferenceRecord(actor);
    const row = {
      ...clone(current),
      revision: command.expected_revision + 1,
      values: clone(command.values),
      updated_at: alertInstant(Math.floor(state.now / 1000)),
      receipt: { operation_id: command.operation_id, expected_revision: command.expected_revision },
    };
    state.preferenceRecords.set(actor, row);
    state.preferenceReceipts.set(`${actor}:${command.operation_id}`, clone(row));
    return clone(row);
  };
  await page.route(
    /\/api\/v1\/(auth\/me|alerts(?:[/?]|$)|alert-preferences(?:[/?]|$))/,
    async (route) => {
      const request = route.request(),
        url = new URL(request.url());
      const call = {
        path: url.pathname,
        method: request.method(),
        search: url.search,
        body: request.postData(),
        headers: request.headers(),
        at: state.now,
      };
      state.calls.push(call);
      const reply = (json, status = 200) =>
        route.fulfill({ status, json, headers: { 'Cache-Control': 'no-store' } });
      const fail = (code, status) => reply({ error: { code } }, status);
      const unexpected = () => {
        state.writes.push(call);
        return fail('unexpected_alert_preference_request', 501);
      };
      const wait = async () => {
        const gate = state.preferenceGates.find(
          (row) => !row.entered && row.method === call.method && row.path === call.path,
        );
        if (gate) {
          gate.entered = true;
          await gate.promise;
        }
      };
      if (
        !state.current ||
        call.headers.authorization !== `Bearer ${state.current.token}` ||
        state.now >= Math.min(state.current.absolute, state.current.deadline)
      )
        return fail('invalid_session', 401);
      if (state.preferenceFailures.has(call.path))
        return fail('service_unavailable', state.preferenceFailures.get(call.path));
      const principal = state.identityOverride ?? state.current.user;
      if (
        call.path === principalPath &&
        call.method === 'GET' &&
        call.body === null &&
        !call.search
      ) {
        const snapshot = clone(principal);
        await wait();
        return reply(snapshot);
      }
      if (!['owner', 'litigator', 'paralegal'].includes(principal.role))
        return fail('permission_denied', 403);
      if (call.path === alertsPath && call.method === 'GET' && call.body === null) {
        const query = url.searchParams;
        if (
          [...query.keys()].some((key) => !['read', 'state', 'limit'].includes(key)) ||
          !['all', 'unread'].includes(query.get('read')) ||
          !['all', 'active'].includes(query.get('state'))
        )
          return unexpected();
        const snapshot = {
          checked_at: alertInstant(Math.floor(state.now / 1000)),
          alerts: [],
          has_more: false,
          next_cursor: null,
        };
        await wait();
        return reply(snapshot);
      }
      if (call.path !== preferencesPath || call.search) return unexpected();
      if (call.method === 'GET' && call.body === null) {
        const snapshot = { preferences: clone(state.preferenceRecord()) };
        await wait();
        return reply(snapshot);
      }
      if (call.method !== 'PUT' || !state.nextPreferenceWrite) return unexpected();
      const command = request.postDataJSON(),
        mode = state.nextPreferenceWrite;
      state.nextPreferenceWrite = null;
      state.preferenceWrites.push({ ...call, values: clone(command) });
      expect(Object.keys(command).sort()).toEqual(['expected_revision', 'operation_id', 'values']);
      expect(command.operation_id).toMatch(
        /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/,
      );
      const current = state.preferenceRecord();
      const original = state.preferenceReceipts.get(
        `${state.current.user.id}:${command.operation_id}`,
      );
      let result, code;
      if (original) {
        if (
          original.receipt.expected_revision === command.expected_revision &&
          JSON.stringify(original.values) === JSON.stringify(command.values)
        )
          result = clone(original);
        else code = 'alert_operation_conflict';
      } else if (current.revision !== command.expected_revision) code = 'alert_revision_conflict';
      else if (mode.commit !== false) result = state.commitPreferences(command);
      await wait();
      if (code) return fail(code, 409);
      return mode.status
        ? fail('service_unavailable', mode.status)
        : reply({ preferences: result });
    },
  );
  return state;
}
