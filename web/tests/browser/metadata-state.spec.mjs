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

test('metadata editing waits for its active GET before confirming a new revision', async ({
  page,
}) => {
  await metadataSetup(page);
  let release;
  await page.route(`**/documents/${id}/metadata`, async (route) => {
    if (route.request().method() !== 'GET') return route.fallback();
    await new Promise((resolve) => {
      release = resolve;
    });
    await route.fulfill({ json: { case_id: caseId, id, ...values, metadata_revision: 1 } });
  });
  await page.getByRole('button', { name: 'Actualizar clasificaci\u00f3n', exact: true }).click();
  await expect.poll(() => typeof release).toBe('function');
  await expect(
    page.getByRole('button', { name: 'Editar clasificaci\u00f3n', exact: true }),
  ).toBeDisabled();
  const finished = page.waitForEvent(
    'requestfinished',
    (request) => request.url().endsWith('/metadata') && request.method() === 'GET',
  );
  release();
  await finished;
  await settle(page);
  const modal = await edit(page);
  await modal.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true }).fill('Confirmed');
  await modal.getByRole('button', { name: 'Guardar clasificaci\u00f3n', exact: true }).click();
  await expect(card(page).getByText('Confirmed', { exact: true })).toBeVisible();
  await expect(
    card(page).getByText('Revisi\u00f3n de clasificaci\u00f3n: 2', { exact: true }),
  ).toBeVisible();
});

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

test('metadata exhaustion preserves the draft without suggesting a conflict refresh', async ({
  page,
}) => {
  await metadataSetup(page);
  await page.route(`**/documents/${id}/metadata`, (route) =>
    route.request().method() === 'PUT'
      ? route.fulfill({
          status: 409,
          json: { error: { code: 'document_metadata_revision_exhausted' } },
        })
      : route.fallback(),
  );
  const modal = await edit(page);
  await modal.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true }).fill('Retained');
  await modal.getByRole('button', { name: 'Guardar clasificaci\u00f3n', exact: true }).click();
  await expect(modal.getByRole('alert')).toContainText('l\u00edmite');
  await expect(
    modal.getByRole('button', { name: 'Guardar clasificaci\u00f3n', exact: true }),
  ).toBeDisabled();
  await expect(
    modal.getByRole('button', { name: 'Consultar clasificaci\u00f3n actual' }),
  ).toHaveCount(0);
  await expect(modal.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true })).toHaveValue(
    'Retained',
  );
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

test('a pending edit cannot enter a different document workspace', async ({ page }) => {
  await metadataSetup(page);
  let release;
  await page.route(`**/documents/${id}/metadata`, async (route) => {
    if (route.request().method() !== 'PUT') return route.fallback();
    await new Promise((resolve) => {
      release = resolve;
    });
    await route.fulfill({
      json: { case_id: caseId, id, ...values, classification: 'Late secret', metadata_revision: 2 },
    });
  });
  const modal = await edit(page);
  await modal.getByRole('button', { name: 'Guardar clasificaci\u00f3n', exact: true }).click();
  await expect.poll(() => typeof release).toBe('function');
  await page.evaluate(() => {
    location.hash = '#cases';
  });
  await expect(page.getByRole('heading', { name: 'Expedientes', exact: true })).toBeVisible();
  const finished = page.waitForEvent(
    'requestfinished',
    (request) => request.url().endsWith('/metadata') && request.method() === 'PUT',
  );
  release();
  await finished;
  await settle(page);
  await expect(page.getByText('Late secret', { exact: true })).toHaveCount(0);
  await expect(page.getByRole('heading', { name: 'Expedientes', exact: true })).toBeVisible();
});

test('R0 classification shows no fabricated actor and accepts an explicit empty first revision', async ({
  page,
}) => {
  const { state } = await metadataSetup(page, { revision: 0 });
  state.current = { ...emptyMetadata };
  await page.getByRole('button', { name: 'Actualizar clasificaci\u00f3n', exact: true }).click();
  const modal = await edit(page);
  await modal.getByRole('button', { name: 'Guardar clasificaci\u00f3n', exact: true }).click();
  await expect(
    card(page).getByText('Revisi\u00f3n de clasificaci\u00f3n: 1', { exact: true }),
  ).toBeVisible();
  await expect(
    card(page).getByText('Sin valores de clasificaci\u00f3n', { exact: true }),
  ).toBeVisible();
  await expect(card(page).getByText('historic@example.com', { exact: true })).toHaveCount(0);
});
