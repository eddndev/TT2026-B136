import { expect } from '@playwright/test';
import { caseId } from './helpers.mjs';
import { profile, id } from '../fixtures/deadline-unit.mjs';
import {
  activityDraftSetup,
  checkActivityDraftRequests,
  holdActivityRequest,
  activitiesPath,
  casePath,
} from './session-activity-draft-fixtures.mjs';
import { createHearingDraftIo } from './session-hearing-draft-io.mjs';
import { installResourceDeadlineCommands } from './session-resource-deadline-commands.mjs';
import { installResourceDeadlineReads } from './session-resource-deadline-reads.mjs';

export { casePath, activitiesPath };
export const holdDeadlineRequest = holdActivityRequest;
export const compositePath = (resourceId) => `${activitiesPath(resourceId)}/deadlines`;
const states = new WeakMap();

export async function checkResourceDeadlineRequests(page) {
  await checkActivityDraftRequests(page);
  const state = states.get(page);
  if (!state) return;
  expect(state.compositePrepareBudget, 'Preparation must follow an explicit review action').toBe(0);
  expect(state.nextCompositeWrite, 'An armed composite write must occur exactly once').toBeNull();
  expect(state.activityPosts, 'The composite editor must not issue a separate link').toEqual([]);
}

export async function resourceDeadlineDraftSetup(page) {
  const state = await activityDraftSetup(page);
  Object.assign(state, {
    profiles: [profile(caseId)],
    responsibles: [{ id: id(4), email: 'staff@example.test', role: 'owner' }],
    deadlineRecords: new Map(),
    compositeDrafts: new Map(),
    jointOperations: new Map(),
    compositePrepares: [],
    compositePosts: [],
    compositePrepareBudget: 0,
    nextCompositeWrite: null,
  });
  states.set(page, state);
  installResourceDeadlineCommands(state);
  const io = createHearingDraftIo(state),
    { authorize, reply, fail, unexpected, wait } = io;
  await installResourceDeadlineReads(page, state, io);
  await page.route(
    '**/api/v1/cases/*/procedural-resources/*/activities/deadlines/**',
    async (route) => {
      const call = await authorize(route);
      if (!call) return;
      const preparing = call.path.endsWith('/prepare'),
        body = route.request().postDataJSON();
      const command = preparing ? body : body?.command,
        resourceId = command?.resource_id;
      if (
        call.method !== 'POST' ||
        call.search ||
        command?.case_id !== caseId ||
        call.path !== `${compositePath(resourceId)}/${preparing ? 'prepare' : 'submit'}` ||
        command?.deadline?.change.action !== 'register' ||
        command.deadline.change.expected_revision !== 0 ||
        (preparing ? !state.compositePrepareBudget : !state.nextCompositeWrite)
      )
        return unexpected(route, call);
      const head = state.resources.get(resourceId)?.at(-1);
      if (!head) return fail(route, 'procedural_resource_not_found', 404);
      if (preparing) {
        state.compositePrepareBudget--;
        state.compositePrepares.push({ ...call, values: structuredClone(command) });
        if (state.caseStatus === 'closed') return fail(route, 'case_closed');
        if (command.expected_resource_revision !== head.revision)
          return fail(route, 'resource_activity_resource_revision_conflict');
        const draft = state.compositePrepare(command);
        await wait(call);
        return reply(route, draft);
      }
      const mode = state.nextCompositeWrite;
      state.nextCompositeWrite = null;
      state.compositePosts.push({ ...call, values: structuredClone(body) });
      const operation = command.deadline.operation_id,
        draft = state.compositeDrafts.get(operation);
      if (
        !draft ||
        JSON.stringify(command) !== JSON.stringify(draft.command) ||
        body.expected_submission_digest !== draft.submission_digest
      )
        return fail(route, 'resource_activity_submission_mismatch');
      const joint = state.jointOperations.get(operation);
      let result, code;
      if (joint) result = joint.result;
      else if (
        state.deadlineRecords.has(command.deadline.deadline_id) ||
        state.associations.has(command.association_id)
      )
        code = 'resource_activity_operation_conflict';
      else if (state.caseStatus === 'closed') code = 'case_closed';
      else if (command.expected_resource_revision !== head.revision)
        code = 'resource_activity_resource_revision_conflict';
      else if (mode.commit !== false) result = state.compositeCommit(draft);
      await wait(call);
      if (code) return fail(route, code);
      return mode.status ? fail(route, 'service_busy', mode.status) : reply(route, result, 201);
    },
  );
  return state;
}
