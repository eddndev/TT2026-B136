import { test, expect } from '@playwright/test';
import { setupHearings } from './hearing-helpers.mjs';
import { login, navigate } from './helpers.mjs';

test('opens resolution capture with explicit declarations and no inferred event precision', async ({
  page,
}) => {
  await setupHearings(page);
  await page.route('**/api/v1/cases/*/resolutions?*', (route) =>
    route.fulfill({
      json: { resolutions: [], has_more: false, next_after_id: null },
    }),
  );
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await page.getByRole('link', { name: 'Resoluciones', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Resoluciones y notificaciones', exact: true }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Registrar resoluci\u00f3n', exact: true }).click();
  const editor = page.getByRole('region', { name: 'Formulario de resoluci\u00f3n', exact: true });
  await expect(
    editor.getByRole('combobox', { name: 'Clase de resoluci\u00f3n', exact: true }),
  ).toHaveValue('');
  await expect(editor.getByRole('combobox', { name: 'Emisor', exact: true })).toHaveValue('');
  await expect(
    editor.getByRole('combobox', { name: 'Precisi\u00f3n de emisi\u00f3n', exact: true }),
  ).toHaveValue('');
  await expect(editor.getByLabel('Resumen de la resoluci\u00f3n', { exact: true })).toHaveValue('');
  await expect(
    editor.getByRole('combobox', { name: 'Procedencia de la resoluci\u00f3n', exact: true }),
  ).toHaveValue('');
  await expect(
    editor.getByRole('button', { name: 'Preparar registro', exact: true }),
  ).toBeVisible();
});

test('records corrects and withdraws a resolution while keeping its exact original revision', async ({
  page,
}) => {
  const { setupFacts, openFacts, factEditor, factDetail, fillResolution, confirmFact } =
    await import('./procedural-facts-helpers.mjs');
  const state = await setupFacts(page);
  await openFacts(page);
  await page.getByRole('button', { name: 'Registrar resoluci\u00f3n', exact: true }).click();
  await fillResolution(page);
  await confirmFact(page);
  await expect(factEditor(page)).toHaveCount(0);
  await expect(factDetail(page)).toContainText('Declaracion de resolucion capturada');
  const first = state.submissions[0];
  expect(first.change.values.issued_at).toEqual({ precision: 'unknown' });
  expect(first.change.values.issuer).toEqual({
    kind: 'unknown',
    reason: 'El emisor no consta en la fuente',
  });
  expect(first.change).not.toHaveProperty('expected_case_revision');
  await factDetail(page)
    .getByRole('button', { name: 'Corregir resoluci\u00f3n', exact: true })
    .click();
  await factEditor(page)
    .getByLabel('Resumen de la resoluci\u00f3n', { exact: true })
    .fill('Texto precisado posteriormente');
  await factEditor(page).getByLabel('Motivo', { exact: true }).fill('Precision de la captura');
  await confirmFact(page);
  await expect(factDetail(page)).toContainText('Texto precisado posteriormente');
  await factDetail(page)
    .getByRole('button', { name: 'Retirar resoluci\u00f3n', exact: true })
    .click();
  await factEditor(page).getByLabel('Motivo', { exact: true }).fill('Registro duplicado');
  await confirmFact(page);
  await expect(
    factDetail(page).getByRole('button', { name: 'Corregir resoluci\u00f3n', exact: true }),
  ).toHaveCount(0);
  await factDetail(page)
    .getByRole('button', { name: 'Ver historial de resoluci\u00f3n', exact: true })
    .click();
  await page
    .getByRole('button', { name: 'Consultar resoluci\u00f3n revisi\u00f3n 1', exact: true })
    .click();
  await expect(factDetail(page)).toContainText('Declaracion de resolucion capturada');
  await expect(
    factDetail(page).getByRole('button', { name: 'Corregir resoluci\u00f3n', exact: true }),
  ).toHaveCount(0);
  expect(state.submissions.map((row) => row.change.action)).toEqual([
    'record',
    'correct',
    'withdraw',
  ]);
  expect(state.submissions.map((row) => row.change.expected_revision)).toEqual([0, 1, 2]);
  expect(state.submissions[2].change).not.toHaveProperty('values');
  expect(state.calls.at(-1).path).toContain(`/resolutions/${first.id}/revisions/1`);
});
