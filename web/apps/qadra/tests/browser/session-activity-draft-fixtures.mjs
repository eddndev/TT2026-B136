import { expect } from '@playwright/test';
import { caseId } from './helpers.mjs';
import { browserResource } from './procedural-resources-helpers.mjs';
import {
  resourceDraftSetup,
  checkResourceDraftRequests,
  holdResourceRequest,
  resourcesPath,
  casePath,
} from './session-resource-draft-fixtures.mjs';
import { createHearingDraftIo } from './session-hearing-draft-io.mjs';
import { installActivityCommands } from './session-activity-command-fixtures.mjs';
import { installActivityDeadlineReads } from './session-activity-target-fixtures.mjs';
export { casePath, resourcesPath };
export const activitiesPath = (resourceId) => `${resourcesPath}/${resourceId}/activities`;
export const holdActivityRequest = holdResourceRequest;
const states = new WeakMap();
export async function checkActivityDraftRequests(page) {
  await checkResourceDraftRequests(page);
  const state = states.get(page);
  if (!state) return;
  expect(
    state.activityPrepareBudget,
    'Every association preparation requires an explicit review',
  ).toBe(0);
  expect(state.nextActivityWrite, 'An armed association write must occur exactly once').toBeNull();
  expect(state.resourcePosts, 'An association never mutates the resource').toEqual([]);
  expect(state.hearingPosts, 'An association never mutates its hearing').toEqual([]);
  expect(state.uploads, 'This association editor has no document upload surface').toEqual([]);
}
export async function activityDraftSetup(page) {
  const original = browserResource(),
    state = await resourceDraftSetup(page, { resources: [original] });
  Object.assign(state, {
    original,
    associations: new Map(),
    activityPrepares: [],
    activityPosts: [],
    activityPrepareBudget: 0,
    nextActivityWrite: null,
    deniedAssociations: new Set(),
  });
  states.set(page, state);
  installActivityCommands(state, original);
  const io = createHearingDraftIo(state),
    { authorize, reply, fail, unexpected, wait } = io;
  await page.route('**/api/v1/cases/*/procedural-resources/*/activities**', async (route) => {
    const call = await authorize(route);
    if (!call) return;
    const match = /^\/api\/v1\/cases\/[^/]+\/procedural-resources\/([^/]+)\/activities(.*)$/.exec(
      call.path,
    );
    const resourceId = match?.[1],
      parts = match?.[2].split('/').filter(Boolean),
      resource = state.resources.get(resourceId)?.at(-1),
      query = new URLSearchParams(call.search);
    if (!resource) return fail(route, 'procedural_resource_not_found', 404);
    const rows = state.associations.get(parts[0]);
    if (state.deniedAssociations.has(parts[0]))
      return fail(route, 'resource_activity_not_found', 404);
    if (call.method === 'GET' && call.body === null) {
      let result;
      if (!parts.length) {
        if ([...query.keys()].some((key) => !['limit', 'kind', 'status', 'after_id'].includes(key)))
          return unexpected(route, call);
        const selected = [...state.associations.values()]
          .map((history) => history.at(-1))
          .filter(
            (row) =>
              row.resource_id === resourceId &&
              (!query.has('kind') || row.selection.target.kind === query.get('kind')) &&
              (!query.has('status') || row.status === query.get('status')) &&
              (!query.has('after_id') || row.id > query.get('after_id')),
          )
          .sort((a, b) => a.id.localeCompare(b.id));
        const limit = Number(query.get('limit') || 20),
          pageRows = selected.slice(0, limit),
          more = selected.length > limit;
        result = {
          case_id: caseId,
          resource_id: resourceId,
          associations: pageRows.map(state.activityView),
          has_more: more,
          next_after_id: more ? pageRows.at(-1).id : null,
        };
      } else if (!rows || rows[0].resource_id !== resourceId)
        return fail(route, 'resource_activity_not_found', 404);
      else if (parts.length === 1 && !call.search) result = state.activityView(rows.at(-1));
      else if (parts.length === 3 && parts[1] === 'revisions' && !call.search) {
        const row = rows.find((entry) => entry.revision === Number(parts[2]));
        if (row) result = state.activityView(row);
      } else if (parts.length === 2 && parts[1] === 'history') {
        if ([...query.keys()].some((key) => !['limit', 'before_revision'].includes(key)))
          return unexpected(route, call);
        const selected = [...rows]
            .reverse()
            .filter((row) => row.revision < Number(query.get('before_revision') || Infinity)),
          limit = Number(query.get('limit') || 20),
          pageRows = selected.slice(0, limit),
          more = selected.length > limit;
        result = {
          case_id: caseId,
          resource_id: resourceId,
          association_id: parts[0],
          revisions: pageRows,
          has_more: more,
          next_before_revision: more ? pageRows.at(-1).revision : null,
        };
      } else return unexpected(route, call);
      if (!result) return fail(route, 'resource_activity_not_found', 404);
      await wait(call);
      return reply(route, result);
    }
    const body = route.request().postDataJSON(),
      preparing = parts.length === 1 && parts[0] === 'prepare',
      command = preparing ? body : body?.command,
      change = command?.change;
    if (
      !change ||
      command.case_id !== caseId ||
      command.resource_id !== resourceId ||
      call.search ||
      call.method !== 'POST' ||
      (preparing ? !state.activityPrepareBudget : !state.nextActivityWrite)
    )
      return unexpected(route, call);
    if (
      !preparing &&
      !(change.action === 'link'
        ? !parts.length
        : change.action === 'unlink' &&
          parts.length === 2 &&
          parts[0] === command.association_id &&
          parts[1] === 'unlink')
    )
      return unexpected(route, call);
    let mode;
    if (preparing) {
      state.activityPrepareBudget--;
      state.activityPrepares.push({ ...call, values: structuredClone(command) });
    } else {
      mode = state.nextActivityWrite;
      state.nextActivityWrite = null;
      state.activityPosts.push({ ...call, values: structuredClone(body) });
    }
    if (state.caseStatus === 'closed') return fail(route, 'case_closed');
    if (command.expected_resource_revision !== resource.revision)
      return fail(route, 'resource_activity_resource_revision_conflict');
    const current = state.associations.get(command.association_id)?.at(-1);
    if ((current?.revision || 0) !== change.expected_revision)
      return fail(route, 'resource_activity_revision_conflict');
    if (change.action === 'link' && resource.status !== 'active')
      return fail(route, 'resource_activity_resource_archived');
    const prepared = state.activityPrepare(command);
    if (preparing) return reply(route, prepared);
    if (body.expected_submission_digest !== prepared.submission_digest)
      return fail(route, 'resource_activity_submission_mismatch');
    const result = mode.commit === false ? null : state.activityCommit(prepared);
    await wait(call);
    return mode.status ? fail(route, 'service_busy', mode.status) : reply(route, result, 201);
  });
  await installActivityDeadlineReads(page, state, io);
  return state;
}
