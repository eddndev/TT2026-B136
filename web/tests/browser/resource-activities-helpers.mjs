import { createHash } from 'node:crypto';
import { expect } from '@playwright/test';
import {
  setupProceduralResources,
  browserResource,
  openResources,
} from './procedural-resources-helpers.mjs';
import { installResourceActivityRoutes } from './resource-activities-routes.mjs';
import { caseId } from './helpers.mjs';
import { resourceActor, resourceCommandFixture } from '../fixtures/procedural-resource-unit.mjs';
import {
  activityPrepared,
  activityRecord,
  activityView,
  activityCheckedAt,
} from '../fixtures/resource-activity-unit.mjs';

export const clone = (value) => structuredClone(value);
const hash = (value) => createHash('sha256').update(JSON.stringify(value)).digest('hex');
export const activityPanel = (page) =>
  page.getByRole('region', { name: 'Actividades del recurso', exact: true });
export const activityEditor = (page) =>
  page.getByRole('region', { name: 'Formulario de actividad vinculada', exact: true });
export const activityDetail = (page) =>
  page.getByRole('region', { name: 'Detalle de actividad vinculada', exact: true });
const resourceRef = (row) => ({
  id: row.id,
  revision: row.revision,
  capture_digest: row.receipt.capture_digest,
});
function targets(kind) {
  const value = activityView(activityPrepared(kind));
  const rebound = JSON.parse(JSON.stringify(value).replaceAll(value.association.case_id, caseId));
  const rows = [rebound.association.sources.target.record, rebound.current_target.record];
  if (kind === 'hearing') {
    rows[0].values.venue = 'Sala historica uno';
    rows[1].values.venue = 'Sala actual dos';
  } else {
    rows[0].definition.title = 'Plazo con calculo historico';
    rows[1].definition.title = 'Plazo actual sin fecha operativa';
    rows[1].receipt.operation_id = 'e0000000-0000-4000-8000-000000000098';
    rows[1].receipt.submission_digest = '6'.repeat(64);
    rows[1].receipt.capture_digest = '5'.repeat(64);
  }
  return rows;
}
function addResourceActs(resources, original) {
  const command = resourceCommandFixture('record_act');
  command.resource_id = original.id;
  command.change.expected_revision = 1;
  command.change.values.kind = 'interposition';
  command.change.values.statement = 'Interposicion original declarada';
  command.change.values.evidence = [clone(original.values.resolution_evidence)];
  const act = resources.commit(resources.prepare(command));
  const correction = resourceCommandFixture('correct_act');
  correction.operation_id = 'c0000000-0000-4000-8000-000000000099';
  correction.resource_id = original.id;
  correction.change.expected_revision = 2;
  correction.change.values = clone(command.change.values);
  correction.change.values.statement = 'Interposicion corregida declarada';
  return [act, resources.commit(resources.prepare(correction))];
}
export async function setupResourceActivities(page, options = {}) {
  const original = browserResource();
  const resources = await setupProceduralResources(page, { ...options, resources: [original] });
  const [act, resource] = addResourceActs(resources, original);
  const hearing = targets('hearing'),
    deadline = targets('deadline');
  resources.facts.results.scheduling.records.set(hearing[0].id, hearing);
  const state = {
    resources,
    original,
    resource,
    act,
    hearing,
    deadline,
    records: new Map(),
    calls: [],
    submissions: [],
    targetCalls: [],
    handle: null,
  };
  state.prepare = (command) => {
    const base = state.records.get(command.association_id)?.at(-1);
    const change = command.change;
    const selection =
      change.action === 'link'
        ? { resource: clone(change.resource), act: clone(change.act), target: clone(change.target) }
        : clone(base.selection);
    const history = resources.records.get(resource.id);
    const sources =
      change.action === 'link'
        ? {
            resource: clone(history.find((row) => row.revision === selection.resource.revision)),
            act:
              selection.act === null
                ? null
                : clone(history.find((row) => row.revision === selection.act.resource_revision)),
            target: {
              kind: selection.target.kind,
              record: clone(
                state[selection.target.kind].find(
                  (row) =>
                    row.id === selection.target.id && row.revision === selection.target.revision,
                ),
              ),
            },
          }
        : clone(base.sources);
    const draft = {
      case_id: caseId,
      resource_id: resource.id,
      command: clone(command),
      result_revision: change.expected_revision + 1,
      selection,
      status: change.action === 'link' ? 'linked' : 'unlinked',
      sources,
      previous: base
        ? { revision: base.revision, capture_digest: base.receipt.capture_digest }
        : null,
      recorded_by: clone(resourceActor),
      observed_administration: clone(resource.recorded_administration),
      observed_resource_head: resourceRef(history.at(-1)),
      submission_digest: '',
    };
    draft.submission_digest = hash(draft);
    return draft;
  };
  state.commit = (draft) => {
    const row = activityRecord(draft);
    row.receipt.capture_digest = hash(row);
    state.records.set(row.id, [...(state.records.get(row.id) || []), row]);
    return row;
  };
  state.view = (association) => ({
    association: clone(association),
    checked_at: clone(activityCheckedAt),
    current_target: {
      kind: association.selection.target.kind,
      record: clone(state[association.selection.target.kind].at(-1)),
    },
  });
  state.seed = (kind = 'hearing') => {
    const target = state[kind][0];
    return state.commit(
      state.prepare({
        case_id: caseId,
        resource_id: resource.id,
        association_id: 'e0000000-0000-4000-8000-000000000001',
        operation_id: 'e0000000-0000-4000-8000-000000000002',
        expected_resource_revision: resource.revision,
        change: {
          action: 'link',
          expected_revision: 0,
          resource: resourceRef(original),
          act: null,
          target: {
            kind,
            id: target.id,
            revision: target.revision,
            ...(kind === 'hearing'
              ? { submission_digest: target.receipt.submission_digest }
              : { capture_digest: target.receipt.capture_digest }),
          },
        },
      }),
    );
  };
  await installResourceActivityRoutes(page, state, options);
  return state;
}
export async function openResourceActivities(page, state) {
  await openResources(page);
  await page
    .getByRole('button', { name: `Consultar recurso ${state.resource.values.title}`, exact: true })
    .click();
  await expect(activityPanel(page)).toBeVisible();
}
export async function selectTarget(page, state, kind, revision = 1) {
  const editor = activityEditor(page);
  await editor.getByRole('combobox', { name: 'Tipo de actividad', exact: true }).selectOption(kind);
  await editor
    .getByRole('combobox', { name: 'Actividad existente', exact: true })
    .selectOption(state[kind][0].id);
  await editor
    .getByRole('combobox', { name: 'Revisi\u00f3n de la actividad', exact: true })
    .selectOption(String(revision));
  await editor.getByRole('button', { name: 'Usar esta revisi\u00f3n', exact: true }).click();
}
