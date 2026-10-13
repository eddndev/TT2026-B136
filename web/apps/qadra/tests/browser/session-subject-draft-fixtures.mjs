import { expect } from '@playwright/test';
import { caseRecord } from './helpers.mjs';
import { administration } from './case-administration-helpers.mjs';
import { subject, subjectId, typed } from './typed-participant-helpers.mjs';
import {
  participantDraftSetup,
  checkParticipantDraftRequests,
} from './session-participant-drafts-helpers.mjs';
import { casePath } from './session-inactivity-helpers.mjs';
import { installSubjectDocuments } from './session-subject-document-fixtures.mjs';

export { subject, subjectId, typed, casePath };
export const subjectPath = `/api/v1/cases/${caseRecord.id}/subjects/${subjectId}`;
export const reviewPath = `${subjectPath}/review`;
export const candidateId = '55555555-5555-4555-8555-555555555555';
export const candidate = {
  reference: { kind: 'subject', id: candidateId, revision: 1 },
  display_name: 'Otra identidad',
  kind: 'natural_person',
  signals: ['name'],
};
const states = new WeakMap();

export function holdSubjectRequest(state, method, path) {
  const gate = { method, path, entered: false, call: null, release: () => {} };
  gate.promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.subjectGates.push(gate);
  return gate;
}

export async function checkSubjectRequests(page) {
  const state = states.get(page);
  state?.subjectGates.forEach((gate) => gate.release());
  await checkParticipantDraftRequests(page);
  if (!state) return;
  expect(state.reviewBudget, 'Only explicit review clicks authorize review requests').toBe(0);
  expect(state.nextSubjectWrite, 'An armed identity replacement must be sent once').toBeNull();
  expect(state.nextUpload, 'An armed child upload must be sent once').toBeNull();
}

export async function subjectDraftSetup(page) {
  const state = await participantDraftSetup(page);
  Object.assign(state, {
    records: new Map([[typed.id, [structuredClone(typed)]]]),
    subjects: new Map([[subjectId, [structuredClone(subject)]]]),
    caseStatus: 'active',
    caseRevision: 1,
    subjectGates: [],
    deniedSubjects: new Set(),
    candidates: [],
    directoryStamp: 'c'.repeat(64),
    reviewBudget: 0,
    reviews: [],
    subjectWrites: [],
    nextSubjectWrite: null,
  });
  states.set(page, state);
  const reply = (route, json, status = 200) =>
    route.fulfill({
      status,
      json,
      headers: { 'Cache-Control': 'no-store' },
    });
  function unexpected(route, call) {
    state.writes.push(call);
    return reply(route, { error: { code: 'unexpected_subject_draft_request' } }, 501);
  }
  async function authorize(route) {
    const request = route.request(),
      url = new URL(request.url());
    const call = {
      path: url.pathname,
      search: url.search,
      method: request.method(),
      body: request.postData(),
      headers: request.headers(),
      at: state.now,
    };
    state.calls.push(call);
    if (
      !state.current ||
      call.headers.authorization !== `Bearer ${state.current.token}` ||
      state.now >= Math.min(state.current.absolute, state.current.deadline)
    ) {
      await reply(route, { error: { code: 'invalid_session' } }, 401);
      return null;
    }
    if (!state.allowed) {
      await reply(route, { error: { code: 'case_not_found' } }, 404);
      return null;
    }
    const roles =
      call.method === 'GET' ? ['owner', 'litigator', 'paralegal'] : ['owner', 'litigator'];
    if (!roles.includes(state.current.user.role)) {
      await reply(route, { error: { code: 'permission_denied' } }, 403);
      return null;
    }
    return call;
  }
  async function wait(call) {
    const gate = state.subjectGates.find(
      (item) => !item.entered && item.method === call.method && item.path === call.path,
    );
    if (!gate) return;
    gate.entered = true;
    gate.call = call;
    await gate.promise;
  }
  await page.route(
    (url) =>
      url.pathname === casePath ||
      url.pathname.startsWith(`/api/v1/cases/${caseRecord.id}/subjects/`),
    async (route) => {
      const call = await authorize(route);
      if (!call) return;
      if (call.path === casePath && call.method === 'GET' && call.body === null && !call.search) {
        const result = administration(caseRecord, state.caseRevision, null, state.caseStatus);
        await wait(call);
        return reply(route, result);
      }
      const parts = call.path.split('/subjects/')[1]?.split('/') || [];
      const rows = state.subjects.get(parts[0]);
      if (state.deniedSubjects.has(parts[0]))
        return reply(route, { error: { code: 'subject_not_found' } }, 404);
      if (!rows) return reply(route, { error: { code: 'subject_not_found' } }, 404);
      if (call.method === 'GET' && call.body === null && !call.search) {
        const result =
          parts.length === 1
            ? rows.at(-1)
            : parts.length === 3 && parts[1] === 'revisions'
              ? rows.find((row) => row.revision === Number(parts[2]))
              : null;
        await wait(call);
        return result
          ? reply(route, result)
          : reply(route, { error: { code: 'subject_not_found' } }, 404);
      }
      const body = route.request().postDataJSON();
      if (call.path === reviewPath && call.method === 'POST' && !call.search) {
        if (!state.reviewBudget) return unexpected(route, call);
        state.reviewBudget--;
        state.reviews.push({ ...call, values: body });
        if (body.expected_revision !== rows.at(-1).revision)
          return reply(route, { error: { code: 'subject_revision_conflict' } }, 409);
        await wait(call);
        return reply(route, {
          case_id: caseRecord.id,
          id: subjectId,
          ...body,
          directory_stamp: state.directoryStamp,
          candidates: state.candidates,
        });
      }
      if (call.path === subjectPath && call.method === 'PUT' && !call.search) {
        const mode = state.nextSubjectWrite;
        if (!mode) return unexpected(route, call);
        state.nextSubjectWrite = null;
        state.subjectWrites.push({ ...call, values: body });
        if (state.caseStatus === 'closed')
          return reply(route, { error: { code: 'case_closed' } }, 409);
        if (body.expected_revision !== rows.at(-1).revision)
          return reply(route, { error: { code: 'subject_revision_conflict' } }, 409);
        if (body.review?.directory_stamp !== state.directoryStamp)
          return reply(route, { error: { code: 'participant_review_stale' } }, 409);
        const result = {
          ...rows.at(-1),
          revision: body.expected_revision + 1,
          values: body.values,
        };
        if (mode.commit !== false) rows.push(result);
        await wait(call);
        return mode.status
          ? reply(route, { error: { code: 'service_busy' } }, mode.status)
          : reply(route, result);
      }
      return unexpected(route, call);
    },
  );
  await installSubjectDocuments(page, state, { authorize, wait, reply, unexpected });
  return state;
}
