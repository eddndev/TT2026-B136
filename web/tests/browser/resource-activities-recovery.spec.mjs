import { test, expect } from '@playwright/test';
import { resourceCommandFixture } from '../fixtures/procedural-resource-unit.mjs';
import {
  setupResourceActivities,
  openResourceActivities,
  selectTarget,
  activityPanel,
  activityEditor,
  activityDetail,
} from './resource-activities-helpers.mjs';

test('a missing uncommitted link stays uncertain until an explicit retry of the exact envelope', async ({
  page,
}) => {
  const state = await setupResourceActivities(page);
  let firstEnvelope;
  state.handle = async (route, call) => {
    if (call.method !== 'POST' || !call.path.endsWith('/activities') || firstEnvelope) return false;
    firstEnvelope = structuredClone(call.body);
    await route.abort('failed');
    return true;
  };
  await openResourceActivities(page, state);
  await activityPanel(page)
    .getByRole('button', { name: 'Vincular actividad', exact: true })
    .click();
  await selectTarget(page, state, 'hearing');
  const editor = activityEditor(page);
  await editor.getByRole('button', { name: 'Preparar v\u00ednculo', exact: true }).click();
  await editor.getByRole('button', { name: 'Confirmar v\u00ednculo', exact: true }).click();
  await expect(
    editor.getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  expect(state.records.size).toBe(0);
  expect(state.submissions).toHaveLength(0);
  const resultPath = `/activities/${firstEnvelope.command.association_id}/revisions/1`;
  const pending = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname.endsWith(resultPath) &&
      response.request().method() === 'GET',
  );
  await editor.getByRole('button', { name: 'Consultar resultado', exact: true }).click();
  expect((await pending).status()).toBe(404);
  await expect(editor).toContainText('El resultado sigue incierto');
  await expect(
    editor.getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  const confirmations = () =>
    state.calls.filter((row) => row.method === 'POST' && row.path.endsWith('/activities'));
  const preparations = () =>
    state.calls.filter((row) => row.method === 'POST' && row.path.endsWith('/prepare'));
  expect(confirmations()).toHaveLength(1);
  expect(preparations()).toHaveLength(1);
  expect(state.records.size).toBe(0);
  await editor.getByRole('button', { name: 'Reintentar envio exacto', exact: true }).click();
  await expect(editor).toHaveCount(0);
  await expect(activityDetail(page)).toBeVisible();
  expect(preparations()).toHaveLength(1);
  expect(confirmations()).toHaveLength(2);
  expect(confirmations()[1].body).toEqual(firstEnvelope);
  expect(confirmations()[1].body.command.operation_id).toBe(firstEnvelope.command.operation_id);
  expect(confirmations()[1].body.expected_submission_digest).toBe(
    firstEnvelope.expected_submission_digest,
  );
  expect(state.submissions).toEqual([firstEnvelope.command]);
  const rows = state.records.get(firstEnvelope.command.association_id);
  expect(rows).toHaveLength(1);
  expect(rows[0].receipt.operation_id).toBe(firstEnvelope.command.operation_id);
  expect(rows[0].receipt.submission_digest).toBe(firstEnvelope.expected_submission_digest);
});

test('a confirmation conflict reloads the target selector when the preserved draft becomes editable', async ({
  page,
}) => {
  const state = await setupResourceActivities(page);
  const alternative = structuredClone(state.hearing[0]);
  alternative.id = 'f0000000-0000-4000-8000-000000000099';
  alternative.values.venue = 'Otra audiencia disponible tras comparar';
  alternative.receipt.operation_id = 'f0000000-0000-4000-8000-000000000098';
  alternative.receipt.submission_digest = '5'.repeat(64);
  state.resources.facts.results.scheduling.records.set(state.hearing[0].id, [...state.hearing]);
  state.hearing.push(alternative);
  state.resources.facts.results.scheduling.records.set(alternative.id, [alternative]);
  await openResourceActivities(page, state);
  await activityPanel(page)
    .getByRole('button', { name: 'Vincular actividad', exact: true })
    .click();
  await selectTarget(page, state, 'hearing');
  const editor = activityEditor(page);
  await editor.getByRole('button', { name: 'Elegir acto del recurso', exact: true }).click();
  await editor.getByRole('combobox', { name: 'Acto del recurso', exact: true }).selectOption('2');
  await editor.getByRole('button', { name: 'Usar este acto', exact: true }).click();
  await editor.getByRole('button', { name: 'Preparar v\u00ednculo', exact: true }).click();
  await expect(
    editor.getByRole('button', { name: 'Confirmar v\u00ednculo', exact: true }),
  ).toBeVisible();
  const firstCommand = structuredClone(
    state.calls.findLast((row) => row.path.endsWith('/prepare')).body,
  );
  const correction = resourceCommandFixture('correct');
  correction.operation_id = 'c0000000-0000-4000-8000-000000000097';
  correction.resource_id = state.resource.id;
  correction.change.values = structuredClone(state.resource.values);
  correction.change.values.title = 'Cabeza posterior a la preparacion';
  expect(state.resources.commit(state.resources.prepare(correction)).revision).toBe(4);
  const listCalls = () =>
    state.resources.facts.results.scheduling.calls.filter(
      (row) => row.method === 'GET' && row.path.endsWith('/hearings'),
    );
  const previousLists = listCalls().length;
  await editor.getByRole('button', { name: 'Confirmar v\u00ednculo', exact: true }).click();
  await expect(
    editor.getByRole('heading', { name: 'El registro cambi\u00f3', exact: true }),
  ).toBeVisible();
  expect(state.records.size).toBe(0);
  expect(listCalls()).toHaveLength(previousLists);
  await editor.getByRole('button', { name: 'Comparar con registro actual', exact: true }).click();
  await expect(editor).toContainText('Cabeza posterior a la preparacion');
  await editor
    .getByRole('button', { name: 'Usar base actual y conservar borrador', exact: true })
    .click();
  const existing = editor.getByRole('combobox', { name: 'Actividad existente', exact: true });
  await expect(existing.locator(`option[value="${alternative.id}"]`)).toHaveCount(1);
  expect(listCalls()).toHaveLength(previousLists + 1);
  await existing.selectOption(alternative.id);
  await editor
    .getByRole('combobox', { name: 'Revisi\u00f3n de la actividad', exact: true })
    .selectOption('1');
  await editor.getByRole('button', { name: 'Usar esta revisi\u00f3n', exact: true }).click();
  await expect(editor).toContainText(alternative.values.venue);
  await editor.getByRole('button', { name: 'Preparar v\u00ednculo', exact: true }).click();
  await expect(
    editor.getByRole('button', { name: 'Confirmar v\u00ednculo', exact: true }),
  ).toBeVisible();
  const nextCommand = state.calls.findLast((row) => row.path.endsWith('/prepare')).body;
  expect(nextCommand.expected_resource_revision).toBe(4);
  expect(nextCommand.change.resource).toEqual(firstCommand.change.resource);
  expect(nextCommand.change.act).toEqual(firstCommand.change.act);
  expect(nextCommand.change.target).toEqual({
    kind: 'hearing',
    id: alternative.id,
    revision: 1,
    submission_digest: alternative.receipt.submission_digest,
  });
  expect(state.submissions).toHaveLength(0);
});
