import { test, expect } from '@playwright/test';
import { metadataSetup, emptyMetadata, values } from './metadata-helpers.mjs';
import { caseId, id } from './helpers.mjs';
const card = (page) =>
  page.getByRole('region', { name: 'Clasificaci\u00f3n actual del documento' });
const dialog = (page) => page.getByRole('dialog', { name: 'Editar clasificaci\u00f3n' });
async function edit(page) {
  await page.getByRole('button', { name: 'Editar clasificaci\u00f3n', exact: true }).click();
  return dialog(page);
}
async function settle(page) {
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
}

test('classification changes preserve the selected content verification and clear values explicitly', async ({
  page,
}) => {
  await metadataSetup(page);
  await page.getByRole('button', { name: 'Verificar integridad', exact: true }).click();
  await expect(page.getByText('Verificaci\u00f3n v\u00e1lida', { exact: true })).toBeVisible();
  const modal = await edit(page);
  await modal.getByLabel('Tipo de documento (opcional)', { exact: true }).fill('');
  await modal.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true }).fill('');
  await modal.getByRole('button', { name: 'Quitar etiqueta: acci\u00f3n, prueba' }).click();
  await modal.getByRole('button', { name: 'Guardar clasificaci\u00f3n', exact: true }).click();
  await expect(
    card(page).getByText('Sin valores de clasificaci\u00f3n', { exact: true }),
  ).toBeVisible();
  await expect(
    card(page).getByText('Revisi\u00f3n de clasificaci\u00f3n: 2', { exact: true }),
  ).toBeVisible();
  await expect(page.getByText('Verificaci\u00f3n v\u00e1lida', { exact: true })).toBeVisible();
  await expect(page.getByText('Consultando versi\u00f3n actual: 1', { exact: true })).toBeVisible();
});

test('history denial removes classification, content and cached rows', async ({ page }) => {
  await metadataSetup(page);
  await page.route(`**/documents/${id}/metadata/history?**`, (route) =>
    route.fulfill({ status: 404, json: { error: { code: 'document_not_found' } } }),
  );
  await page.getByRole('button', { name: 'Ver historial de clasificaci\u00f3n' }).click();
  await expect(page.getByRole('alert')).toContainText('No se encontr');
  await expect(card(page)).toHaveCount(0);
  await expect(page.getByRole('heading', { name: 'contrato.pdf' })).toHaveCount(0);
  await expect(page.locator('.document-list tbody tr')).toHaveCount(0);
});
