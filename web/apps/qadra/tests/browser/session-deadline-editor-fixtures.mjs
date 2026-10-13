import { createHash } from 'node:crypto';
import { expect } from '@playwright/test';
import { caseId } from './helpers.mjs';
import { profile, id } from '../fixtures/deadline-unit.mjs';
import { browserDeadlinePrepared, prepareBrowserDeadline } from '../fixtures/deadline-browser.mjs';
import { v2Record, summary, historyRow, notChecked } from '../fixtures/deadline-v2-unit.mjs';
import { activityResourcePage } from './activity-resources-fixtures.mjs';
import {
  hearingDraftSetup,
  checkHearingDraftRequests,
  holdHearingRead,
  casePath,
} from './session-hearing-draft-fixtures.mjs';
import { createHearingDraftIo } from './session-hearing-draft-io.mjs';

export { casePath };
export const deadlinesPath = `/api/v1/cases/${caseId}/deadlines`;
export const profilesPath = `/api/v1/cases/${caseId}/deadline-profiles`;
export const holdDeadlineRead = holdHearingRead;
export const initialDeadline = () => v2Record(browserDeadlinePrepared(caseId));
const states = new WeakMap();
const clone = (value) => structuredClone(value);
const digest = (value) => createHash('sha256').update(JSON.stringify(value)).digest('hex');
const queryAllowed = (query, names) => [...query.keys()].every((key) => names.includes(key));

export async function checkDeadlineEditorRequests(page) {
  await checkHearingDraftRequests(page);
  const state = states.get(page);
  if (!state) return;
  expect(state.deadlinePrepareBudget, 'Each preparation requires an explicit review click').toBe(0);
  expect(state.nextDeadlineWrite, 'Each armed deadline write must be sent exactly once').toBeNull();
}

export async function deadlineEditorSetup(page, { deadlines = [] } = {}) {
  const state = await hearingDraftSetup(page);
  Object.assign(state, {
    profiles: [profile(caseId)],
    responsibles: [{ id: id(4), email: 'staff@example.test', role: 'owner' }],
    deadlineRecords: new Map(),
    deadlineDrafts: new Map(),
    deniedDeadlines: new Set(),
    deadlinePrepares: [],
    deadlinePosts: [],
    deadlinePrepareBudget: 0,
    nextDeadlineWrite: null,
  });
  states.set(page, state);
  for (const row of deadlines)
    state.deadlineRecords.set(row.id, [...(state.deadlineRecords.get(row.id) || []), clone(row)]);
  // These transport fixtures preserve exact commands and receipts, not cryptographic evidence.
  state.deadlinePrepare = (command) => {
    const current = state.deadlineRecords.get(command.deadline_id)?.at(-1);
    const prepared = prepareBrowserDeadline(command, current, state, caseId);
    prepared.actor_id = state.current.user.id;
    prepared.author = { kind: 'user', id: state.current.user.id, email: state.current.user.email };
    prepared.review_digest = digest(prepared.calculation);
    prepared.capture_digest = digest({
      definition: prepared.definition,
      attention: prepared.attention,
    });
    prepared.submission_digest = digest(prepared);
    state.deadlineDrafts.set(command.operation_id, clone(prepared));
    return prepared;
  };
  state.deadlineCommit = (prepared) => {
    const row = v2Record(prepared);
    state.deadlineRecords.set(row.id, [...(state.deadlineRecords.get(row.id) || []), row]);
    return row;
  };
  const { authorize, reply, fail, unexpected, wait } = createHearingDraftIo(state);
  await page.route('**/api/v1/cases/*/deadline-profiles**', async (route) => {
    const call = await authorize(route);
    if (!call) return;
    const parts = call.path.slice(profilesPath.length).split('/').filter(Boolean);
    const query = new URLSearchParams(call.search),
      collection = { kind: 'case', case_id: caseId };
    if (call.method !== 'GET' || call.body !== null || !call.path.startsWith(profilesPath))
      return unexpected(route, call);
    let result;
    if (!parts.length && queryAllowed(query, ['limit', 'status', 'after_id'])) {
      result = {
        collection,
        profiles: state.profiles.map((row) => ({
          id: row.id,
          revision: row.revision,
          status: row.status,
          algorithm: row.algorithm,
          definition_digest: row.definition_digest,
          title: row.definition.title,
          scope: row.scope,
        })),
        has_more: false,
        next_after_id: null,
      };
    } else {
      const rows = state.profiles.filter((row) => row.id === parts[0]);
      if (!rows.length) return fail(route, 'deadline_profile_not_found', 404);
      if (
        parts.length === 2 &&
        parts[1] === 'history' &&
        queryAllowed(query, ['limit', 'before_revision'])
      )
        result = {
          collection,
          revisions: rows.map(({ definition, collection: ignored, ...header }) => header),
          has_more: false,
          next_before_revision: null,
        };
      else if (parts.length === 3 && parts[1] === 'revisions' && !call.search) {
        const row = rows.find((entry) => entry.revision === Number(parts[2]));
        if (!row) return fail(route, 'deadline_profile_not_found', 404);
        result = { ...row, collection };
      } else return unexpected(route, call);
    }
    const captured = clone(result);
    await wait(call);
    return reply(route, captured);
  });
  await page.route('**/api/v1/cases/*/deadlines**', async (route) => {
    const call = await authorize(route);
    if (!call) return;
    if (!call.path.startsWith(deadlinesPath)) return unexpected(route, call);
    const parts = call.path.slice(deadlinesPath.length).split('/').filter(Boolean);
    const query = new URLSearchParams(call.search);
    if (state.deniedDeadlines.has(parts[0])) return fail(route, 'deadline_not_found', 404);
    if (call.method === 'GET' && call.body === null) {
      const rows = state.deadlineRecords.get(parts[0]);
      let result;
      if (!parts.length && queryAllowed(query, ['limit', 'status', 'after_id'])) {
        const status = query.get('status') || 'active',
          after = query.get('after_id');
        result = {
          case_id: caseId,
          deadlines: [...state.deadlineRecords.values()]
            .map((items) => items.at(-1))
            .filter(
              (row) => (!after || row.id > after) && (status === 'all' || row.status === status),
            )
            .sort((a, b) => a.id.localeCompare(b.id))
            .map(summary),
          has_more: false,
          next_after_id: null,
        };
      } else if (
        parts.length === 1 &&
        parts[0] === 'responsibles' &&
        queryAllowed(query, ['limit', 'after_id'])
      )
        result = {
          case_id: caseId,
          responsibles: state.responsibles,
          has_more: false,
          next_after_id: null,
        };
      else if (parts.length === 1 && !call.search) result = rows?.at(-1);
      else if (parts.length === 3 && parts[1] === 'revisions' && !call.search) {
        const row = rows?.find((item) => item.revision === Number(parts[2]));
        if (row) result = { ...row, operational: notChecked() };
      } else if (
        parts.length === 2 &&
        parts[1] === 'history' &&
        queryAllowed(query, ['limit', 'before_revision'])
      )
        result = {
          case_id: caseId,
          id: parts[0],
          revisions: [...(rows || [])]
            .reverse()
            .filter(
              (row) =>
                !query.has('before_revision') ||
                row.revision < Number(query.get('before_revision')),
            )
            .map(historyRow),
          has_more: false,
          next_before_revision: null,
        };
      else if (parts.length === 2 && parts[1] === 'resource-associations')
        result = activityResourcePage(caseId, 'deadline', parts[0], new URL(route.request().url()));
      else return unexpected(route, call);
      const captured = result === undefined ? undefined : clone(result);
      await wait(call);
      return captured === undefined
        ? fail(route, 'deadline_not_found', 404)
        : reply(route, captured);
    }
    const body = route.request().postDataJSON(),
      preparing = call.path === `${deadlinesPath}/prepare`;
    const command = preparing ? body : body?.command,
      change = command?.change;
    if (
      !change ||
      call.search ||
      (preparing ? !state.deadlinePrepareBudget : !state.nextDeadlineWrite)
    )
      return unexpected(route, call);
    const path =
      change.action === 'register'
        ? deadlinesPath
        : `${deadlinesPath}/${command.deadline_id}${change.action === 'set_attention' ? '/attention' : change.action === 'retire' ? '/retirement' : ''}`;
    if (
      (preparing && call.method !== 'POST') ||
      (!preparing &&
        (call.path !== path || call.method !== (change.action === 'correct' ? 'PUT' : 'POST')))
    )
      return unexpected(route, call);
    let mode;
    if (preparing) {
      state.deadlinePrepareBudget--;
      state.deadlinePrepares.push({ ...call, values: clone(command) });
    } else {
      mode = state.nextDeadlineWrite;
      state.nextDeadlineWrite = null;
      state.deadlinePosts.push({ ...call, values: clone(body) });
    }
    if (state.caseStatus === 'closed') return fail(route, 'case_closed');
    const current = state.deadlineRecords.get(command.deadline_id)?.at(-1);
    if (change.expected_revision !== (current?.revision || 0))
      return fail(route, 'deadline_revision_conflict');
    if (current?.status === 'retired') return fail(route, 'deadline_retired');
    if (preparing) {
      const result = state.deadlinePrepare(command);
      await wait(call);
      return reply(route, result);
    }
    const draft = state.deadlineDrafts.get(command.operation_id);
    if (
      !draft ||
      JSON.stringify(command) !== JSON.stringify(draft.command) ||
      body.expected_submission_digest !== draft.submission_digest
    )
      return fail(route, 'deadline_submission_mismatch');
    const result = mode.commit === false ? null : state.deadlineCommit(draft);
    await wait(call);
    return mode.status ? fail(route, 'service_busy', mode.status) : reply(route, result, 201);
  });
  return state;
}
