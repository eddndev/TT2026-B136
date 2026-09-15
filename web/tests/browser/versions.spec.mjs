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

test('history loads older pages using the returned cursor and identifies imported history', async ({
  page,
}) => {
  await versionSetup(page);
  const historyCalls = [];
  await page.route(`**/documents/${id}/versions?**`, (route) => {
    const before = new URL(route.request().url()).searchParams.get('before_version');
    historyCalls.push(before);
    return route.fulfill({
      json: {
        versions: (before ? [4] : [6, 5]).map((version) => ({
          ...document,
          version,
          name: `version-${version}.txt`,
        })),
        has_more: !before,
        next_before_version: before ? null : 5,
        first_available_version: 4,
      },
    });
  });
  await page.getByRole('button', { name: 'Actualizar historial', exact: true }).click();
  await expect(
    page.getByText('El historial disponible comienza en la versi\u00f3n 4.'),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Cargar versiones anteriores', exact: true }).click();
  await expect(page.getByRole('button', { name: /Versi\u00f3n 4.*version-4.txt/ })).toBeVisible();
  expect(historyCalls).toEqual([null, '5']);
  await expect(page.locator('.version-row strong')).toHaveText([
    'Versi\u00f3n 6 / version-6.txt',
    'Versi\u00f3n 5 / version-5.txt',
    'Versi\u00f3n 4 / version-4.txt',
  ]);
  await expect(
    page.getByRole('button', { name: 'Cargar versiones anteriores', exact: true }),
  ).toHaveCount(0);
});

test('late historical detail cannot replace a more recently selected version', async ({ page }) => {
  await versionSetup(page);
  await append(page);
  let release;
  await page.route(
    `**/documents/${id}/versions/1`,
    (route) =>
      new Promise((resolve) => {
        release = async () => {
          await route.fulfill({ json: { ...document, sealed: true } });
          resolve();
        };
      }),
  );
  await page.getByRole('button', { name: /Versi\u00f3n 1.*contrato.pdf/ }).click();
  await expect.poll(() => typeof release).toBe('function');
  await page.getByRole('button', { name: /Versi\u00f3n 2.*version-two.txt/ }).click();
  await expect(page.getByRole('heading', { name: 'version-two.txt' })).toBeVisible();
  const finished = page.waitForEvent('requestfinished', (request) =>
    request.url().endsWith('/versions/1'),
  );
  await release();
  await finished;
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
  await expect(page.getByRole('heading', { name: 'version-two.txt' })).toBeVisible();
});

test('late historical evidence is discarded after a different version is selected', async ({
  page,
}) => {
  await versionSetup(page);
  await append(page);
  await page.getByRole('button', { name: /Versi\u00f3n 1.*contrato.pdf/ }).click();
  await expect(
    page.getByRole('button', { name: 'Descargar evidencia', exact: true }),
  ).toBeEnabled();
  const downloads = [];
  page.on('download', (download) => downloads.push(download));
  let release;
  await page.route(
    `**/documents/${id}/versions/1/evidence`,
    (route) =>
      new Promise((resolve) => {
        release = async () => {
          await route.fulfill({ contentType: 'application/zip', body: 'late archive' });
          resolve();
        };
      }),
  );
  await page.getByRole('button', { name: 'Descargar evidencia', exact: true }).click();
  await expect.poll(() => typeof release).toBe('function');
  await page.getByRole('button', { name: /Versi\u00f3n 2.*version-two.txt/ }).click();
  await expect(page.getByRole('heading', { name: 'version-two.txt' })).toBeVisible();
  const finished = page.waitForEvent('requestfinished', (request) =>
    request.url().endsWith('/versions/1/evidence'),
  );
  await release();
  await finished;
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
  expect(downloads).toHaveLength(0);
  await expect(page.getByText('Evidencia descargada.')).toHaveCount(0);
});

test('mobile history keeps version controls readable without horizontal overflow', async ({
  page,
}) => {
  await versionSetup(page);
  await append(page, 'new-procedural-document-with-a-long-name.txt');
  await page.setViewportSize({ width: 390, height: 844 });
  const history = page.getByRole('region', { name: 'Historial de versiones' });
  const width = await history.boundingBox();
  for (const button of await history.getByRole('button').all()) {
    const box = await button.boundingBox();
    expect(box.x).toBeGreaterThanOrEqual(width.x);
    expect(box.x + box.width).toBeLessThanOrEqual(width.x + width.width);
  }
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

test('exhausted version numbers preserve the file without suggesting a conflict refresh', async ({
  page,
}) => {
  await versionSetup(page);
  let submissions = 0;
  await page.route(`**/documents/${id}/versions?expected_version=*`, (route) => {
    submissions++;
    return route.fulfill({ status: 409, json: { error: { code: 'document_version_exhausted' } } });
  });
  await append(page, 'retained-file.txt');
  const modal = page.getByRole('dialog', { name: 'Agregar versi\u00f3n', exact: true });
  await expect(modal.getByRole('alert')).toContainText('alcanzado el l\u00edmite de versiones');
  await expect(
    modal.getByRole('button', { name: 'Consultar versi\u00f3n actual', exact: true }),
  ).toHaveCount(0);
  await expect(
    modal.getByRole('button', { name: 'Guardar nueva versi\u00f3n', exact: true }),
  ).toBeDisabled();
  await expect(modal.getByLabel('Nombre de la nueva versi\u00f3n')).toHaveValue(
    'retained-file.txt',
  );
  expect(
    await modal
      .getByLabel('Archivo de la nueva versi\u00f3n')
      .evaluate((input) => input.files[0].name),
  ).toBe('retained-file.txt');
  expect(submissions).toBe(1);
});

test('detail access denial clears cached history and ignores a pending history response', async ({
  page,
}) => {
  await versionSetup(page);
  let denyDetail;
  let releaseHistory;
  await page.route(
    `**/documents/${id}/versions/1`,
    (route) =>
      new Promise((resolve) => {
        denyDetail = async () => {
          await route.fulfill({ status: 403, json: { error: { code: 'permission_denied' } } });
          resolve();
        };
      }),
  );
  await page.route(
    `**/documents/${id}/versions?**`,
    (route) =>
      new Promise((resolve) => {
        releaseHistory = async () => {
          await route.fulfill({
            json: {
              versions: [document],
              has_more: false,
              next_before_version: null,
              first_available_version: 1,
            },
          });
          resolve();
        };
      }),
  );
  await page.getByRole('button', { name: /Versi\u00f3n 1.*contrato.pdf/ }).click();
  await expect.poll(() => typeof denyDetail).toBe('function');
  await page.getByRole('button', { name: 'Actualizar historial', exact: true }).click();
  await expect.poll(() => typeof releaseHistory).toBe('function');
  await denyDetail();
  await expect(page.getByRole('alert')).toContainText('No tienes permiso');
  const finished = page.waitForEvent('requestfinished', (request) =>
    request.url().includes('/versions?'),
  );
  await releaseHistory();
  await finished;
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
  await expect(page.getByRole('button', { name: 'Agregar versi\u00f3n', exact: true })).toHaveCount(
    0,
  );
  await expect(page.locator('.version-row')).toHaveCount(0);
  await expect(page.getByRole('heading', { name: 'contrato.pdf' })).toHaveCount(0);
  await expect(page.getByText('Consultando la versi\u00f3n seleccionada...')).toHaveCount(0);
});
