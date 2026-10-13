import { expect } from '@playwright/test';
import {
  setupResourceActivities,
  openResourceActivities,
  activityPanel,
  clone,
} from './resource-activities-helpers.mjs';
import { resourceDetail } from './procedural-resources-helpers.mjs';
import { caseId } from './helpers.mjs';
import { participant } from './hearing-helpers.mjs';
import { failFact } from './procedural-facts-helpers.mjs';
import { resourceActor } from '../fixtures/procedural-resource-unit.mjs';
import {
  resourceHearingCommand,
  resourceHearingDraft,
  resourceHearingResult,
} from '../fixtures/resource-hearing-unit.mjs';

export { clone, participant, activityPanel };
export const hearingPanel = (page) =>
  page.getByRole('region', { name: 'Audiencias del recurso', exact: true });
export const hearingEditor = (page) =>
  page.getByRole('region', { name: 'Formulario de audiencia de recurso', exact: true });
export const hearingDetail = (page) =>
  page.getByRole('region', { name: 'Detalle de audiencia de recurso', exact: true });
export const reference = (row) => ({
  id: row.id,
  revision: row.revision,
  capture_digest: row.receipt.capture_digest,
});

export async function setupResourceHearing(page, options = {}) {
  const activities = await setupResourceActivities(page, options);
  const resources = activities.resources;
  const history = resources.records.get(activities.resource.id);
  for (const row of history) row.recorded_administration.changed_by = clone(resourceActor);
  const prepareResource = resources.prepare;
  resources.prepare = (command) => {
    const draft = prepareResource(command);
    draft.observed_administration.changed_by = clone(resourceActor);
    return draft;
  };
  for (const row of history.filter((value) => value.act)) {
    Object.assign(row.act.values.evidence[0], {
      document_id: 'd0000000-0000-4000-8000-000000000088',
      digest: '8'.repeat(64),
    });
    Object.assign(row.act.supports[0], {
      document_id: 'd0000000-0000-4000-8000-000000000088',
      digest: '8'.repeat(64),
      name: 'senalamiento-del-acto.pdf',
    });
  }
  const base = `/api/v1/cases/${caseId}/procedural-resources/${activities.resource.id}/activities/resource-hearings`;
  const state = {
    activities,
    resources,
    base,
    calls: [],
    submissions: [],
    committed: new Map(),
    handle: null,
  };
  activities.resource_hearing = [];
  state.prepare = (command) => {
    const replay = state.committed.get(command.operation_id);
    if (replay) return clone(replay.draft);
    const rows = resources.records.get(command.resource_id);
    const resource = rows.find((row) => row.revision === command.resource.revision);
    const act = command.act
      ? rows.find((row) => row.revision === command.act.resource_revision)
      : null;
    const support = [...resource.sources.supports, ...(act?.act.supports || [])].find(
      (row) =>
        row.document_id === command.values.scheduling_basis.support.document_id &&
        row.version === command.values.scheduling_basis.support.version,
    );
    const participants = command.values.participants.map((selected) => {
      const row = resources.facts.results.directory
        .get(selected.participant_id)
        .find((value) => value.revision === selected.revision);
      return {
        case_id: caseId,
        id: row.id,
        revision: row.revision,
        values_digest: row.values_digest,
        directory_status: row.directory_status,
        subject: row.subject
          ? {
              id: row.subject.id,
              revision: row.subject.revision,
              values_digest: row.subject.values_digest,
            }
          : null,
        display_name: row.display_name,
        procedural_role: row.procedural_role,
        organization: row.organization || null,
        kind: row.profile?.kind || null,
      };
    });
    return resourceHearingDraft(command, {
      resource,
      act,
      head: reference(rows.at(-1)),
      participants,
      support,
      actor: resourceActor,
      administration: resource.recorded_administration,
    });
  };
  state.commit = (draft) => {
    const saved = state.committed.get(draft.command.operation_id);
    if (saved) return clone(saved.creation);
    const creation = resourceHearingResult(draft);
    state.committed.set(draft.command.operation_id, { draft: clone(draft), creation });
    activities.records.set(creation.association.id, [clone(creation.association)]);
    activities.resource_hearing.push(clone(creation.hearing));
    return clone(creation);
  };
  state.seed = () => {
    const command = resourceHearingCommand();
    Object.assign(command, {
      case_id: caseId,
      resource_id: activities.resource.id,
      resource: reference(activities.original),
      act: null,
      expected_resource_revision: history.at(-1).revision,
    });
    command.values.participants = [];
    const { document_id, version, digest } = activities.original.sources.supports[0];
    command.values.scheduling_basis.support = { document_id, version, digest };
    return state.commit(state.prepare(command));
  };
  activities.handle = async (route, call) => {
    if (!call.path.startsWith(base)) return false;
    state.calls.push(call);
    if (state.handle && (await state.handle(route, call))) return true;
    if (call.method === 'GET') {
      const items = [...state.committed.values()].map((row) => clone(row.creation));
      if (call.path === base) {
        await route.fulfill({
          json: {
            case_id: caseId,
            resource_id: activities.resource.id,
            items,
            has_more: false,
            next_after_id: null,
          },
        });
      } else {
        const found = items.find((row) => `${base}/${row.hearing.id}/revisions/1` === call.path);
        if (found) await route.fulfill({ json: found });
        else await failFact(route, 'resource_activity_not_found', 404);
      }
      return true;
    }
    if (options.role === 'paralegal') await failFact(route, 'permission_denied', 403);
    else if (resources.facts.results.scheduling.context.administrative_status === 'closed')
      await failFact(route, 'case_closed');
    else {
      const preparing = call.path === `${base}/prepare`;
      const command = preparing ? call.body : call.body.command;
      const head = resources.records.get(command.resource_id).at(-1);
      if (head.revision !== command.expected_resource_revision)
        await failFact(route, 'resource_activity_resource_revision_conflict');
      else if (head.status !== 'active')
        await failFact(route, 'resource_activity_resource_archived');
      else {
        const draft = state.prepare(command);
        if (preparing) await route.fulfill({ json: draft });
        else if (draft.submission_digest !== call.body.expected_submission_digest)
          await failFact(route, 'resource_activity_submission_mismatch');
        else {
          state.submissions.push(clone(call.body));
          await route.fulfill({ status: 201, json: state.commit(draft) });
        }
      }
    }
    return true;
  };
  return state;
}

export async function openHearingForm(page, state, historical = true) {
  await openResourceActivities(page, state.activities);
  if (historical) {
    await resourceDetail(page)
      .getByRole('button', { name: 'Ver historial de recurso', exact: true })
      .click();
    await page.getByRole('button', { name: 'Consultar recurso revision 1', exact: true }).click();
  }
  await activityPanel(page)
    .getByRole('button', { name: 'Crear audiencia de recurso', exact: true })
    .click();
  await expect(hearingEditor(page)).toBeVisible();
}
export async function fillHearingForm(page) {
  const editor = hearingEditor(page);
  await editor.getByLabel('Fecha', { exact: true }).fill('2026-10-10');
  await editor.getByLabel('Hora', { exact: true }).fill('09:02:03');
  await editor.getByLabel('Desfase UTC', { exact: true }).fill('-06:00');
  await editor.getByLabel('Sede o enlace', { exact: true }).fill('Sala propia del recurso');
  await editor.getByLabel('Nota', { exact: true }).fill('Programacion exacta conservada');
  await editor
    .getByLabel('Base de senalamiento', { exact: true })
    .fill('Senalamiento declarado con evidencia admitida');
}
export async function selectActSupportAndParticipant(page, state) {
  const editor = hearingEditor(page);
  await editor.getByRole('button', { name: 'Elegir acto del recurso', exact: true }).click();
  await editor.getByRole('combobox', { name: 'Acto del recurso', exact: true }).selectOption('2');
  await editor.getByRole('button', { name: 'Usar este acto', exact: true }).click();
  const support = state.activities.act.act.supports[0];
  await editor
    .getByRole('combobox', { name: 'Soporte admitido del senalamiento', exact: true })
    .selectOption({ label: `${support.name} / Version ${support.version}` });
  await editor.getByRole('button', { name: 'Elegir participante', exact: true }).click();
  await editor
    .getByRole('button', { name: `Consultar ficha: ${participant.display_name}`, exact: true })
    .click();
  await editor.getByRole('button', { name: 'Vincular esta revisi\u00f3n', exact: true }).click();
}
export async function prepareHearing(page) {
  const editor = hearingEditor(page);
  await editor.getByRole('button', { name: 'Preparar audiencia y vinculo', exact: true }).click();
  await expect(
    editor.getByRole('region', { name: 'Revision de audiencia y vinculo', exact: true }),
  ).toBeVisible();
  const confirm = editor.getByRole('button', {
    name: 'Confirmar audiencia y vinculo',
    exact: true,
  });
  await expect(confirm).toBeDisabled();
  await editor
    .getByRole('checkbox', {
      name: 'Reconozco la programacion y las capturas seleccionadas',
      exact: true,
    })
    .check();
  await expect(confirm).toBeEnabled();
}
export const submitHearing = (page) =>
  hearingEditor(page)
    .getByRole('button', { name: 'Confirmar audiencia y vinculo', exact: true })
    .click();
