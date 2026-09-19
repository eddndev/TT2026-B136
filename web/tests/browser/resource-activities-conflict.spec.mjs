import { test, expect } from '@playwright/test';
import { resourceCommandFixture } from '../fixtures/procedural-resource-unit.mjs';
import {
  setupResourceActivities,
  openResourceActivities,
  selectTarget,
  activityPanel,
  activityEditor,
} from './resource-activities-helpers.mjs';

test('accepts a new resource head explicitly while retaining the selected historical sources', async ({
  page,
}) => {
  const state = await setupResourceActivities(page);
  await openResourceActivities(page, state);
  await activityPanel(page)
    .getByRole('button', { name: 'Vincular actividad', exact: true })
    .click();
  await selectTarget(page, state, 'hearing');
  const editor = activityEditor(page);
  await editor.getByRole('button', { name: 'Elegir acto del recurso', exact: true }).click();
  await editor.getByRole('combobox', { name: 'Acto del recurso', exact: true }).selectOption('2');
  await editor.getByRole('button', { name: 'Usar este acto', exact: true }).click();
  const correction = resourceCommandFixture('correct');
  correction.operation_id = 'c0000000-0000-4000-8000-000000000098';
  correction.resource_id = state.resource.id;
  correction.change.values = structuredClone(state.resource.values);
  correction.change.values.title = 'Titulo actualizado concurrentemente';
  const updated = state.resources.commit(state.resources.prepare(correction));
  expect(updated.revision).toBe(4);
  await editor.getByRole('button', { name: 'Preparar v\u00ednculo', exact: true }).click();
  await expect(
    editor.getByRole('heading', { name: 'El registro cambi\u00f3', exact: true }),
  ).toBeVisible();
  expect(state.submissions).toHaveLength(0);
  expect(state.calls.filter((row) => row.method === 'POST')).toHaveLength(1);
  await editor.getByRole('button', { name: 'Comparar con registro actual', exact: true }).click();
  await expect(editor).toContainText(updated.values.title);
  expect(state.calls.filter((row) => row.method === 'POST')).toHaveLength(1);
  await editor
    .getByRole('button', { name: 'Usar base actual y conservar borrador', exact: true })
    .click();
  await editor.getByRole('button', { name: 'Preparar v\u00ednculo', exact: true }).click();
  const prepared = state.calls.findLast((row) => row.path.endsWith('/prepare')).body;
  expect(prepared.expected_resource_revision).toBe(4);
  expect(prepared.change.resource).toEqual({
    id: state.resource.id,
    revision: 3,
    capture_digest: state.resource.receipt.capture_digest,
  });
  expect(prepared.change.act).toEqual({
    id: state.act.act.id,
    revision: 1,
    resource_revision: 2,
    capture_digest: state.act.receipt.capture_digest,
  });
  expect(prepared.change.target).toEqual({
    kind: 'hearing',
    id: state.hearing[0].id,
    revision: 1,
    submission_digest: state.hearing[0].receipt.submission_digest,
  });
  await editor.getByRole('button', { name: 'Confirmar v\u00ednculo', exact: true }).click();
  await expect(editor).toHaveCount(0);
  expect(state.submissions).toHaveLength(1);
  expect(state.submissions[0]).toEqual(prepared);
});
