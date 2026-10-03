import { expect } from '@playwright/test';
import { caseId } from './helpers.mjs';
import { factRecord } from '../fixtures/procedural-facts.mjs';
import {
  factDraftSetup,
  checkFactDraftRequests,
  holdFactRequest,
  casePath,
} from './session-fact-draft-fixtures.mjs';
import { createHearingDraftIo } from './session-hearing-draft-io.mjs';
import { installResourceCommands } from './session-resource-command-fixtures.mjs';

export { casePath };
export const resourcesPath = `/api/v1/cases/${caseId}/procedural-resources`;
export const holdResourceRequest = holdFactRequest;
const states = new WeakMap();
export async function checkResourceDraftRequests(page) {
  await checkFactDraftRequests(page);
  const state = states.get(page);
  if (!state) return;
  expect(state.resourcePrepareBudget, 'Only explicit review clicks may prepare a resource').toBe(0);
  expect(state.nextResourceWrite, 'An armed resource write must occur exactly once').toBeNull();
}
export async function resourceDraftSetup(page, { resources = [], facts = [factRecord()] } = {}) {
  const state = await factDraftSetup(page, { facts });
  Object.assign(state, {
    resources: new Map(),
    resourcePrepares: [],
    resourcePosts: [],
    resourcePrepareBudget: 0,
    nextResourceWrite: null,
    deniedResources: new Set(),
  });
  for (const row of resources)
    state.resources.set(row.id, [...(state.resources.get(row.id) || []), structuredClone(row)]);
  states.set(page, state);
  installResourceCommands(state);
  const { authorize, reply, fail, unexpected, wait } = createHearingDraftIo(state);
  await page.route('**/api/v1/cases/*/procedural-resources**', async (route) => {
    const call = await authorize(route);
    if (!call) return;
    const parts = call.path.slice(resourcesPath.length).split('/').filter(Boolean),
      query = new URLSearchParams(call.search),
      rows = state.resources.get(parts[0]);
    if (state.deniedResources.has(parts[0]))
      return fail(route, 'procedural_resource_not_found', 404);
    if (call.method === 'GET' && call.body === null) {
      let result;
      if (!parts.length) {
        if ([...query.keys()].some((key) => !['limit', 'kind', 'status', 'after_id'].includes(key)))
          return unexpected(route, call);
        const selected = [...state.resources.values()]
          .map((history) => history.at(-1))
          .filter(
            (row) =>
              (!query.has('kind') || row.values.kind === query.get('kind')) &&
              (!query.has('status') || row.status === query.get('status')) &&
              (!query.has('after_id') || row.id > query.get('after_id')),
          )
          .sort((a, b) => a.id.localeCompare(b.id));
        const limit = Number(query.get('limit') || 20),
          pageRows = selected.slice(0, limit),
          more = selected.length > limit;
        result = {
          resources: pageRows,
          has_more: more,
          next_after_id: more ? pageRows.at(-1).id : null,
        };
      } else if (!rows) return fail(route, 'procedural_resource_not_found', 404);
      else if (parts.length === 1 && !call.search) result = rows.at(-1);
      else if (
        parts.length === 2 &&
        parts[1] === 'activities' &&
        call.search === '?limit=20&status=linked'
      )
        result = {
          case_id: caseId,
          resource_id: parts[0],
          associations: [],
          has_more: false,
          next_after_id: null,
        };
      else if (parts.length === 2 && parts[1] === 'history') {
        if ([...query.keys()].some((key) => !['limit', 'before_revision'].includes(key)))
          return unexpected(route, call);
        const selected = [...rows]
            .reverse()
            .filter((row) => row.revision < Number(query.get('before_revision') || Infinity)),
          limit = Number(query.get('limit') || 10),
          pageRows = selected.slice(0, limit),
          more = selected.length > limit;
        result = {
          revisions: pageRows,
          has_more: more,
          next_before_revision: more ? pageRows.at(-1).revision : null,
        };
      } else if (parts.length === 3 && parts[1] === 'revisions' && !call.search)
        result = rows.find((row) => row.revision === Number(parts[2]));
      else return unexpected(route, call);
      if (!result) return fail(route, 'procedural_resource_not_found', 404);
      const snapshot = structuredClone(result);
      await wait(call);
      return reply(route, snapshot);
    }
    const body = route.request().postDataJSON(),
      preparing = call.path === `${resourcesPath}/prepare`,
      command = preparing ? body : body?.command,
      change = command?.change;
    if (
      !change ||
      call.search ||
      (preparing ? !state.resourcePrepareBudget : !state.nextResourceWrite)
    )
      return unexpected(route, call);
    const suffix = {
      register: '',
      correct: `/${command.resource_id}`,
      record_act: `/${command.resource_id}/acts`,
      correct_act: `/${command.resource_id}/acts/${change.act_id}`,
      archive: `/${command.resource_id}/archive`,
      reactivate: `/${command.resource_id}/reactivation`,
    }[change.action];
    if (
      suffix === undefined ||
      (preparing
        ? call.method !== 'POST'
        : call.path !== resourcesPath + suffix ||
          call.method !== (['correct', 'correct_act'].includes(change.action) ? 'PUT' : 'POST'))
    )
      return unexpected(route, call);
    let mode;
    if (preparing) {
      state.resourcePrepareBudget--;
      state.resourcePrepares.push({ ...call, values: structuredClone(command) });
    } else {
      mode = state.nextResourceWrite;
      state.nextResourceWrite = null;
      state.resourcePosts.push({ ...call, values: structuredClone(body) });
    }
    if (state.caseStatus === 'closed') return fail(route, 'case_closed');
    const current = state.resources.get(command.resource_id)?.at(-1);
    if ((current?.revision || 0) !== change.expected_revision)
      return fail(route, 'procedural_resource_revision_conflict');
    if (current?.status === 'archived' && change.action !== 'reactivate')
      return fail(route, 'procedural_resource_archived');
    if (change.action === 'correct_act') {
      const prior = state.resources
        .get(command.resource_id)
        ?.findLast((row) => row.act?.id === change.act_id);
      if (prior?.act.revision !== change.expected_act_revision)
        return fail(route, 'procedural_resource_revision_conflict');
    }
    const prepared = state.resourcePrepare(command);
    if (preparing) return reply(route, prepared);
    if (body.expected_submission_digest !== prepared.submission_digest)
      return fail(route, 'procedural_resource_submission_mismatch');
    const result = mode.commit === false ? null : state.resourceCommit(prepared);
    await wait(call);
    return mode.status ? fail(route, 'service_busy', mode.status) : reply(route, result, 201);
  });
  return state;
}
