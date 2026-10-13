import { expect } from '@playwright/test';
import { caseId, caseRecord, document } from './helpers.mjs';
import { administration, profile } from './case-administration-helpers.mjs';
import { initial, entry } from './stage-helpers.mjs';
import {
  subjectDraftSetup,
  checkSubjectRequests,
  holdSubjectRequest,
  casePath,
} from './session-subject-draft-fixtures.mjs';

export { casePath, initial, document, holdSubjectRequest };
export const stagePath = `/api/v1/cases/${caseId}/stage`;
export const historyPath = `${stagePath}/history`;
export const transitionPath = `${stagePath}/transitions`;
export const adoptionPath = `${stagePath}/adoption`;
export const stageTime = { precision: 'date', date: '2026-09-01', offset: '-06:00' };
export const support = { document_id: document.id, version: 1, digest: document.digest };
export const intermediateCommand = {
  expected_revision: 1,
  target: 'intermediate',
  accusation_declared_at: stageTime,
  accusation: support,
};
export const intermediate = () => entry(intermediateCommand, 2);
const states = new WeakMap();

export function holdStageRequest(state, method, path, beforeRevision) {
  const gate = { method, path, beforeRevision, entered: false, call: null, release: () => {} };
  gate.promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.stageGates.push(gate);
  return gate;
}

export async function checkStageDraftRequests(page) {
  const state = states.get(page);
  state?.stageGates.forEach((gate) => gate.release());
  await checkSubjectRequests(page);
  if (state) expect(state.nextStageWrite, 'Every armed stage command must be sent once').toBeNull();
}

export async function stageDraftSetup(page, current = initial) {
  const state = await subjectDraftSetup(page);
  Object.assign(state, {
    stageHead: structuredClone(current),
    stageHistory: current
      ? current.stage_revision === 1
        ? [structuredClone(current)]
        : [structuredClone(current), structuredClone(initial)]
      : [],
    stageGates: [],
    stagePosts: [],
    nextStageWrite: null,
    historyPageSize: 20,
    completeProfile: true,
  });
  states.set(page, state);
  const reply = (route, json, status = 200) =>
    route.fulfill({
      status,
      json,
      headers: { 'Cache-Control': 'no-store' },
    });
  const unexpected = (route, call) => {
    state.writes.push(call);
    return reply(route, { error: { code: 'unexpected_stage_draft_request' } }, 501);
  };
  async function wait(call) {
    const cursor = new URLSearchParams(call.search).get('before_revision');
    const gate = state.stageGates.find(
      (row) =>
        !row.entered &&
        row.method === call.method &&
        row.path === call.path &&
        (row.beforeRevision === undefined || row.beforeRevision === cursor),
    );
    if (!gate) return;
    gate.entered = true;
    gate.call = call;
    await gate.promise;
  }
  await page.route(
    (url) => url.pathname === casePath || url.pathname.startsWith(stagePath),
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
      if (
        !state.current ||
        call.headers.authorization !== `Bearer ${state.current.token}` ||
        state.now >= Math.min(state.current.absolute, state.current.deadline)
      )
        return reply(route, { error: { code: 'invalid_session' } }, 401);
      if (!state.allowed) return reply(route, { error: { code: 'case_not_found' } }, 404);
      const roles =
        call.method === 'GET' ? ['owner', 'litigator', 'paralegal'] : ['owner', 'litigator'];
      if (!roles.includes(state.current.user.role))
        return reply(route, { error: { code: 'permission_denied' } }, 403);
      if (call.method === 'GET' && call.body === null) {
        if (call.path === casePath && !call.search) {
          const result = administration(
            caseRecord,
            state.caseRevision,
            state.completeProfile ? profile : null,
            state.caseStatus,
          );
          await wait(call);
          return reply(route, result);
        }
        if (call.path === stagePath && !call.search) {
          const current = structuredClone(state.stageHead);
          await wait(call);
          return reply(route, { case_id: caseId, current });
        }
        if (call.path === historyPath) {
          if (
            [...url.searchParams.keys()].some((key) => !['limit', 'before_revision'].includes(key))
          )
            return unexpected(route, call);
          const cursor = Number(url.searchParams.get('before_revision') || Infinity);
          const limit = Math.min(
            Number(url.searchParams.get('limit') || 20),
            state.historyPageSize,
          );
          const rows = state.stageHistory.filter((row) => row.stage_revision < cursor);
          const entries = structuredClone(rows.slice(0, limit)),
            more = rows.length > limit;
          await wait(call);
          return reply(route, {
            entries,
            has_more: more,
            next_before_revision: more ? entries.at(-1).stage_revision : null,
          });
        }
        return unexpected(route, call);
      }
      if (
        call.method !== 'POST' ||
        call.search ||
        ![adoptionPath, transitionPath].includes(call.path) ||
        !state.nextStageWrite
      )
        return unexpected(route, call);
      const mode = state.nextStageWrite;
      state.nextStageWrite = null;
      const values = request.postDataJSON();
      state.stagePosts.push({ ...call, values });
      if (state.caseStatus === 'closed')
        return reply(route, { error: { code: 'case_closed' } }, 409);
      if (!state.completeProfile)
        return reply(route, { error: { code: 'case_stage_profile_incomplete' } }, 409);
      if (values.expected_revision !== (state.stageHead?.stage_revision || 0))
        return reply(route, { error: { code: 'case_stage_conflict' } }, 409);
      const target = !state.stageHead
        ? 'adoption'
        : state.stageHead.stage === 'investigation'
          ? 'intermediate'
          : state.stageHead.stage === 'intermediate'
            ? 'trial'
            : null;
      if (
        (target === 'adoption') !== (call.path === adoptionPath) ||
        (target !== 'adoption' && values.target !== target)
      )
        return reply(route, { error: { code: 'case_stage_transition_rejected' } }, 409);
      const result = entry(values, values.expected_revision + 1);
      if (mode.commit !== false) {
        state.stageHead = result;
        state.stageHistory.unshift(result);
      }
      await wait(call);
      return mode.status
        ? reply(route, { error: { code: 'service_busy' } }, mode.status)
        : reply(route, { case_id: caseId, current: result }, 201);
    },
  );
  return state;
}
