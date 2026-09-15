import { test, expect } from '@playwright/test';
import { setup, login, navigate, selectCase, caseId, id, document } from './helpers.mjs';

test('persistent case selection lists metadata and GET detail without verifying', async ({
  page,
}) => {
  const requests = await setup(page);
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await expect(page.getByRole('button', { name: /Defensa inicial/ })).toBeVisible();
  await page.screenshot({ path: 'test-results/cases-list-desktop.png', fullPage: true });
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
  await page.getByRole('link', { name: 'Documentos', exact: true }).click();
  await page.getByRole('button', { name: `Abrir ${document.name}`, exact: true }).click();
  await expect(page.getByRole('heading', { name: document.name, exact: true })).toBeVisible();
  expect(requests.some((r) => r.path.endsWith(`/documents/${id}`) && r.method === 'GET')).toBe(
    true,
  );
  expect(requests.some((r) => r.path.endsWith('/verify'))).toBe(false);
});

test('document pagination and filters query server and clear old rows', async ({ page }) => {
  await setup(page);
  const calls = [];
  await page.route(`**/cases/${caseId}/documents?**`, (route) => {
    const query = new URL(route.request().url()).searchParams;
    calls.push(query);
    return route.fulfill({
      json: {
        documents: query.get('name') ? [] : [document],
        has_more: !query.get('name') && query.get('offset') === '0',
      },
    });
  });
  await login(page);
  await page.getByRole('button', { name: 'Siguiente', exact: true }).click();
  await expect.poll(() => calls.some((q) => q.get('offset') === '50')).toBe(true);
  await page.getByLabel('Buscar por nombre').fill('inexistente');
  await page.getByRole('button', { name: 'Buscar', exact: true }).click();
  await expect(page.getByText('No encontramos coincidencias')).toBeVisible();
  expect(calls.some((q) => q.get('name') === 'inexistente' && q.get('offset') === '0')).toBe(true);
  await page.getByLabel('Filtrar por estado').selectOption('sealed');
  await expect.poll(() => calls.some((q) => q.get('sealed') === 'true')).toBe(true);
});

test('client can select assigned case metadata but never requests documents or staff data', async ({
  page,
}) => {
  const requests = await setup(page, 'client');
  await login(page, false, false);
  await selectCase(page);
  await expect(page.getByText('Acceso documental pendiente')).toBeVisible();
  expect(
    requests.some((r) => /\/documents|case-administrations|\/administration/.test(r.path)),
  ).toBe(false);
  await expect(page.getByRole('button', { name: 'Nuevo expediente penal' })).toHaveCount(0);
});

test('mobile case filters and document search preserve useful widths without overlap', async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await setup(page);
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  const title = await page.getByLabel('Buscar por t\u00edtulo').boundingBox();
  const nuc = await page.getByLabel('NUC exacto').boundingBox();
  expect(title.width).toBeGreaterThan(200);
  expect(title.y + title.height).toBeLessThan(nuc.y);
  await page.screenshot({ path: 'test-results/cases-list-mobile.png', fullPage: true });
  await selectCase(page);
  const input = await page.getByLabel('Buscar por nombre').boundingBox();
  const search = await page.getByRole('button', { name: 'Buscar', exact: true }).boundingBox();
  const filter = await page.getByLabel('Filtrar por estado').boundingBox();
  expect(input.width).toBeGreaterThanOrEqual(150);
  expect(input.x + input.width).toBeLessThanOrEqual(search.x);
  expect(search.y + search.height).toBeLessThanOrEqual(filter.y);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});
