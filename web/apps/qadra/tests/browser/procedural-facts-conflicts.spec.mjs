import { test, expect } from '@playwright/test';
import {
  setupFacts,
  openFacts,
  factEditor,
  factDetail,
  fillResolution,
  confirmFact,
  failFact,
} from './procedural-facts-helpers.mjs';
import { factRecord, factCommand } from '../fixtures/procedural-facts.mjs';

test('revision conflict retains the draft until the operator reads and explicitly adopts the new base', async ({
  page,
}) => {
  const row = factRecord(),
    state = await setupFacts(page, { facts: [row] });
  await openFacts(page);
  await page
    .getByRole('button', { name: `Consultar resoluci\u00f3n ${row.id}`, exact: true })
    .click();
  await factDetail(page)
    .getByRole('button', { name: 'Corregir resoluci\u00f3n', exact: true })
    .click();
  const editor = factEditor(page);
  await editor
    .getByLabel('Resumen de la resoluci\u00f3n', { exact: true })
    .fill('Mi borrador conservado');
  await editor.getByLabel('Motivo', { exact: true }).fill('Mi precision');
  const other = factCommand('resolution', 'correct', 1);
  other.change.values.summary = 'Cambio concurrente del expediente';
  state.commit(state.prepare(other));
  await editor.getByRole('button', { name: 'Preparar registro', exact: true }).click();
  await expect(editor.getByLabel('Resumen de la resoluci\u00f3n', { exact: true })).toHaveValue(
    'Mi borrador conservado',
  );
  await expect(
    editor.getByRole('button', { name: 'Preparar registro', exact: true }),
  ).toBeDisabled();
  expect(state.submissions).toHaveLength(0);
  await editor.getByRole('button', { name: 'Consultar base actual', exact: true }).click();
  await expect(editor).toContainText('Cambio concurrente del expediente');
  await expect(editor.getByLabel('Resumen de la resoluci\u00f3n', { exact: true })).toHaveValue(
    'Mi borrador conservado',
  );
  await editor
    .getByRole('button', { name: 'Usar esta base y conservar borrador', exact: true })
    .click();
  await confirmFact(page);
  await expect(editor).toHaveCount(0);
  expect(state.submissions[0].change.expected_revision).toBe(2);
  expect(state.submissions[0].change.values.summary).toBe('Mi borrador conservado');
});

test('closing a case before preparation preserves the draft and disables confirmation', async ({
  page,
}) => {
  const state = await setupFacts(page);
  await openFacts(page);
  await page.getByRole('button', { name: 'Registrar resoluci\u00f3n', exact: true }).click();
  await fillResolution(page);
  state.results.scheduling.context.administrative_status = 'closed';
  state.results.scheduling.admin.administration.administrative_status = 'closed';
  await factEditor(page).getByRole('button', { name: 'Preparar registro', exact: true }).click();
  await expect(
    factEditor(page).getByLabel('Resumen de la resoluci\u00f3n', { exact: true }),
  ).toHaveValue('Declaracion de resolucion capturada');
  await expect(
    factEditor(page).getByRole('button', { name: 'Preparar registro', exact: true }),
  ).toBeDisabled();
  expect(state.submissions).toHaveLength(0);
});

test('support admission failure preserves capture and shows actionable guidance without submitting', async ({
  page,
}) => {
  const state = await setupFacts(page);
  state.handle = async (route, call) => {
    if (!call.path.endsWith('/prepare')) return;
    await failFact(route, 'procedural_fact_support_digest_mismatch', 422);
    return true;
  };
  await openFacts(page);
  await page.getByRole('button', { name: 'Registrar resoluci\u00f3n', exact: true }).click();
  await fillResolution(page);
  await factEditor(page).getByRole('button', { name: 'Preparar registro', exact: true }).click();
  await expect(factEditor(page).getByRole('alert')).toBeVisible();
  await expect(factEditor(page).getByRole('alert')).not.toContainText('procedural_fact_');
  await expect(
    factEditor(page).getByLabel('Resumen de la resoluci\u00f3n', { exact: true }),
  ).toHaveValue('Declaracion de resolucion capturada');
  expect(state.submissions).toHaveLength(0);
});
