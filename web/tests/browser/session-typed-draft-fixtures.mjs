import { expect } from '@playwright/test';
import { caseId } from './helpers.mjs';
import { participant } from './participant-helpers.mjs';
import {
  subjectDraftSetup,
  checkSubjectRequests,
  holdSubjectRequest,
  subject,
  typed,
  subjectId,
  casePath,
} from './session-subject-draft-fixtures.mjs';
import { typedCommand } from './session-typed-command-fixtures.mjs';

export { holdSubjectRequest as holdTypedRequest, subject, typed, subjectId, casePath };
export const participantsPath = `/api/v1/cases/${caseId}/participants`;
export const subjectsPath = `/api/v1/cases/${caseId}/subjects`;
export const proposalPath = `${participantsPath}/proposals`;
export const typedPath = `${participantsPath}/${typed.id}`;
const states = new WeakMap();

export async function checkTypedRequests(page) {
  await checkSubjectRequests(page);
  const state = states.get(page);
  if (!state) return;
  expect(state.typedReviewBudget).toBe(0);
  expect(state.typedPrepareBudget).toBe(0);
  expect(state.nextTypedCommit).toBeNull();
}

export async function typedDraftSetup(page) {
  const state = await subjectDraftSetup(page);
  state.records.set(participant.id, [structuredClone(participant)]);
  Object.assign(state, {
    typedReviewBudget: 0,
    typedPrepareBudget: 0,
    nextTypedCommit: null,
    typedReviews: [],
    typedPreparations: [],
    typedCommits: [],
    typedCandidates: [],
    credentialEvidence: new Map(),
    deniedParticipants: new Set(),
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
    return reply(route, { error: { code: 'unexpected_typed_draft_request' } }, 501);
  };
  async function wait(call) {
    const gate = state.subjectGates.find(
      (entry) => !entry.entered && entry.path === call.path && entry.method === call.method,
    );
    if (!gate) return;
    gate.entered = true;
    gate.call = call;
    await gate.promise;
  }
  await page.route(
    (url) => url.pathname === subjectsPath || url.pathname.startsWith(`${participantsPath}/`),
    async (route) => {
      const request = route.request(),
        url = new URL(request.url());
      const call = {
        path: url.pathname,
        method: request.method(),
        body: request.postData(),
        headers: request.headers(),
        search: url.search,
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
      if (call.path.startsWith(proposalPath) && call.method === 'POST' && !call.search)
        return typedCommand(route, call, state, { reply, wait, unexpected });
      if (call.method !== 'GET' || call.body !== null) return unexpected(route, call);
      if (call.path === subjectsPath) {
        const rows = [...state.subjects.values()]
          .map((entries) => entries.at(-1))
          .filter((row) => !state.deniedSubjects.has(row.id))
          .map((row) => ({
            case_id: caseId,
            id: row.id,
            revision: row.revision,
            kind: row.values.kind,
            display_name: row.values.name.value || row.values.name,
          }))
          .filter(
            (row) =>
              row.display_name.includes(url.searchParams.get('name') || '') &&
              (!url.searchParams.get('kind') || row.kind === url.searchParams.get('kind')),
          );
        await wait(call);
        return reply(route, { subjects: rows, has_more: false, next_after_id: null });
      }
      const parts = call.path.slice(participantsPath.length + 1).split('/');
      const rows = state.records.get(parts[0]);
      if (!rows || state.deniedParticipants.has(parts[0]))
        return reply(route, { error: { code: 'participant_not_found' } }, 404);
      let result;
      if (parts.length === 1 && !call.search) result = rows.at(-1);
      else if (parts[1] === 'revisions' && !call.search)
        result =
          parts[3] === 'credential'
            ? state.credentialEvidence.get(`${parts[0]}:${parts[2]}`)
            : rows.find((row) => row.revision === Number(parts[2]));
      else return route.fallback();
      await wait(call);
      return result
        ? reply(route, result)
        : reply(route, { error: { code: 'participant_not_found' } }, 404);
    },
  );
  return state;
}
