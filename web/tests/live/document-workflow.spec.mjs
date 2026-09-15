import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { test, expect } from '@playwright/test';

const fixture = JSON.parse(readFileSync(process.env.TT_WEB_FIXTURES, 'utf8'));

async function login(page, recoveryCode) {
  await page.getByLabel('Correo electr\u00f3nico').fill(fixture.email);
  await page.getByLabel('Contrase\u00f1a', { exact: true }).fill(fixture.password);
  await page.getByRole('button', { name: 'Continuar', exact: true }).click();
  await page.getByRole('button', { name: 'Usar c\u00f3digo de recuperaci\u00f3n' }).click();
  await page.getByLabel('C\u00f3digo de recuperaci\u00f3n', { exact: true }).fill(recoveryCode);
  await page.getByRole('button', { name: 'Verificar y entrar' }).click();
  await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
}

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
  await page.evaluate(() => {
    document.activeElement?.blur();
    window.scrollTo(0, 0);
  });
  await page.screenshot({ path: testInfo.outputPath('document-desktop.png'), fullPage: true });
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n' }).click();
  await expect(page.getByLabel('Correo electr\u00f3nico')).toBeVisible();
  await page.reload();
  await login(page, fixture.recoveryCodes[1]);
  await page
    .getByRole('navigation')
    .getByRole('button', { name: 'Expedientes', exact: true })
    .click();
  await page.getByRole('button', { name: /Browser evidence case/ }).click();
  await expect(page.getByText('browser-evidence.txt', { exact: true })).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({ path: testInfo.outputPath('documents-mobile.png'), fullPage: true });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  expect(paths.some((path) => /^\/api\/v1\/cases\/[^/]+\/documents$/.test(path))).toBe(true);
  expect(paths.some((path) => path.startsWith('/api/v1/documents'))).toBe(false);
  expect(errors).toEqual([]);
});
