import { test, expect } from '@playwright/test';
import { metadataSetup } from './metadata-helpers.mjs';

test('classification filters keep a comma tag exact and clear all applied values', async ({
  page,
}) => {
  const { calls } = await metadataSetup(page);
  await page.getByText('Filtros de clasificaci\u00f3n', { exact: true }).click();
  await page.getByLabel('Tipo exacto', { exact: true }).fill('Escrito');
  await page.getByLabel('Clasificaci\u00f3n exacta', { exact: true }).fill('Civil');
  await page.getByLabel('Etiqueta exacta', { exact: true }).fill('acci\u00f3n, prueba');
  await page.getByRole('button', { name: 'Aplicar filtros', exact: true }).click();
  await expect
    .poll(() => calls.filter((call) => call.search?.includes('tag=')).length)
    .toBeGreaterThan(0);
  const params = new URLSearchParams(
    calls.filter((call) => call.search?.includes('tag=')).at(-1).search,
  );
  expect(params.get('tag')).toBe('acci\u00f3n, prueba');
  expect(params.get('classification')).toBe('Civil');
  await page.getByRole('button', { name: 'Limpiar filtros', exact: true }).click();
  await expect(page.getByLabel('Etiqueta exacta', { exact: true })).toHaveValue('');
});

test('mobile classification keeps long comma tags and controls inside the Qadra dialog', async ({
  page,
}) => {
  const { state } = await metadataSetup(page);
  state.current = {
    ...state.current,
    tags: Array.from({ length: 20 }, (_, i) => `Etiqueta, prueba ${i} ${'\u00e1'.repeat(20)}`),
    metadata_revision: 2,
  };
  await page.getByRole('button', { name: 'Actualizar clasificaci\u00f3n', exact: true }).click();
  const card = page.getByRole('region', { name: 'Clasificaci\u00f3n actual del documento' });
  await expect(
    card.getByText('Revisi\u00f3n de clasificaci\u00f3n: 2', { exact: true }),
  ).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  await page.getByRole('button', { name: 'Editar clasificaci\u00f3n', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Editar clasificaci\u00f3n' });
  const bounds = await modal.boundingBox();
  for (const badge of await modal.locator('.metadata-tags .badge').all()) {
    const box = await badge.boundingBox();
    expect(box.x).toBeGreaterThanOrEqual(bounds.x);
    expect(box.x + box.width).toBeLessThanOrEqual(bounds.x + bounds.width);
    const remove = await badge.getByRole('button').boundingBox();
    const label = await badge.locator('span').boundingBox();
    expect(label.x + label.width).toBeLessThanOrEqual(remove.x);
  }
  await modal
    .getByRole('button', { name: 'Guardar clasificaci\u00f3n', exact: true })
    .scrollIntoViewIfNeeded();
  await page.screenshot({ path: 'test-results/classification-editor-mobile.png' });
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await page.evaluate(() => {
    document.activeElement?.blur();
    scrollTo(0, 0);
  });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: 'test-results/classification-mobile.png', fullPage: true });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.screenshot({ path: 'test-results/classification-desktop.png', fullPage: true });
});

test('the document list presents current metadata as escaped compact labels', async ({ page }) => {
  const { state } = await metadataSetup(page);
  state.current = {
    ...state.current,
    document_type: '<img src=x onerror=alert(1)>',
    metadata_revision: 2,
  };
  await page.getByRole('button', { name: 'Actualizar clasificaci\u00f3n', exact: true }).click();
  const list = page.locator('.document-list');
  await expect(list.getByText('<img src=x onerror=alert(1)>', { exact: true })).toBeVisible();
  await expect(list.getByText('acci\u00f3n, prueba', { exact: true })).toBeVisible();
  await expect(list.locator('img')).toHaveCount(0);
});
