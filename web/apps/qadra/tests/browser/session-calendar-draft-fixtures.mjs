import { expect } from '@playwright/test';
import { sessionSetup, checkSessionRequests } from './session-inactivity-helpers.mjs';
import {
  calendarPrepared,
  calendarRecord,
  calendarOverview,
  calendarHistoryRow,
  calendarDayRows,
} from '../fixtures/judicial-calendars.mjs';

export const calendarsPath = '/api/v1/judicial-calendars';
export const ownerPath = '/api/v1/auth/me';
const states = new WeakMap();

export function holdCalendarRequest(state, method, path) {
  const gate = { method, path, entered: false, release: () => {} };
  gate.promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.calendarGates.push(gate);
  return gate;
}
export async function checkCalendarDraftRequests(page) {
  const state = states.get(page);
  state?.calendarGates.forEach((gate) => gate.release());
  await checkSessionRequests(page);
  if (!state) return;
  expect(state.calendarBudget, 'Preparation requires an explicit review click').toBe(0);
  expect(state.calendarWrite, 'An armed calendar mutation must occur once').toBeNull();
  expect(
    state.calls.filter((call) => call.path.startsWith('/api/v1/cases/')),
    'A global calendar draft must not authorize through a fictitious case',
  ).toEqual([]);
}
export async function calendarDraftSetup(page, { records = [] } = {}) {
  const state = await sessionSetup(page);
  Object.assign(state, {
    calendarRecords: new Map(),
    calendarGates: [],
    calendarPosts: [],
    calendarPrepares: [],
    calendarBudget: 0,
    calendarWrite: null,
    currentRole: null,
    calendarDenied: false,
  });
  for (const row of records)
    state.calendarRecords.set(row.id, [
      ...(state.calendarRecords.get(row.id) || []),
      structuredClone(row),
    ]);
  states.set(page, state);
  state.prepareCalendar = (command) => {
    const base = state.calendarRecords.get(command.calendar_id)?.at(-1);
    const value = calendarPrepared(command, base);
    value.actor_id = state.current.user.id;
    value.initial_scope = structuredClone(base?.values.scope || value.values.scope);
    return value;
  };
  state.commitCalendar = (prepared) => {
    const row = calendarRecord(prepared);
    state.calendarRecords.set(row.id, [...(state.calendarRecords.get(row.id) || []), row]);
    return row;
  };
  await page.route(
    (url) => url.pathname === ownerPath || url.pathname.startsWith(calendarsPath),
    async (route) => {
      const request = route.request(),
        url = new URL(request.url());
      const call = {
        path: url.pathname,
        method: request.method(),
        body: request.postData(),
        search: url.search,
        headers: request.headers(),
        at: state.now,
      };
      state.calls.push(call);
      const reply = (json, status = 200) =>
        route.fulfill({ status, json, headers: { 'Cache-Control': 'no-store' } });
      const fail = (code, status) => reply({ error: { code } }, status);
      const unexpected = () => {
        state.unexpected.push(call);
        return fail('unexpected_calendar_draft_request', 501);
      };
      const wait = async () => {
        const gate = state.calendarGates.find(
          (item) => !item.entered && item.method === call.method && item.path === call.path,
        );
        if (gate) {
          gate.entered = true;
          gate.call = call;
          await gate.promise;
        }
      };
      if (
        !state.current ||
        call.headers.authorization !== `Bearer ${state.current.token}` ||
        state.now >= Math.min(state.current.absolute, state.current.deadline)
      )
        return fail('invalid_session', 401);
      const role = state.currentRole ?? state.current.user.role;
      if (call.path === ownerPath) {
        if (call.method !== 'GET' || call.body !== null || call.search) return unexpected();
        const value = { ...state.current.user, role };
        await wait();
        return reply(value);
      }
      if (
        state.calendarDenied ||
        !['owner', 'litigator', 'paralegal'].includes(role) ||
        (call.method !== 'GET' && role !== 'owner')
      )
        return fail('permission_denied', 403);
      const parts = call.path.slice(calendarsPath.length).split('/').filter(Boolean);
      if (call.method === 'GET') {
        let value;
        if (!parts.length) {
          const status = url.searchParams.get('status') || 'published';
          const limit = Number(url.searchParams.get('limit') || 20),
            after = url.searchParams.get('after_id');
          const all = [...state.calendarRecords.values()]
            .map((rows) => rows.at(-1))
            .filter(
              (row) => (status === 'all' || row.status === status) && (!after || row.id > after),
            )
            .sort((a, b) => a.id.localeCompare(b.id));
          const rows = all.slice(0, limit),
            more = all.length > limit;
          value = {
            calendars: rows.map(calendarOverview),
            has_more: more,
            next_after_id: more ? rows.at(-1).id : null,
          };
        } else {
          const history = state.calendarRecords.get(parts[0]);
          if (!history) return fail('judicial_calendar_not_found', 404);
          if (parts[1] === 'history' && parts.length === 2) {
            const before = Number(url.searchParams.get('before_revision') || Infinity),
              limit = Number(url.searchParams.get('limit') || 10);
            const all = [...history].reverse().filter((row) => row.revision < before),
              rows = all.slice(0, limit),
              more = all.length > limit;
            value = {
              revisions: rows.map(calendarHistoryRow),
              has_more: more,
              next_before_revision: more ? rows.at(-1).revision : null,
            };
          } else {
            const row =
              parts[1] === 'revisions'
                ? history.find((item) => item.revision === Number(parts[2]))
                : history.at(-1);
            if (!row) return fail('judicial_calendar_not_found', 404);
            if (parts.length === 1 || (parts[1] === 'revisions' && parts.length === 3)) value = row;
            else if (parts.length === 4 && parts[1] === 'revisions' && parts[3] === 'days')
              value = {
                calendar_id: row.id,
                revision: row.revision,
                values_digest: row.values_digest,
                days: calendarDayRows(
                  row.values,
                  url.searchParams.get('from'),
                  url.searchParams.get('through'),
                ),
              };
            else return unexpected();
          }
        }
        const snapshot = structuredClone(value);
        await wait();
        return reply(snapshot);
      }
      const body = request.postDataJSON(),
        preparing = parts[0] === 'prepare';
      const command = preparing ? body : body?.command,
        change = command?.change;
      if (!change || call.search || (preparing ? !state.calendarBudget : !state.calendarWrite))
        return unexpected();
      const valid = preparing
        ? call.method === 'POST' && parts.length === 1
        : change.action === 'publish'
          ? call.method === 'POST' && parts.length === 0
          : command.calendar_id === parts[0] &&
            (change.action === 'replace'
              ? call.method === 'PUT' && parts.length === 1
              : change.action === 'retire' &&
                call.method === 'POST' &&
                parts.length === 2 &&
                parts[1] === 'retirement');
      if (!valid) return unexpected();
      const base = state.calendarRecords.get(command.calendar_id)?.at(-1);
      if ((base?.revision ?? 0) !== change.expected_revision)
        return fail('judicial_calendar_revision_conflict', 409);
      const prepared = state.prepareCalendar(command);
      if (preparing) {
        state.calendarBudget--;
        state.calendarPrepares.push(structuredClone(prepared));
        return reply(prepared);
      }
      if (body.expected_submission_digest !== prepared.submission_digest) return unexpected();
      const behavior = state.calendarWrite;
      state.calendarWrite = null;
      state.calendarPosts.push(structuredClone(prepared));
      const value = behavior.commit === false ? null : state.commitCalendar(prepared);
      await wait();
      if (behavior.status) return fail('service_busy', behavior.status);
      return reply(value, change.action === 'publish' ? 201 : 200);
    },
  );
  return state;
}
