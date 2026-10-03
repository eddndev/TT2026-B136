import { expect } from '@playwright/test';
import { sessionSetup, checkSessionRequests } from './session-inactivity-helpers.mjs';
import { caseId, caseRecord, id, document } from './helpers.mjs';
import { administration, overview } from './case-administration-helpers.mjs';
import { participant, participantId } from './participant-helpers.mjs';
import { assignedMember, caseMemberPage, memberId } from '../fixtures/members.mjs';

export const casePath = `/api/v1/cases/${caseId}/administration`;
export const statusPath = `/api/v1/cases/${caseId}/administrative-status`;
export const participantsPath = `/api/v1/cases/${caseId}/participants`;
export const participantPath = `${participantsPath}/${participantId}`;
export const membersPath = `/api/v1/cases/${caseId}/members`;
export const assignmentPath = `${membersPath}/${memberId(2)}`;
export const versionPath = `/api/v1/cases/${caseId}/documents/${id}/versions/1`;
const casesPath = '/api/v1/case-administrations';
const states = new WeakMap();

export function holdConfirmationRequest(state, method, path) {
  const gate = { method, path, entered: false, released: false, release() {} };
  gate.promise = new Promise((resolve) => {
    gate.release = () => {
      gate.released = true;
      resolve();
    };
  });
  state.confirmationGates.push(gate);
  return gate;
}
export async function checkConfirmationRequests(page) {
  const state = states.get(page);
  state?.confirmationGates.forEach((gate) => gate.release());
  await checkSessionRequests(page);
  if (state)
    expect(state.confirmationWrite, 'Every armed intention must be sent explicitly').toBeNull();
}

export async function confirmationSetup(page) {
  const state = await sessionSetup(page);
  Object.assign(state, {
    confirmationGates: [],
    confirmationWrites: [],
    confirmationWrite: null,
    administration: administration(),
    participant: structuredClone(participant),
    assignment: true,
    version: structuredClone(document),
  });
  states.set(page, state);
  const owns = (path) =>
    [casesPath, casePath, statusPath, versionPath, `${versionPath}/seal`].includes(path) ||
    path === participantsPath ||
    path.startsWith(`${participantsPath}/`) ||
    path === membersPath ||
    path.startsWith(`${membersPath}/`);
  await page.route(
    (url) => owns(url.pathname),
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
        state.unexpected.push(call);
        return fail('unexpected_confirmation_request', 501);
      };
      if (
        !state.current ||
        call.headers.authorization !== `Bearer ${state.current.token}` ||
        state.now >= Math.min(state.current.absolute, state.current.deadline)
      )
        return fail('invalid_session', 401);
      const wait = async () => {
        const gate = state.confirmationGates.find(
          (value) => !value.entered && value.path === call.path && value.method === call.method,
        );
        if (gate) {
          gate.entered = true;
          gate.call = call;
          await gate.promise;
        }
      };
      if (call.method !== 'GET') {
        const command = state.confirmationWrite;
        if (!command || call.path !== command.path || call.method !== command.method || call.search)
          return unexpected();
        expect(call.body === null ? null : request.postDataJSON()).toEqual(command.body);
        state.confirmationWrite = null;
        state.confirmationWrites.push(structuredClone(call));
        command.apply?.();
        const response = structuredClone(command.response);
        await wait();
        return command.status === 204
          ? route.fulfill({ status: 204, headers: { 'Cache-Control': 'no-store' } })
          : reply(response, command.status ?? 200);
      }
      if (call.body !== null) return unexpected();
      let result;
      if (call.path === casesPath) {
        const row = overview(state.administration),
          status = url.searchParams.get('status');
        result = {
          cases: !status || status === 'all' || status === row.administrative_status ? [row] : [],
          has_more: false,
          next_after_id: null,
        };
      } else if (call.path === casePath && !call.search) result = state.administration;
      else if (call.path === participantsPath) {
        const status = url.searchParams.get('status') ?? 'active';
        result = {
          participants: ['all', state.participant.directory_status].includes(status)
            ? [state.participant]
            : [],
          has_more: false,
          next_after_id: null,
        };
      } else if (call.path === participantPath && !call.search) result = state.participant;
      else if (call.path === `${participantPath}/history`)
        result = {
          revisions: [state.participant],
          has_more: false,
          next_before_revision: null,
        };
      else if (call.path === membersPath) {
        const row = {
          ...assignedMember(2),
          assigned_at: state.assignment ? '2026-09-19T10:00:00.123456789Z' : null,
        };
        const query = url.searchParams;
        const visible =
          (query.get('selection') === 'available' ? !state.assignment : state.assignment) &&
          (!query.get('role') || query.get('role') === row.role) &&
          (!query.get('email_prefix') || row.email.startsWith(query.get('email_prefix')));
        result = caseMemberPage(visible ? [row] : []);
      } else if (call.path === versionPath && !call.search) result = state.version;
      else return unexpected();
      const snapshot = structuredClone(result);
      await wait();
      return reply(snapshot);
    },
  );
  return state;
}

export function appliedCaseStatus() {
  return administration(caseRecord, 2, null, 'closed');
}
