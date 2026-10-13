import { expect } from '@playwright/test';
import { sessionSetup, checkSessionRequests } from './session-inactivity-helpers.mjs';
import { holdConfirmationRequest } from './session-confirmation-fixtures.mjs';
import { alertRecord, alertInstant, alertId, alertOtherId } from '../fixtures/alerts.mjs';
import { report, reportPage, reportId } from './case-reports-fixtures.mjs';

export { alertId, alertOtherId, reportId };
export { holdConfirmationRequest as holdNoticeRequest };
export {
  expectFreshRead,
  releaseOldConfirmation as releaseNoticeResponse,
} from './session-confirmation-ui.mjs';
export const alertsPath = '/api/v1/alerts';
export const reportsPath = '/api/v1/case-reports';
export const reportDetailPath = `${reportsPath}/${reportId}`;
const states = new WeakMap();
const instant = (state) => alertInstant(Math.floor(state.now / 1000));

export function seedAlert(state, id = alertId) {
  const row = {
    ...alertRecord(),
    id,
    occurrence_id:
      id === alertId
        ? '40000000-0000-4000-8000-000000000001'
        : '40000000-0000-4000-8000-000000000002',
    recipient_id: state.current.user.id,
  };
  state.noticeAlerts.set(id, row);
  return row;
}

export function seedReport(state) {
  const row = report();
  state.noticeReports.set(row.id, { requester: state.current.user.id, row });
  return row;
}

export async function checkNoticeRequests(page) {
  const state = states.get(page);
  state?.confirmationGates.forEach((gate) => gate.release());
  await checkSessionRequests(page);
  if (state)
    expect(state.nextRead, 'An armed read must require and receive an explicit click').toBeNull();
}

export async function readNoticeSetup(page) {
  const state = await sessionSetup(page);
  Object.assign(state, {
    confirmationGates: [],
    noticeAlerts: new Map(),
    noticeReports: new Map(),
    noticeDenials: new Map(),
    noticeWrites: [],
    nextRead: null,
  });
  states.set(page, state);
  await page.route(
    (url) =>
      [alertsPath, reportsPath].some(
        (prefix) => url.pathname === prefix || url.pathname.startsWith(prefix + '/'),
      ),
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
        route.fulfill({
          status,
          json,
          headers: { 'Cache-Control': 'no-store' },
        });
      const fail = (code, status) => reply({ error: { code } }, status);
      const unexpected = () => {
        state.unexpected.push(call);
        return fail('unexpected_read_notice_request', 501);
      };
      const wait = async () => {
        const gate = state.confirmationGates.find(
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
      const actor = state.current.user.id;
      const denial = state.noticeDenials.get(call.path);
      if (denial) {
        await wait();
        return fail(denial.code, denial.status);
      }
      const alertMatch = /^\/api\/v1\/alerts\/([^/]+)(\/read)?$/.exec(call.path);
      const reportMatch = /^\/api\/v1\/case-reports\/([^/]+)(\/notice-read)?$/.exec(call.path);
      const alert = alertMatch && state.noticeAlerts.get(alertMatch[1]);
      const entry = reportMatch && state.noticeReports.get(reportMatch[1]);
      if (call.method === 'GET' && call.body === null) {
        let result;
        if (call.path === alertsPath) {
          if (
            url.searchParams.get('limit') !== '20' ||
            !['all', 'unread'].includes(url.searchParams.get('read')) ||
            !['all', 'active'].includes(url.searchParams.get('state')) ||
            [...url.searchParams.keys()].some((key) => !['limit', 'read', 'state'].includes(key))
          )
            return unexpected();
          const rows = [...state.noticeAlerts.values()].filter(
            (row) =>
              row.recipient_id === actor &&
              (url.searchParams.get('read') !== 'unread' || row.read_at === null) &&
              (url.searchParams.get('state') !== 'active' || row.state.kind === 'active'),
          );
          rows.sort((a, b) => b.id.localeCompare(a.id));
          result = { checked_at: instant(state), alerts: rows, has_more: false, next_cursor: null };
        } else if (call.path === `${reportsPath}/litigators`) {
          if (url.search !== '?limit=20') return unexpected();
          result = {
            scope: 'office',
            checked_at: new Date(state.now).toISOString(),
            litigators: [],
            has_more: false,
            next_after_id: null,
          };
        } else if (call.path === reportsPath) {
          if (
            url.searchParams.get('limit') !== '20' ||
            url.searchParams.get('unread_only') !== 'false' ||
            [...url.searchParams.keys()].some((key) => !['limit', 'unread_only'].includes(key))
          )
            return unexpected();
          const rows = [...state.noticeReports.values()]
            .filter((value) => value.requester === actor)
            .map((value) => value.row)
            .sort((a, b) => a.id.localeCompare(b.id));
          result = reportPage(rows);
        } else if (alertMatch && !alertMatch[2] && !call.search) {
          if (!alert || alert.recipient_id !== actor) return fail('alert_not_found', 404);
          result = { checked_at: instant(state), alert };
        } else if (reportMatch && !reportMatch[2] && !call.search) {
          if (!entry || entry.requester !== actor) return fail('case_report_not_found', 404);
          result = entry.row;
        } else return unexpected();
        const snapshot = structuredClone(result);
        await wait();
        return reply(snapshot);
      }
      const mode = state.nextRead;
      if (call.method !== 'POST' || call.search || !mode || mode.path !== call.path)
        return unexpected();
      const command = request.postDataJSON();
      let result;
      if (alertMatch?.[2] === '/read') {
        if (!alert || alert.recipient_id !== actor) return fail('alert_not_found', 404);
        expect(Object.keys(command)).toEqual(['operation_id']);
        expect(command.operation_id).toMatch(
          /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
        );
        if (mode.commit !== false) alert.read_at ??= instant(state);
        result = { operation_id: command.operation_id, checked_at: instant(state), alert };
      } else if (reportMatch?.[2] === '/notice-read') {
        if (!entry || entry.requester !== actor) return fail('case_report_not_found', 404);
        expect(command).toEqual({});
        if (mode.commit !== false) {
          entry.row.notice.read_at ??= new Date(state.now).toISOString();
          entry.row.updated_at = entry.row.notice.read_at;
        }
        result = entry.row;
      } else return unexpected();
      state.nextRead = null;
      state.noticeWrites.push({ ...structuredClone(call), command, actor });
      const snapshot = structuredClone(result);
      await wait();
      return mode.status ? fail('service_unavailable', mode.status) : reply(snapshot);
    },
  );
  return state;
}
