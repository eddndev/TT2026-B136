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
import { login, navigate, expectClientWorkspace } from './helpers.mjs';
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
  await expectClientWorkspace(page, ['stages']);
  expect(state.requests).toHaveLength(0);
});
