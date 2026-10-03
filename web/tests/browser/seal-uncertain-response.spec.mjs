import { test, expect } from '@playwright/test';
import { setup, login, openDocument, caseId, id, document } from './helpers.mjs';

for (const failure of ['network', '503'])
  for (const applied of [false, true]) {
    test(`uncertain seal ${failure} requires the exact version read before another intent, applied=${applied}`, async ({
      page,
    }) => {
      await setup(page);
      await login(page);
      await openDocument(page);
      await expect(
        page.getByRole('button', { name: 'Actualizar historial', exact: true }),
      ).toBeEnabled();
      let writes = 0,
        reads = 0,
        entered = false,
        release;
      const gate = new Promise((resolve) => {
        release = resolve;
      });
      await page.route(`**/cases/${caseId}/documents/${id}/versions/1/seal`, (route) => {
        expect(route.request().method()).toBe('POST');
        writes++;
        return failure === 'network'
          ? route.abort('failed')
          : route.fulfill({ status: 503, json: { error: { code: 'server_busy' } } });
      });
      await page.route(`**/cases/${caseId}/documents/${id}/versions/1`, async (route) => {
        expect(route.request().method()).toBe('GET');
        reads++;
        entered = true;
        await gate;
        await route.fulfill({ json: { ...document, sealed: applied } });
      });
      const confirm = page.getByRole('button', { name: 'Confirmar sellado', exact: true });
      try {
        await page.getByRole('button', { name: 'Sellar documento', exact: true }).click();
        await confirm.click();
        await expect(page.locator('.detail-panel').getByRole('alert')).toBeVisible();
        await expect(confirm).toBeDisabled();
        expect(writes).toBe(1);
        expect(reads).toBe(0);
        await page
          .getByRole('button', { name: 'Consultar estado del sellado', exact: true })
          .click();
        await expect.poll(() => entered).toBe(true);
        await expect(confirm).toBeDisabled();
        expect(writes).toBe(1);
      } finally {
        release();
      }
      await expect(confirm).toHaveCount(0);
      const start = page.getByRole('button', { name: 'Sellar documento', exact: true });
      if (applied) {
        await expect(start).toHaveCount(0);
        await expect(page.locator('.detail-panel .badge')).toHaveText('Sellado');
      } else {
        await expect(start).toBeEnabled();
        await start.click();
        await expect(confirm).toBeEnabled();
      }
      expect(writes).toBe(1);
      expect(reads).toBe(1);
    });
  }

test('cancelling an uncertain seal keeps lookup available and prevents reopening until the exact read', async ({
  page,
}) => {
  await setup(page);
  await login(page);
  await openDocument(page);
  await expect(
    page.getByRole('button', { name: 'Actualizar historial', exact: true }),
  ).toBeEnabled();
  let writes = 0,
    reads = 0;
  await page.route(`**/cases/${caseId}/documents/${id}/versions/1/seal`, (route) => {
    expect(route.request().method()).toBe('POST');
    writes++;
    return route.fulfill({ status: 503, json: { error: { code: 'server_busy' } } });
  });
  await page.route(`**/cases/${caseId}/documents/${id}/versions/1`, (route) => {
    expect(route.request().method()).toBe('GET');
    reads++;
    return route.fulfill({ json: { ...document, sealed: false } });
  });
  const start = page.getByRole('button', { name: 'Sellar documento', exact: true });
  const confirm = page.getByRole('button', { name: 'Confirmar sellado', exact: true });
  await start.click();
  await confirm.click();
  await expect(page.locator('.detail-panel').getByRole('alert')).toBeVisible();
  expect(reads).toBe(0);
  expect(writes).toBe(1);
  await page
    .locator('.seal-confirmation')
    .getByRole('button', { name: 'Cancelar', exact: true })
    .click();
  await expect(confirm).toHaveCount(0);
  await expect(start).toBeDisabled();
  expect(reads).toBe(0);
  expect(writes).toBe(1);
  await page.getByRole('button', { name: 'Consultar estado del sellado', exact: true }).click();
  await expect.poll(() => reads).toBe(1);
  await expect(start).toBeEnabled();
  await expect(confirm).toHaveCount(0);
  await start.click();
  await expect(confirm).toBeEnabled();
  expect(reads).toBe(1);
  expect(writes).toBe(1);
});
