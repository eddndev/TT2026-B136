import { expect } from '@playwright/test';
import { caseId } from './helpers.mjs';
import {
  hearingDraftSetup,
  checkHearingDraftRequests,
  holdHearingRead,
  casePath,
} from './session-hearing-draft-fixtures.mjs';
import { createHearingDraftIo } from './session-hearing-draft-io.mjs';
import { prepareFact } from '../fixtures/procedural-facts-sources.mjs';
import {
  factAdministration,
  factCommand,
  factPrepared,
  factRecord,
  factKey,
  factRow,
  factHistoryRow,
  factResolutionSource,
  emptyFactSources,
  resolutionId,
  notificationId,
} from '../fixtures/procedural-facts.mjs';

export { casePath, resolutionId, notificationId, factKey };
export const factsPath = `/api/v1/cases/${caseId}/resolutions`;
export const resolutionPath = `${factsPath}/${resolutionId}`;
export const notificationsPath = `${resolutionPath}/notifications`;
export const notificationPath = `${notificationsPath}/${notificationId}`;
export const holdFactRequest = holdHearingRead;
const states = new WeakMap();

export function notificationRecord(parent = factRecord()) {
  const command = factCommand('notification');
  command.resolution_id = parent.id;
  command.change.values.resolution = { id: parent.id, revision: parent.revision };
  const sources = emptyFactSources();
  sources.resolution = factResolutionSource(parent);
  return factRecord(factPrepared(command, null, sources));
}
export async function checkFactDraftRequests(page) {
  await checkHearingDraftRequests(page);
  const state = states.get(page);
  if (!state) return;
  expect(state.factPrepareBudget, 'Only explicit review clicks may prepare a fact').toBe(0);
  expect(state.nextFactWrite, 'Each armed fact write must occur once').toBeNull();
}
export async function factDraftSetup(page, { facts = [], ...options } = {}) {
  const state = await hearingDraftSetup(page, options);
  Object.assign(state, {
    factRecords: new Map(),
    factPrepares: [],
    factPosts: [],
    factPrepareBudget: 0,
    nextFactWrite: null,
    deniedFacts: new Set(),
  });
  for (const row of facts) {
    const key = factKey(row);
    state.factRecords.set(key, [...(state.factRecords.get(key) || []), structuredClone(row)]);
  }
  states.set(page, state);
  state.factPrepare = (command) => {
    const prepared = prepareFact(
      {
        records: state.factRecords,
        results: { directory: state.records, records: state.results },
      },
      command,
    );
    prepared.actor_id = state.current.user.id;
    prepared.observed_administration = {
      ...structuredClone(factAdministration),
      revision: state.caseRevision,
      status: state.caseStatus,
    };
    return prepared;
  };
  state.factCommit = (prepared) => {
    const row = factRecord(prepared),
      key = factKey(row);
    state.factRecords.set(key, [...(state.factRecords.get(key) || []), row]);
    return row;
  };
  const { authorize, reply, fail, unexpected, wait } = createHearingDraftIo(state);
  await page.route('**/api/v1/cases/*/resolutions**', async (route) => {
    const call = await authorize(route);
    if (!call) return;
    const match = /\/cases\/([^/]+)\/resolutions(?:\/([^/]+)\/notifications)?(.*)/.exec(call.path);
    if (!match || match[1] !== caseId) return fail(route, 'case_not_found', 404);
    const family = match[2] ? 'notification' : 'resolution',
      parent = match[2];
    const parts = match[3].split('/').filter(Boolean),
      url = new URL(route.request().url());
    const key = factKey({ family, resolution_id: parent, id: parts[0] });
    if (state.deniedFacts.has(key)) return fail(route, 'procedural_fact_not_found', 404);
    if (call.method === 'GET' && call.body === null) {
      let value;
      const rows = state.factRecords.get(key);
      if (!parts.length) {
        const status = url.searchParams.get('status') || 'all';
        const after = url.searchParams.get('after_id'),
          limit = Number(url.searchParams.get('limit') || 20);
        const selected = [...state.factRecords.values()]
          .map((items) => items.at(-1))
          .filter(
            (row) =>
              row.family === family &&
              row.resolution_id === parent &&
              (status === 'all' || row.status === status) &&
              (!after || row.id > after),
          )
          .sort((a, b) => a.id.localeCompare(b.id));
        const pageRows = selected.slice(0, limit),
          more = selected.length > limit;
        value = {
          [family === 'resolution' ? 'resolutions' : 'notifications']: pageRows.map(factRow),
          has_more: more,
          next_after_id: more ? pageRows.at(-1).id : null,
        };
      } else if (!rows) return fail(route, 'procedural_fact_not_found', 404);
      else if (parts.length === 1) value = rows.at(-1);
      else if (parts.length === 2 && parts[1] === 'history') {
        const before = Number(url.searchParams.get('before_revision') || Infinity);
        const limit = Number(url.searchParams.get('limit') || 10);
        const selected = [...rows].reverse().filter((row) => row.revision < before);
        const pageRows = selected.slice(0, limit),
          more = selected.length > limit;
        value = {
          revisions: pageRows.map(factHistoryRow),
          has_more: more,
          next_before_revision: more ? pageRows.at(-1).revision : null,
        };
      } else if (parts.length === 3 && parts[1] === 'revisions')
        value = rows.find((row) => row.revision === Number(parts[2]));
      else return unexpected(route, call);
      if (!value) return fail(route, 'procedural_fact_not_found', 404);
      const snapshot = structuredClone(value);
      await wait(call);
      return reply(route, snapshot);
    }
    const body = route.request().postDataJSON(),
      preparing = parts[0] === 'prepare';
    const command = preparing ? body : body?.command,
      change = command?.change;
    if (
      !change ||
      command.family !== family ||
      command.resolution_id !== parent ||
      call.search ||
      (preparing ? !state.factPrepareBudget : !state.nextFactWrite)
    )
      return unexpected(route, call);
    const valid = preparing
      ? call.method === 'POST' && parts.length === 1
      : change.action === 'record'
        ? call.method === 'POST' && parts.length === 0
        : command.id === parts[0] &&
          (change.action === 'correct'
            ? call.method === 'PUT' && parts.length === 1
            : change.action === 'withdraw' &&
              call.method === 'POST' &&
              parts.length === 2 &&
              parts[1] === 'withdrawal');
    if (!valid) return unexpected(route, call);
    let mode;
    if (preparing) {
      state.factPrepareBudget--;
      state.factPrepares.push({ ...call, values: command });
    } else {
      mode = state.nextFactWrite;
      state.nextFactWrite = null;
      state.factPosts.push({ ...call, values: body });
    }
    if (state.caseStatus === 'closed') return fail(route, 'case_closed');
    const current = state.factRecords.get(factKey(command))?.at(-1);
    if ((current?.revision || 0) !== change.expected_revision)
      return fail(route, 'procedural_fact_revision_conflict');
    if (current?.status === 'withdrawn') return fail(route, 'procedural_fact_already_withdrawn');
    const prepared = state.factPrepare(command);
    if (preparing) return reply(route, prepared);
    if (body.expected_submission_digest !== prepared.submission_digest)
      return fail(route, 'procedural_fact_submission_mismatch');
    const result = mode.commit === false ? null : state.factCommit(prepared);
    await wait(call);
    return mode.status ? fail(route, 'service_busy', mode.status) : reply(route, result, 201);
  });
  return state;
}
