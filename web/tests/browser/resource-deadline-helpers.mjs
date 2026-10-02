import { createHash } from 'node:crypto';
import { expect } from '@playwright/test';
import {
  setupResourceActivities,
  openResourceActivities,
  activityPanel,
  clone,
} from './resource-activities-helpers.mjs';
import { setupDeadlines, fillDeadline, editor, caseId } from './deadline-editor-helpers.mjs';
import { resourceDeadlineResult } from '../fixtures/resource-deadline-unit.mjs';
export { editor, fillDeadline, activityPanel };
const hash = (v) => createHash('sha256').update(JSON.stringify(v)).digest('hex');
const ref = (row) => ({
  id: row.id,
  revision: row.revision,
  capture_digest: row.receipt.capture_digest,
});
export async function setupResourceDeadline(page, options = {}) {
  const activities = await setupResourceActivities(page, options);
  const deadlines = await setupDeadlines(page, {
    ...options,
    factState: activities.resources.facts,
  });
  const state = {
    activities,
    deadlines,
    calls: [],
    submissions: [],
    committed: new Map(),
    handle: null,
  };
  state.prepare = (command) => {
    const existing = state.committed.get(command.deadline.operation_id);
    if (existing) return clone(existing.draft);
    const deadline = deadlines.prepare(command.deadline);
    const rows = activities.resources.records.get(command.resource_id);
    const association = {
      command: {
        case_id: caseId,
        resource_id: command.resource_id,
        association_id: command.association_id,
        operation_id: command.deadline.operation_id,
        expected_resource_revision: command.expected_resource_revision,
        change: {
          action: 'link',
          expected_revision: 0,
          resource: clone(command.resource),
          act: clone(command.act),
          target: {
            kind: 'deadline',
            id: command.deadline.deadline_id,
            revision: 1,
            capture_digest: deadline.capture_digest,
          },
        },
      },
      resource: clone(rows.find((r) => r.revision === command.resource.revision)),
      act: command.act
        ? clone(rows.find((r) => r.revision === command.act.resource_revision))
        : null,
      recorded_by: { id: deadline.actor_id, email: deadline.author.email },
      observed_administration: clone(deadline.calculation.material.administration),
      observed_resource_head: ref(rows.at(-1)),
      submission_digest: '8'.repeat(64),
    };
    const result = { command: clone(command), deadline, association, submission_digest: '' };
    result.submission_digest = hash(result);
    return result;
  };
  state.commit = (draft) => {
    const existing = state.committed.get(draft.command.deadline.operation_id);
    if (existing) return clone(existing.result);
    const result = resourceDeadlineResult(draft);
    deadlines.records.set(result.deadline.id, [result.deadline]);
    activities.records.set(result.association.id, [result.association]);
    activities.deadline.push(clone(result.deadline));
    state.committed.set(draft.command.deadline.operation_id, {
      draft: clone(draft),
      result: clone(result),
    });
    return result;
  };
  activities.handle = async (route, call) => {
    if (!call.path.includes('/activities/deadlines/')) return false;
    state.calls.push(call);
    if (state.handle && (await state.handle(route, call))) return true;
    const preparing = call.path.endsWith('/prepare');
    const command = preparing ? call.body : call.body.command;
    if (
      command.expected_resource_revision !==
      activities.resources.records.get(command.resource_id).at(-1).revision
    ) {
      await route.fulfill({
        status: 409,
        json: { error: { code: 'resource_activity_resource_revision_conflict' } },
      });
      return true;
    }
    const draft = state.prepare(command);
    if (preparing) await route.fulfill({ json: draft });
    else {
      state.submissions.push(clone(call.body));
      await route.fulfill({ status: 201, json: state.commit(draft) });
    }
    return true;
  };
  return state;
}
export async function openResourceDeadline(page, state) {
  await openResourceActivities(page, state.activities);
  await activityPanel(page).getByRole('button', { name: 'Crear plazo', exact: true }).click();
  await fillDeadline(page, 'Plazo contextual');
  await editor(page)
    .getByRole('combobox', { name: 'Cuando cambie el perfil', exact: true })
    .selectOption('follow');
}
export async function prepareResourceDeadline(page) {
  await editor(page).getByRole('button', { name: 'Preparar plazo y vinculo', exact: true }).click();
  await expect(
    editor(page).getByRole('button', { name: 'Confirmar plazo y vinculo', exact: true }),
  ).toBeDisabled();
  await editor(page)
    .getByRole('checkbox', {
      name: 'Reconozco el resultado y las capturas seleccionadas',
      exact: true,
    })
    .check();
}
export async function submitResourceDeadline(page) {
  await editor(page)
    .getByRole('button', { name: 'Confirmar plazo y vinculo', exact: true })
    .click();
}
