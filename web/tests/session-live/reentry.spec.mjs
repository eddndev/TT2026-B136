import { readFileSync } from 'node:fs';
import { test, expect } from '@playwright/test';
import { login } from '../live/helpers.mjs';
import { fillProfile } from '../live/case-administration-helpers.mjs';

const fixture = JSON.parse(readFileSync(process.env.TT_WEB_FIXTURES, 'utf8'));
const title = 'Idle acceptance case';
const modalCase = (page) =>
  page.getByRole('region', { name: 'Formulario del expediente', exact: true });

async function newCase(page) {
  await page
    .getByRole('navigation')
    .getByRole('button', { name: 'Expedientes', exact: true })
    .click();
  await page.getByRole('button', { name: 'Nuevo expediente penal', exact: true }).click();
  return modalCase(page);
}

async function expired(page, modal, token, request) {
  await expect(page.getByLabel('Correo electr\u00f3nico')).toBeVisible({ timeout: 20000 });
  await expect(modal).toBeHidden();
  const read = () =>
    request.get(`${process.env.API_PROXY_TARGET}/api/v1/auth/session`, {
      headers: { Authorization: `Bearer ${token}` },
    });
  let response = await read();
  if (response.status() === 200) {
    const state = await response.json();
    const remaining = state.idle_expires_at_unix_ms - state.server_now_unix_ms;
    console.log(JSON.stringify({ conservative_client_expiry_lead_ms: remaining }));
    expect(remaining).toBeGreaterThan(0);
    expect(remaining).toBeLessThanOrEqual(1000);
    await new Promise((resolve) => setTimeout(resolve, remaining + 50));
    response = await read();
  }
  expect(response.status()).toBe(401);
}

test('real idle expiry retains raw case fields and an upload until explicit authorized reentry', async ({
  page,
  request,
}, testInfo) => {
  const grants = [],
    writes = [],
    errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('response', async (response) => {
    if (new URL(response.url()).pathname === '/api/v1/auth/mfa/recovery' && response.ok())
      grants.push(await response.json());
  });
  page.on('request', (request) => {
    if (request.method() === 'POST' && !new URL(request.url()).pathname.startsWith('/api/v1/auth/'))
      writes.push(new URL(request.url()).pathname);
  });
  await page.goto('/');
  await login(page, fixture.recoveryCodes[0]);
  let modal = await newCase(page);
  await modal.getByLabel('T\u00edtulo del expediente', { exact: true }).fill(`  ${title}  `);
  await modal.getByLabel('Referencia interna', { exact: true }).fill('  IDLE-CASE  ');
  await fillProfile(page, 'IDLE');
  await expect.poll(() => grants.length).toBe(1);
  expect(grants[0].policy.idle_ttl_seconds).toBe(12);
  await expired(page, modal, grants[0].access_token, request);
  expect(writes).toEqual([]);
  await page.screenshot({ path: testInfo.outputPath('idle-expired.png'), fullPage: true });
  await login(page, fixture.recoveryCodes[1]);
  modal = await newCase(page);
  await expect(modal.getByLabel('T\u00edtulo del expediente', { exact: true })).toHaveValue(
    `  ${title}  `,
  );
  await expect(modal.getByLabel('Referencia interna', { exact: true })).toHaveValue(
    '  IDLE-CASE  ',
  );
  await expect(modal.getByLabel('Nueva descripci\u00f3n de delito', { exact: true })).toHaveValue(
    'Descripci\u00f3n manual, sin cat\u00e1logo',
  );
  expect(writes).toEqual([]);
  const created = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === '/api/v1/penal-cases' &&
      response.request().method() === 'POST',
  );
  await modal.getByRole('button', { name: 'Crear expediente penal', exact: true }).click();
  expect((await created).status()).toBe(201);
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
  await page.getByRole('link', { name: 'Documentos', exact: true }).click();
  await page.getByRole('button', { name: 'Subir documento', exact: true }).first().click();
  const upload = page.getByRole('dialog', { name: 'Subir documento', exact: true });
  await upload.getByLabel('Archivo', { exact: true }).setInputFiles({
    name: 'idle-recovered.txt',
    mimeType: 'text/plain',
    buffer: Buffer.from('Idle retained content.\n'),
  });
  await upload
    .getByLabel('Clasificaci\u00f3n (opcional)', { exact: true })
    .fill('  Retained classification  ');
  await upload.getByLabel('Nueva etiqueta', { exact: true }).fill('  raw pending tag  ');
  await expect.poll(() => grants.length).toBe(2);
  await expired(page, upload, grants[1].access_token, request);
  expect(writes).toEqual(['/api/v1/penal-cases']);
  await login(page, fixture.recoveryCodes[2]);
  await page
    .getByRole('navigation')
    .getByRole('button', { name: 'Expedientes', exact: true })
    .click();
  await page.getByRole('button', { name: new RegExp(title) }).click();
  await page.getByRole('link', { name: 'Documentos', exact: true }).click();
  await page.getByRole('button', { name: 'Subir documento', exact: true }).first().click();
  await expect(upload.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true })).toHaveValue(
    '  Retained classification  ',
  );
  await expect(upload.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue(
    '  raw pending tag  ',
  );
  await expect(upload).toContainText('idle-recovered.txt');
  expect(writes).toEqual(['/api/v1/penal-cases']);
  await page.screenshot({ path: testInfo.outputPath('idle-restored-upload.png'), fullPage: true });
  const uploaded = page.waitForResponse(
    (response) =>
      /\/documents\/with-metadata$/.test(new URL(response.url()).pathname) &&
      response.request().method() === 'POST',
  );
  await upload.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  expect((await uploaded).status()).toBe(201);
  await expect(upload).toBeHidden();
  expect(writes).toHaveLength(2);
  expect(errors).toEqual([]);
});
