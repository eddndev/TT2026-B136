import {
  factInvalid as invalid,
  factObject as object,
  factSame as same,
  factRevision as revision,
} from './procedural-fact-primitives.mjs';
import {
  resourceHearingUuid as uuid,
  resourceHearingOverview,
} from './resource-hearing-values.mjs';
import {
  resourceHearingCreation,
  resourceHearingCreationScope,
  resourceHearingSubmission,
} from './resource-hearing-creation.mjs';
import { resourceHearingActor } from './resource-hearing-capture.mjs';
import { resourceHearingCommand, resourceHearingBudget } from './resource-hearing-command.mjs';
import { resourceHearingPrepared } from './resource-hearing-prepared.mjs';

export function resourceHearingsApi(request, caseId, resourceId) {
  uuid(caseId);
  uuid(resourceId);
  const base = `/cases/${caseId}/procedural-resources/${resourceId}/activities/resource-hearings`;
  let active = true;
  const assertActive = () => {
    if (!active) invalid('La consulta de audiencia de recurso ya no esta abierta.');
  };
  function scope(value) {
    if (value.case_id !== caseId || value.resource_id !== resourceId) invalid();
  }
  async function call(suffix, options) {
    assertActive();
    if (options?.data) resourceHearingBudget(options.data);
    try {
      const value = await request(base + suffix, options);
      assertActive();
      return value;
    } catch (failure) {
      assertActive();
      throw failure;
    }
  }
  function retained(raw) {
    assertActive();
    const draft = resourceHearingPrepared(structuredClone(raw));
    scope(draft.command);
    return draft;
  }
  return {
    dispose() {
      active = false;
    },
    async exact(overview) {
      assertActive();
      const selected = resourceHearingOverview(structuredClone(overview));
      scope(selected);
      return resourceHearingCreation(
        await call(`/${selected.id}/revisions/${selected.revision}`),
        selected,
      );
    },
    async prepare(raw, principal) {
      assertActive();
      resourceHearingBudget(raw);
      const command = resourceHearingCommand(structuredClone(raw));
      scope(command);
      const actor = { id: principal?.id, email: principal?.email };
      resourceHearingActor(actor);
      if (!['owner', 'litigator'].includes(principal?.role)) invalid();
      const draft = resourceHearingPrepared(
        await call('/prepare', { method: 'POST', data: command }),
      );
      if (!same(draft.command, command) || !same(draft.recorded_by, actor))
        invalid('La preparacion no corresponde al comando y la identidad enviados.');
      return draft;
    },
    async submit(raw) {
      const draft = retained(raw);
      const value = await call('/submit', {
        method: 'POST',
        data: {
          command: draft.command,
          expected_submission_digest: draft.submission_digest,
        },
      });
      return resourceHearingSubmission(value, draft);
    },
    async readSubmission(raw) {
      const draft = retained(raw);
      let value;
      try {
        value = await call(`/${draft.command.hearing_id}/revisions/1`);
      } catch (failure) {
        assertActive();
        if (failure.status === 404 && failure.code === 'resource_activity_not_found')
          return { state: 'unconfirmed' };
        throw failure;
      }
      return { state: 'confirmed', creation: resourceHearingSubmission(value, draft) };
    },
    async list(query = {}) {
      assertActive();
      object(query, ['limit', 'afterId'], []);
      const { limit = 10, afterId } = query;
      revision(limit, 20);
      if (afterId !== undefined) uuid(afterId);
      const parameters = new URLSearchParams({ limit });
      if (afterId !== undefined) parameters.set('after_id', afterId);
      const page = await call(`?${parameters}`);
      object(page, ['case_id', 'resource_id', 'items', 'has_more', 'next_after_id']);
      scope(page);
      if (
        !Array.isArray(page.items) ||
        page.items.length > limit ||
        typeof page.has_more !== 'boolean'
      )
        invalid();
      let previous = afterId;
      for (const value of page.items) {
        resourceHearingCreationScope(value, caseId, resourceId);
        const id = value.hearing.id;
        if (previous !== undefined && id <= previous) invalid();
        previous = id;
      }
      if (
        page.has_more
          ? page.items.length !== limit || page.next_after_id !== previous
          : page.next_after_id !== null
      )
        invalid();
      return page;
    },
  };
}
