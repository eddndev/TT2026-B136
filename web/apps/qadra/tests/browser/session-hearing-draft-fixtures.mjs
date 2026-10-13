import { expect } from '@playwright/test';
import { caseId, caseRecord } from './helpers.mjs';
import { administration, profile } from './case-administration-helpers.mjs';
import { activityResourcePage } from './activity-resources-fixtures.mjs';
import { participant } from './participant-helpers.mjs';
import {
  subjectDraftSetup,
  checkSubjectRequests,
  casePath,
} from './session-subject-draft-fixtures.mjs';
import {
  hearingContext,
  hearingPrepared,
  hearingRecord,
  hearingRow,
  hearingId,
} from '../fixtures/hearings.mjs';
import { installResultDraftRoutes } from './session-result-draft-fixtures.mjs';
import { createHearingDraftIo } from './session-hearing-draft-io.mjs';
import { precautionaryBrowserRecord } from '../fixtures/precautionary-hearing-browser.mjs';

export { casePath, hearingId, participant };
export const hearingsPath = `/api/v1/cases/${caseId}/hearings`;
export const hearingPath = `${hearingsPath}/${hearingId}`;
const states = new WeakMap();

export function holdHearingRead(state, method, path) {
  const gate = { method, path, entered: false, call: null, release: () => {} };
  gate.promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.hearingGates.push(gate);
  return gate;
}

export async function checkHearingDraftRequests(page) {
  const state = states.get(page);
  state?.hearingGates.forEach((gate) => gate.release());
  await checkSubjectRequests(page);
  if (!state) return;
  expect(state.prepareBudget, 'Preparations require an explicit review click').toBe(0);
  expect(state.nextHearingWrite, 'An armed hearing write must be sent once').toBeNull();
  expect(state.nextResultWrite, 'An armed result write must be sent once').toBeNull();
}

export async function hearingDraftSetup(
  page,
  { hearings = [], results = [], stage = 'investigation' } = {},
) {
  const state = await subjectDraftSetup(page);
  Object.assign(state, {
    hearings: new Map(),
    results: new Map(),
    hearingGates: [],
    hearingPosts: [],
    resultPosts: [],
    prepareBudget: 0,
    preparations: [],
    nextHearingWrite: null,
    nextResultWrite: null,
    hearingContext: { ...structuredClone(hearingContext), stage },
    deniedHearings: new Set(),
    deniedResults: new Set(),
    failures: new Map(),
  });
  state.records.set(participant.id, [structuredClone(participant)]);
  for (const row of hearings)
    state.hearings.set(row.id, [...(state.hearings.get(row.id) || []), structuredClone(row)]);
  for (const row of results)
    state.results.set(row.id, [...(state.results.get(row.id) || []), structuredClone(row)]);
  states.set(page, state);
  const { authorize, reply, fail, unexpected, wait } = createHearingDraftIo(state);
  const precautionaryContextPath = `/api/v1/cases/${caseId}/precautionary-context`;
  const precautionaryListPath = `/api/v1/cases/${caseId}/precautionary-hearings`;
  await page.route(
    (url) => [precautionaryContextPath, precautionaryListPath].includes(url.pathname),
    async (route) => {
      const call = await authorize(route);
      if (!call) return;
      const isContext = call.path === precautionaryContextPath;
      if (
        call.method !== 'GET' ||
        call.body !== null ||
        call.search !== (isContext ? '' : '?limit=10')
      )
        return unexpected(route, call);
      const context = precautionaryBrowserRecord(caseId).capture.review.observed_context;
      context.administration.revision = state.caseRevision;
      context.administration.administrative_status = state.caseStatus;
      context.stage.stage = state.hearingContext.stage;
      context.stage.stage_revision = state.hearingContext.stage_revision;
      context.expectation.administration_revision = state.caseRevision;
      context.expectation.stage_revision = state.hearingContext.stage_revision;
      await wait(call);
      return reply(
        route,
        isContext ? context : { case_id: caseId, items: [], has_more: false, next_after_id: null },
      );
    },
  );
  state.hearingPrepare = (command) => {
    const prepared = hearingPrepared(command),
      previous = state.hearings.get(command.hearing_id)?.at(-1);
    prepared.actor_id = state.current.user.id;
    if (command.change.action === 'cancel') prepared.values = structuredClone(previous.values);
    return prepared;
  };
  state.hearingCommit = (prepared) => {
    const row = hearingRecord(prepared),
      previous = state.hearings.get(row.id)?.at(-1);
    row.participants = row.values.participants.map((ref) => ({
      ...structuredClone(
        state.records.get(ref.participant_id).find((person) => person.revision === ref.revision),
      ),
      profile: 'manual',
    }));
    if (row.values.conviction_basis)
      row.support = {
        ...row.values.conviction_basis.support,
        name: 'antecedente.pdf',
        format: 'pdf',
        policy: 'pdf_docx_v1',
      };
    row.scheduling_context =
      prepared.command.change.action === 'cancel'
        ? previous.scheduling_context
        : {
            administration_revision: state.caseRevision,
            administration_digest: 'c'.repeat(64),
            stage_revision: state.hearingContext.stage_revision,
            stage: state.hearingContext.stage,
            stage_digest: null,
          };
    state.hearings.set(row.id, [...(state.hearings.get(row.id) || []), row]);
    return row;
  };
  await page.route(
    (url) =>
      url.pathname === casePath ||
      url.pathname.startsWith(hearingsPath) ||
      /\/participants\/[^/]+\/revisions\/\d+$/.test(url.pathname),
    async (route) => {
      const call = await authorize(route);
      if (!call) return;
      const url = new URL(route.request().url());
      if (call.method === 'GET' && call.body === null) {
        let result;
        if (call.path === casePath && !call.search)
          result = administration(caseRecord, state.caseRevision, profile, state.caseStatus);
        else if (/\/participants\/[^/]+\/revisions\/\d+$/.test(call.path)) {
          const [, id, revision] = /\/participants\/([^/]+)\/revisions\/(\d+)$/.exec(call.path);
          result = state.records.get(id)?.find((row) => row.revision === Number(revision));
          if (!result) return fail(route, 'participant_not_found', 404);
        } else if (call.path === `${hearingsPath}/context`)
          result = {
            ...state.hearingContext,
            case_revision: state.caseRevision,
            administrative_status: state.caseStatus,
          };
        else {
          const parts = call.path.slice(hearingsPath.length).split('/').filter(Boolean);
          const rows = state.hearings.get(parts[0]);
          if (state.deniedHearings.has(parts[0])) return fail(route, 'hearing_not_found', 404);
          if (!parts.length) {
            const status = url.searchParams.get('status') || 'all';
            result = {
              hearings: [...state.hearings.values()]
                .map((rows) => rows.at(-1))
                .filter((row) => status === 'all' || row.status === status)
                .map(hearingRow),
              has_more: false,
              next_after_id: null,
            };
          } else if (parts[1] === 'resource-associations')
            result = activityResourcePage(caseId, 'hearing', parts[0], url);
          else if (parts[1] === 'history')
            result = {
              revisions: [...(rows || [])].reverse(),
              has_more: false,
              next_before_revision: null,
            };
          else if (parts.length === 1) result = rows?.at(-1);
          else if (parts.length === 3 && parts[1] === 'revisions')
            result = rows?.find((row) => row.revision === Number(parts[2]));
          else return unexpected(route, call);
          if (!result) return fail(route, 'hearing_not_found', 404);
        }
        const snapshot = structuredClone(result);
        await wait(call);
        return reply(route, snapshot);
      }
      const body = route.request().postDataJSON(),
        preparing = call.path === `${hearingsPath}/prepare`;
      const command = preparing ? body : body.command,
        change = command?.change;
      if (!change || call.search || (preparing ? !state.prepareBudget : !state.nextHearingWrite))
        return unexpected(route, call);
      const path =
        change.action === 'schedule'
          ? hearingsPath
          : `${hearingsPath}/${command.hearing_id}${change.action === 'cancel' ? '/cancellation' : ''}`;
      if (
        (preparing && call.method !== 'POST') ||
        (!preparing &&
          (call.path !== path || call.method !== (change.action === 'replace' ? 'PUT' : 'POST')))
      )
        return unexpected(route, call);
      let mode;
      if (preparing) {
        state.prepareBudget--;
        state.preparations.push({ ...call, values: command });
      } else {
        mode = state.nextHearingWrite;
        state.nextHearingWrite = null;
        state.hearingPosts.push({ ...call, values: body });
      }
      const current = state.hearings.get(command.hearing_id)?.at(-1);
      if (state.caseStatus === 'closed') return fail(route, 'case_closed');
      if (change.expected_revision !== (current?.revision || 0))
        return fail(route, 'hearing_revision_conflict');
      if (current?.status === 'cancelled') return fail(route, 'hearing_already_cancelled');
      if (
        change.action !== 'cancel' &&
        (change.expected_case_revision !== state.caseRevision ||
          change.expected_stage_revision !== state.hearingContext.stage_revision)
      )
        return fail(route, 'hearing_context_conflict');
      const prepared = state.hearingPrepare(command);
      if (preparing) return reply(route, prepared);
      if (body.expected_submission_digest !== prepared.submission_digest)
        return fail(route, 'hearing_submission_mismatch');
      const result = mode.commit === false ? null : state.hearingCommit(prepared);
      await wait(call);
      return mode.status ? fail(route, 'service_busy', mode.status) : reply(route, result, 201);
    },
  );
  await installResultDraftRoutes(page, state, { authorize, reply, fail, unexpected, wait });
  return state;
}
