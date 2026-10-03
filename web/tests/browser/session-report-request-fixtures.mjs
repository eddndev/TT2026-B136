import { createHash } from 'node:crypto';
import { expect } from '@playwright/test';
import { sessionSetup, checkSessionRequests } from './session-inactivity-helpers.mjs';
import { lawyerId, pending, reportPage } from './case-reports-fixtures.mjs';

export const reportPath = '/api/v1/case-reports';
export const accountPath = '/api/v1/auth/me';
export { lawyerId };
const states = new WeakMap();
export function holdReportRequest(state, method, path) {
  const gate = { method, path, entered: false, release: () => {} };
  gate.promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.reportGates.push(gate);
  return gate;
}
export async function checkReportRequests(page) {
  const state = states.get(page);
  state?.reportGates.forEach((gate) => gate.release());
  await checkSessionRequests(page);
  if (state)
    expect(state.reportWrite, 'Only an explicit request may consume its write budget').toBeNull();
}
export async function reportDraftSetup(page) {
  const state = await sessionSetup(page);
  Object.assign(state, {
    reportGates: [],
    reportWrites: [],
    reportWrite: null,
    reportRecords: new Map(),
    reportRole: null,
    reportDenied: false,
    reportLawyers: [{ user_id: lawyerId, email: 'lawyer@example.test' }],
  });
  states.set(page, state);
  await page.route(
    (url) => url.pathname === accountPath || url.pathname.startsWith(reportPath),
    async (route) => {
      const request = route.request(),
        url = new URL(request.url());
      const call = {
        method: request.method(),
        path: url.pathname,
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
        state.unexpected.push(call);
        return fail('unexpected_report_draft_request', 501);
      };
      const wait = async () => {
        const gate = state.reportGates.find(
          (v) => !v.entered && v.method === call.method && v.path === call.path,
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
      const role = state.reportRole ?? state.current.user.role;
      if (call.path === accountPath) {
        if (call.method !== 'GET' || call.search || call.body !== null) return unexpected();
        const value = { ...state.current.user, role };
        await wait();
        return reply(value);
      }
      if (state.reportDenied || !['owner', 'litigator'].includes(role))
        return fail('permission_denied', 403);
      const scope = role === 'owner' ? 'office' : 'assigned_cases';
      if (call.method === 'GET' && call.body === null) {
        let value;
        if (
          call.path === `${reportPath}/litigators` &&
          [...url.searchParams.keys()].every((k) => ['limit', 'after_id'].includes(k))
        )
          value = {
            scope,
            checked_at: '2026-10-02T18:00:00Z',
            litigators: structuredClone(state.reportLawyers),
            has_more: false,
            next_after_id: null,
          };
        else if (
          call.path === reportPath &&
          [...url.searchParams.keys()].every((k) =>
            ['limit', 'after_id', 'unread_only'].includes(k),
          ) &&
          ['true', 'false'].includes(url.searchParams.get('unread_only'))
        ) {
          const rows = [...state.reportRecords.values()]
            .filter(
              (v) =>
                v.requester === state.current.user.id &&
                (!url.searchParams.has('after_id') || v.id > url.searchParams.get('after_id')) &&
                (url.searchParams.get('unread_only') === 'false' ||
                  (v.notice && !v.notice.read_at)),
            )
            .sort((a, b) => a.id.localeCompare(b.id))
            .map(({ requester, ...row }) => row);
          const limit = Number(url.searchParams.get('limit'));
          if (!Number.isInteger(limit) || limit < 1 || limit > 100) return unexpected();
          value = reportPage(rows.slice(0, limit), {
            has_more: rows.length > limit,
            next_after_id: rows.length > limit ? rows[limit - 1].id : null,
          });
        } else if (call.path.startsWith(reportPath + '/') && !call.search) {
          const record = [...state.reportRecords.values()].find(
            (v) => v.id === call.path.slice(reportPath.length + 1),
          );
          if (!record || record.requester !== state.current.user.id)
            return fail('case_report_not_found', 404);
          const { requester, ...row } = record;
          value = row;
        } else return unexpected();
        await wait();
        return reply(value);
      }
      if (call.path !== reportPath || call.method !== 'POST' || call.search || !state.reportWrite)
        return unexpected();
      const command = request.postDataJSON(),
        mode = state.reportWrite;
      state.reportWrite = null;
      state.reportWrites.push(structuredClone(command));
      const key = state.current.user.id + ':' + command.operation_id;
      let record = state.reportRecords.get(key);
      if (record && JSON.stringify(record.filters) !== JSON.stringify(command.filters))
        return fail('case_report_operation_conflict', 409);
      if (!record) {
        record = {
          ...pending({
            id: `81000000-0000-4000-8000-${String(state.reportRecords.size + 1).padStart(12, '0')}`,
            operation_id: command.operation_id,
            filters: structuredClone(command.filters),
            scope,
            request_digest: createHash('sha256').update(JSON.stringify(command)).digest('hex'),
          }),
          requester: state.current.user.id,
        };
        if (mode.commit !== false) state.reportRecords.set(key, record);
      }
      const { requester, ...result } = record;
      await wait();
      return mode.status ? fail('service_busy', mode.status) : reply(result, 202);
    },
  );
  return state;
}
