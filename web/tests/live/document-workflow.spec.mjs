import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { test, expect } from '@playwright/test';

import { fixture, login, capture } from './helpers.mjs';

test('persisted cases and document evidence work through the real services', async ({
  page,
}, testInfo) => {
  const errors = [];
  const paths = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('request', (request) => {
    const url = new URL(request.url());
    if (url.pathname.startsWith('/api/')) paths.push(url.pathname);
  });
  await page.goto('/');
  await login(page, fixture.recoveryCodes[0]);
  await page
    .getByRole('navigation')
    .getByRole('button', { name: 'Expedientes', exact: true })
    .click();
  await page.getByRole('button', { name: 'Nuevo expediente', exact: true }).click();
  await page.getByLabel('T\u00edtulo del expediente').fill('Browser evidence case');
  await page.getByLabel('Referencia del expediente').fill('BROWSER-001');
  await page.getByRole('button', { name: 'Crear expediente', exact: true }).click();
  await page.getByRole('button', { name: 'Subir documento', exact: true }).first().click();
  await page.getByLabel('Archivo', { exact: true }).setInputFiles({
    name: 'browser-evidence.txt',
    mimeType: 'text/plain',
    buffer: Buffer.from('Browser evidence bytes.\n'),
  });
  await page.getByLabel('Nombre del documento').fill('browser-evidence.txt');
  await page.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'browser-evidence.txt', exact: true }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Sellar documento', exact: true }).click();
  await page.getByRole('button', { name: 'Confirmar sellado', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Descargar evidencia', exact: true }),
  ).toBeEnabled();
  await page.getByRole('button', { name: 'Verificar integridad', exact: true }).click();
  await expect(page.getByText('Verificaci\u00f3n v\u00e1lida', { exact: true })).toBeVisible();
  const downloadEvent = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Descargar evidencia', exact: true }).click();
  const download = await downloadEvent;
  const archive = testInfo.outputPath('browser-evidence.zip');
  await download.saveAs(archive);
  execFileSync('unzip', ['-t', archive]);
  expect(execFileSync('unzip', ['-p', archive, 'browser-evidence.txt'], { encoding: 'utf8' })).toBe(
    'Browser evidence bytes.\n',
  );
  const originalArchive = readFileSync(archive);
  await page.getByRole('button', { name: 'Agregar versi\u00f3n', exact: true }).click();
  await page.getByLabel('Archivo de la nueva versi\u00f3n', { exact: true }).setInputFiles({
    name: 'browser-evidence-v2.txt',
    mimeType: 'text/plain',
    buffer: Buffer.from('Second immutable version.\n'),
  });
  await page.getByRole('button', { name: 'Guardar nueva versi\u00f3n', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'browser-evidence-v2.txt', exact: true }),
  ).toBeVisible();
  await expect(page.getByText('Versi\u00f3n actual: 2', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Sellar documento', exact: true }).click();
  await page.getByRole('button', { name: 'Confirmar sellado', exact: true }).click();
  await page.getByRole('button', { name: 'Verificar integridad', exact: true }).click();
  await expect(page.getByText('Verificaci\u00f3n v\u00e1lida', { exact: true })).toBeVisible();
  const secondDownload = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Descargar evidencia', exact: true }).click();
  const secondArchive = testInfo.outputPath('v2-evidence.zip');
  await (await secondDownload).saveAs(secondArchive);
  expect(
    execFileSync('unzip', ['-p', secondArchive, 'browser-evidence-v2.txt'], { encoding: 'utf8' }),
  ).toBe('Second immutable version.\n');
  expect(readFileSync(secondArchive)).not.toEqual(originalArchive);
  await capture(page, testInfo, 'current-desktop');
  await page.getByRole('button', { name: /Versi\u00f3n 1.*browser-evidence.txt/ }).click();
  await expect(
    page.getByText('Consultando versi\u00f3n hist\u00f3rica: 1', { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole('heading', { name: 'browser-evidence.txt', exact: true }),
  ).toBeVisible();
  const historicDownload = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Descargar evidencia', exact: true }).click();
  const historicalArchive = testInfo.outputPath('historical-v1-evidence.zip');
  await (await historicDownload).saveAs(historicalArchive);
  expect(readFileSync(historicalArchive)).toEqual(originalArchive);
  await capture(page, testInfo, 'historical-desktop');
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n' }).click();
  await expect(page.getByLabel('Correo electr\u00f3nico')).toBeVisible();
  await page.reload();
  await login(page, fixture.recoveryCodes[1]);
  await page
    .getByRole('navigation')
    .getByRole('button', { name: 'Expedientes', exact: true })
    .click();
  await page.getByRole('button', { name: /Browser evidence case/ }).click();
  await expect(page.getByText('browser-evidence-v2.txt', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: /browser-evidence-v2.txt/ }).click();
  await expect(
    page.getByRole('button', { name: /Versi\u00f3n 1.*browser-evidence.txt/ }),
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: /Versi\u00f3n 2.*browser-evidence-v2.txt/ }),
  ).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  await capture(page, testInfo, 'current-mobile');
  await page.getByRole('button', { name: /Versi\u00f3n 1.*browser-evidence.txt/ }).click();
  await expect(
    page.getByText('Consultando versi\u00f3n hist\u00f3rica: 1', { exact: true }),
  ).toBeVisible();
  await capture(page, testInfo, 'historical-mobile');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  expect(paths.some((path) => /^\/api\/v1\/cases\/[^/]+\/documents$/.test(path))).toBe(true);
  expect(paths.some((path) => path.endsWith('/versions/1/evidence'))).toBe(true);
  expect(paths.some((path) => path.endsWith('/versions/2/verify'))).toBe(true);
  expect(paths.some((path) => path.endsWith('/versions/2/evidence'))).toBe(true);
  expect(paths.some((path) => path.startsWith('/api/v1/documents'))).toBe(false);
  expect(errors).toEqual([]);
});
