import { test, expect } from '@playwright/test';
import {
  setupStages,
  openStages,
  chooseSupport,
  fillDate,
  initial,
  caseId,
  document,
  entry,
} from './stage-helpers.mjs';
import { login, navigate } from './helpers.mjs';
const command = {
  expected_revision: 1,
  target: 'intermediate',
  accusation_declared_at: {
    precision: 'date',
    date: '2026-09-01',
    offset: '-06:00',
  },
  accusation: { document_id: document.id, version: 1, digest: document.digest },
};
async function prepare(page) {
  await openStages(page);
  await page.getByRole('button', { name: 'Registrar paso a Intermedia' }).click();
  await fillDate(page, 'Fecha de la acusaci\u00f3n');
  await chooseSupport(page);
  await page.getByLabel('Nota (opcional)', { exact: true }).fill('Borrador preservado');
  await page.getByRole('button', { name: 'Revisar registro' }).click();
}
test('conflict keeps draft, compares current explicitly and rejects an obsolete edge', async ({
  page,
}) => {
  const { state } = await setupStages(page);
  await prepare(page);
  await page.route(`**/api/v1/cases/${caseId}/stage/transitions`, async (route) => {
    state.current = entry(command);
    state.history = [state.current, initial];
    await route.fulfill({ status: 409, json: { error: { code: 'case_stage_conflict' } } });
  });
  await page.getByRole('button', { name: 'Registrar transici\u00f3n', exact: true }).click();
  await expect(page.getByLabel('Nota (opcional)')).toHaveValue('Borrador preservado');
  await expect(page.getByRole('button', { name: 'Revisar registro' })).toBeDisabled();
  await page.getByRole('button', { name: 'Consultar etapa e historial' }).click();
  await expect(page.locator('.stage-reconciliation')).toContainText(
    'El avance del borrador ya no corresponde',
  );
  await expect(
    page.getByRole('button', { name: 'Usar etapa consultada y revisar borrador' }),
  ).toHaveCount(0);
});
test('uncertain mutation reconciles head then history and never resubmits automatically', async ({
  page,
}) => {
  const { state } = await setupStages(page);
  await prepare(page);
  let posts = 0,
    releaseHead,
    releaseHistory,
    headDone = false,
    historyStarted = false;
  await page.route(`**/api/v1/cases/${caseId}/stage/transitions`, async (route) => {
    posts++;
    await route.abort('failed');
  });
  await page.getByRole('button', { name: 'Registrar transici\u00f3n', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('No se pudo confirmar');
  await page.route(`**/api/v1/cases/${caseId}/stage`, async (route) => {
    await new Promise((resolve) => (releaseHead = resolve));
    headDone = true;
    await route.fulfill({ json: { case_id: caseId, current: initial } });
  });
  await page.route(`**/api/v1/cases/${caseId}/stage/history?*`, async (route) => {
    historyStarted = true;
    expect(headDone).toBe(true);
    await new Promise((resolve) => (releaseHistory = resolve));
    await route.fulfill({
      json: { entries: [initial], has_more: false, next_before_revision: null },
    });
  });
  await page.getByRole('button', { name: 'Consultar etapa e historial' }).click();
  await expect.poll(() => !!releaseHead).toBe(true);
  expect(historyStarted).toBe(false);
  releaseHead();
  await expect.poll(() => !!releaseHistory).toBe(true);
  await expect(
    page.getByRole('button', { name: 'Usar etapa consultada y revisar borrador' }),
  ).toBeDisabled();
  releaseHistory();
  await page.getByRole('button', { name: 'Usar etapa consultada y revisar borrador' }).click();
  await expect(page.getByLabel('Nota (opcional)')).toHaveValue('Borrador preservado');
  expect(posts).toBe(1);
  expect(state.posts).toHaveLength(0);
});
test('support conflict preserves selected version and requires a new explicit exact read', async ({
  page,
}) => {
  await setupStages(page);
  await prepare(page);
  await page.route(`**/api/v1/cases/${caseId}/stage/transitions`, (route) =>
    route.fulfill({ status: 409, json: { error: { code: 'stage_support_changed' } } }),
  );
  await page.getByRole('button', { name: 'Registrar transici\u00f3n', exact: true }).click();
  await expect(page.getByRole('group', { name: 'Acusaci\u00f3n', exact: true })).toContainText(
    'contrato.pdf',
  );
  await expect(page.getByRole('button', { name: 'Revisar registro' })).toBeDisabled();
  await chooseSupport(page);
  await expect(page.getByRole('button', { name: 'Revisar registro' })).toBeEnabled();
});
test('confirmed upload remains selected when stage validation rejects its format', async ({
  page,
}) => {
  await setupStages(page);
  await openStages(page);
  await page.getByRole('button', { name: 'Registrar paso a Intermedia' }).click();
  await fillDate(page, 'Fecha de la acusaci\u00f3n');
  await page.getByRole('button', { name: 'Cargar soporte', exact: true }).click();
  await page.getByLabel('Archivo', { exact: true }).setInputFiles({
    name: 'soporte.pdf',
    mimeType: 'application/pdf',
    buffer: Buffer.from('not a pdf'),
  });
  await page.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(page.getByRole('group', { name: 'Acusaci\u00f3n', exact: true })).toContainText(
    'Documento guardado',
  );
  await page.route(`**/api/v1/cases/${caseId}/stage/transitions`, (route) =>
    route.fulfill({ status: 422, json: { error: { code: 'stage_support_format_rejected' } } }),
  );
  await page.getByRole('button', { name: 'Revisar registro' }).click();
  await page.getByRole('button', { name: 'Registrar transici\u00f3n', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText(
    'El documento se guard\u00f3. La etapa no se registr\u00f3.',
  );
  await expect(page.getByRole('group', { name: 'Acusaci\u00f3n', exact: true })).toContainText(
    'soporte.pdf',
  );
});
test('Client neither navigates nor requests staff stages', async ({ page }) => {
  const { state } = await setupStages(page, { role: 'client' });
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await expect(page.getByRole('link', { name: 'Etapas', exact: true })).toHaveCount(0);
  await page.evaluate(() => {
    location.hash = 'stages';
  });
  await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
  expect(state.requests).toHaveLength(0);
});
test('denied exact support clears all protected stage data', async ({ page }) => {
  await setupStages(page);
  await openStages(page);
  await page.getByRole('button', { name: 'Registrar paso a Intermedia' }).click();
  await page.route(`**/api/v1/cases/${caseId}/documents/*/versions/1`, (route) =>
    route.fulfill({ status: 403, json: { error: { code: 'permission_denied' } } }),
  );
  const field = page.getByRole('group', { name: 'Acusaci\u00f3n', exact: true });
  await field.getByRole('button', { name: 'Elegir documento' }).click();
  await field.getByRole('button', { name: /contrato.pdf/ }).click();
  await field.getByRole('button', { name: /Versi\u00f3n 1/ }).click();
  await expect(page.getByRole('heading', { name: 'Expediente no disponible' })).toBeVisible();
  await expect(page.locator('.case-stages')).toHaveCount(0);
});
