import { test, expect } from '@playwright/test';
import { caseId, id, document } from './helpers.mjs';
import { versionSetup, append } from './version-helpers.mjs';

test('append keeps both versions and historic actions target the selected exact version', async ({
  page,
}) => {
  const { calls } = await versionSetup(page);
  await append(page);
  await expect(page.getByRole('heading', { name: 'version-two.txt' })).toBeVisible();
  await expect(page.getByText('Versi\u00f3n actual: 2', { exact: true })).toBeVisible();
  await expect(
    page.locator('.document-list').getByText('version-two.txt', { exact: true }),
  ).toBeVisible();
  await page.getByRole('button', { name: /Versi\u00f3n 1.*contrato.pdf/ }).click();
  await expect(page.getByRole('heading', { name: 'contrato.pdf' })).toBeVisible();
  await expect(
    page.getByText('Consultando versi\u00f3n hist\u00f3rica: 1', { exact: true }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Verificar integridad', exact: true }).click();
  await expect(page.getByText('Verificaci\u00f3n v\u00e1lida', { exact: true })).toBeVisible();
  const downloaded = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Descargar evidencia', exact: true }).click();
  await downloaded;
  expect(calls.some((call) => call.path.endsWith('/versions/1/verify'))).toBe(true);
  expect(calls.some((call) => call.path.endsWith('/versions/1/evidence'))).toBe(true);
  expect(calls.some((call) => call.path.endsWith(`/documents/${id}/verify`))).toBe(false);
  await page.evaluate(() => scrollTo(0, 0));
  await page.screenshot({ path: 'test-results/version-history-desktop.png', fullPage: true });
});

test('append conflict preserves file and requires explicit refresh and resubmission', async ({
  page,
}) => {
  const { calls } = await versionSetup(page, { conflict: true });
  await append(page, 'my-version.txt');
  const modal = page.getByRole('dialog', { name: 'Agregar versi\u00f3n', exact: true });
  await expect(modal.getByRole('alert')).toContainText('cambi');
  await expect(modal.getByLabel('Nombre de la nueva versi\u00f3n')).toHaveValue('my-version.txt');
  expect(
    await modal
      .getByLabel('Archivo de la nueva versi\u00f3n')
      .evaluate((input) => input.files[0].name),
  ).toBe('my-version.txt');
  expect(
    calls.filter((call) => call.path.endsWith('/versions') && call.method === 'POST'),
  ).toHaveLength(1);
  await modal.getByRole('button', { name: 'Consultar versi\u00f3n actual', exact: true }).click();
  await expect(modal.getByText('Versi\u00f3n de partida: 2', { exact: true })).toBeVisible();
  await expect(
    page.locator('.document-list').getByText('other-editor.txt', { exact: true }),
  ).toBeVisible();
  expect(
    calls.filter((call) => call.path.endsWith('/versions') && call.method === 'POST'),
  ).toHaveLength(1);
  await modal.getByRole('button', { name: 'Guardar nueva versi\u00f3n', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'my-version.txt' })).toBeVisible();
  expect(
    calls
      .filter((call) => call.method === 'POST' && call.path.endsWith('/versions'))
      .map((call) => call.search),
  ).toEqual(['?expected_version=1', '?expected_version=2']);
});

test('mobile history keeps version controls readable without horizontal overflow', async ({
  page,
}) => {
  await versionSetup(page);
  await append(page, 'new-procedural-document-with-a-long-name.txt');
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(
    page.getByRole('heading', {
      name: 'new-procedural-document-with-a-long-name.txt',
      exact: true,
    }),
  ).toBeVisible();
  const history = page.getByRole('region', { name: 'Historial de versiones' });
  await expect(history).toHaveAttribute('aria-busy', 'false');
  await expect(history.getByRole('button', { name: /^Versi\u00f3n/ })).toHaveCount(2);
  await expect
    .poll(() =>
      history.evaluate((region) => {
        const bounds = region.getBoundingClientRect();
        return [...region.querySelectorAll('button')].every((button) => {
          const box = button.getBoundingClientRect();
          return box.width > 0 && box.x >= bounds.x && box.right <= bounds.right;
        });
      }),
    )
    .toBe(true);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.evaluate(() => {
    document.activeElement?.blur();
    scrollTo(0, 0);
  });
  await page.screenshot({ path: 'test-results/version-history-mobile.png', fullPage: true });
});

test('history access denial clears the selected version and disables adding a version', async ({
  page,
}) => {
  await versionSetup(page);
  await page.route(`**/documents/${id}/versions?**`, (route) =>
    route.fulfill({ status: 404, json: { error: { code: 'case_not_found' } } }),
  );
  await page.getByRole('button', { name: 'Actualizar historial', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('ya no tienes acceso');
  await expect(page.getByRole('heading', { name: 'contrato.pdf' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Agregar versi\u00f3n', exact: true })).toHaveCount(
    0,
  );
});
