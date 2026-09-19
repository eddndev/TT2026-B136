import { expect } from '@playwright/test';
import { randomUUID } from 'node:crypto';
import { fixture } from './helpers.mjs';
import { openResource, responseTo } from './procedural-resources-helpers.mjs';
export { accountAction } from './hearing-result-helpers.mjs';
export const accounts = fixture.resourceActivities;
export const panel = (page) =>
  page.getByRole('region', { name: 'Actividades del recurso', exact: true });
export const editor = (page) =>
  page.getByRole('region', { name: 'Formulario de actividad vinculada', exact: true });
export const detail = (page) =>
  page.getByRole('region', { name: 'Detalle de actividad vinculada', exact: true });
export const route = (scenario) =>
  `/cases/${scenario.case.id}/procedural-resources/${scenario.resource.id}/activities`;
export async function openActivities(page, scenario) {
  expect(await openResource(page, scenario)).toEqual(scenario.resource);
  await expect(
    panel(page).getByRole('region', { name: 'Vinculos registrados', exact: true }),
  ).toHaveAttribute('aria-busy', 'false');
}
export async function chooseTarget(page, scenario, kind) {
  const form = editor(page),
    row = scenario[kind + 'Initial'];
  await form.getByRole('combobox', { name: 'Tipo de actividad', exact: true }).selectOption(kind);
  await form
    .getByRole('combobox', { name: 'Actividad existente', exact: true })
    .selectOption(row.id);
  await form
    .getByRole('combobox', { name: 'Revisi\u00f3n de la actividad', exact: true })
    .selectOption('1');
  await form.getByRole('button', { name: 'Usar esta revisi\u00f3n', exact: true }).click();
}
export async function prepare(page, scenario, unlink = false) {
  const pending = responseTo(page, '/api/v1' + route(scenario) + '/prepare', 'POST');
  await editor(page)
    .getByRole('button', {
      name: unlink ? 'Preparar desvinculaci\u00f3n' : 'Preparar v\u00ednculo',
      exact: true,
    })
    .click();
  const response = await pending;
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toBe('no-store');
  return response.json();
}
export async function confirm(page, scenario, draft) {
  const unlink = draft.command.change.action === 'unlink';
  const suffix = unlink ? `/${draft.command.association_id}/unlink` : '';
  const pending = responseTo(page, '/api/v1' + route(scenario) + suffix, 'POST');
  await editor(page)
    .getByRole('button', {
      name: unlink ? 'Confirmar desvinculaci\u00f3n' : 'Confirmar v\u00ednculo',
      exact: true,
    })
    .click();
  const response = await pending;
  expect(response.status()).toBe(201);
  expect(response.headers()['cache-control']).toBe('no-store');
  const result = await response.json();
  expect(result.receipt.submission_digest).toBe(draft.submission_digest);
  await expect(editor(page)).toHaveCount(0);
  return result;
}
export async function exactAssociation(page, scenario, id, revision) {
  await detail(page)
    .getByRole('button', { name: 'Ver historial de v\u00ednculo', exact: true })
    .click();
  const pending = responseTo(page, '/api/v1' + route(scenario) + `/${id}/revisions/${revision}`);
  await page
    .getByRole('button', { name: `Consultar v\u00ednculo revisi\u00f3n ${revision}`, exact: true })
    .click();
  const response = await pending;
  expect(response.status()).toBe(200);
  return response.json();
}
export const resourceReference = (record) => ({
  id: record.id,
  revision: record.revision,
  capture_digest: record.receipt.capture_digest,
});
export function linkCommand(scenario) {
  return {
    case_id: scenario.case.id,
    resource_id: scenario.resource.id,
    association_id: randomUUID(),
    operation_id: randomUUID(),
    expected_resource_revision: 3,
    change: {
      action: 'link',
      expected_revision: 0,
      resource: resourceReference(scenario.resource),
      act: null,
      target: {
        kind: 'hearing',
        id: scenario.hearingInitial.id,
        revision: 1,
        submission_digest: scenario.hearingInitial.receipt.submission_digest,
      },
    },
  };
}
export async function alertSnapshot(call, scenario) {
  const rows = [];
  let cursor;
  do {
    const query = new URLSearchParams({ limit: 100, read: 'all', state: 'all' });
    if (cursor) query.set('cursor', cursor);
    const page = await call('GET', `/alerts?${query}`);
    rows.push(...page.alerts.filter((row) => row.subject.case_id === scenario.case.id));
    cursor = page.next_cursor;
    if (rows.length > 100) throw new Error('Unexpected alert volume in isolated case');
  } while (cursor);
  expect(new Set(rows.map((row) => row.id)).size).toBe(rows.length);
  return rows
    .map(({ id, subject, origin, kind, state }) => ({ id, subject, origin, kind, state }))
    .sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
}
export async function readyAlerts(call, scenario) {
  let rows;
  await expect
    .poll(
      async () => {
        rows = await alertSnapshot(call, scenario);
        return rows.filter(
          (row) =>
            row.subject.id === scenario.hearing.id &&
            row.origin.revision === 2 &&
            row.state.kind === 'active' &&
            row.kind.kind === 'upcoming' &&
            row.kind.lead_hours === 48,
        ).length;
      },
      { timeout: 60000, intervals: [250, 500, 1000] },
    )
    .toBe(1);
  return rows;
}
export async function unchangedTargets(call, scenario) {
  const base = `/cases/${scenario.case.id}`;
  expect(await call('GET', `${base}/hearings/${scenario.hearing.id}`)).toEqual(scenario.hearing);
  expect(await call('GET', `${base}/hearings/${scenario.hearing.id}/revisions/1`)).toEqual(
    scenario.hearingInitial,
  );
  const deadline = await call('GET', `${base}/deadlines/${scenario.deadline.id}`);
  const { operational: actualOperational, ...actual } = deadline;
  const { operational: initialOperational, ...initial } = scenario.deadline;
  expect(actual).toEqual(initial);
  expect(actualOperational.due_at).toEqual(initialOperational.due_at);
  expect(await call('GET', `${base}/procedural-resources/${scenario.resource.id}`)).toEqual(
    scenario.resource,
  );
  expect(
    (await call('GET', `${base}/hearings/${scenario.hearing.id}/history?limit=20`)).revisions,
  ).toHaveLength(2);
  expect(
    (await call('GET', `${base}/deadlines/${scenario.deadline.id}/history?limit=20`)).revisions,
  ).toHaveLength(2);
}
