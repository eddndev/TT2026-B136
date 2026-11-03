import { test, expect } from '@playwright/test';
import { verify } from 'node:crypto';
import { contractSetup, clientLogin, openRequest } from './contract-helpers.mjs';
import { inputs } from '../contract-fixture.mjs';
import { localSignatureMaterial, signLocally } from './local-signature-material.mjs';

// The full Chromium headless mode shows embedded PDFs without forced downloads.
test.use({ channel: 'chromium' });

test('a client sees only its contracts, its signature and the evidence validator', async ({
  page,
}) => {
  await contractSetup(page, 'client', localSignatureMaterial());
  await clientLogin(page);
  await expect(page.getByRole('heading', { name: 'Mis contratos', exact: true })).toBeVisible();
  const nav = page.locator('.desktop-sidebar').getByRole('navigation');
  await expect(nav.getByRole('button')).toHaveText([
    'Mis contratos',
    'Mi firma',
    'Validar evidencia',
  ]);
  await expect(page.getByRole('button', { name: 'Gu\u00eda de uso' })).toHaveCount(0);
  for (const hash of ['#cases', '#documents', '#case-summary', '#overview', '#agenda']) {
    await page.evaluate((value) => (location.hash = value), hash);
    await expect(page.getByRole('heading', { name: 'Mis contratos', exact: true })).toBeVisible();
  }
  await page.evaluate(() => (location.hash = '#evidence-validation'));
  await expect(page.getByRole('heading', { name: 'Validar evidencia', exact: true })).toBeVisible();
  await page.route(`**/api/v1/contract-requests/${inputs.request_id}`, (route) =>
    route.fulfill({ status: 404, json: { error: { code: 'contract_not_found' } } }),
  );
  await page.evaluate(() => (location.hash = '#contracts'));
  await page.getByRole('button', { name: 'Abrir Contrato de prestacion de servicios' }).click();
  await expect(page.getByRole('alert')).toContainText('no est\u00e1 disponible');
});

test('a client reviews, confirms and signs in three steps with a local key', async ({ page }) => {
  const material = localSignatureMaterial();
  const state = await contractSetup(page, 'client', material);
  await clientLogin(page);
  const region = await openRequest(page, 'Mis contratos');
  await expect(region.getByText('Litigante Demo')).toBeVisible();
  await expect(region.getByText(state.detail.firm_certificate_fingerprint)).toBeVisible();
  const consent = region.getByRole('checkbox');
  await expect(consent).toBeDisabled();
  await expect(region.getByRole('button', { name: 'Enviar firma' })).toHaveCount(0);
  expect(state.writes).toEqual([]);
  await region.getByRole('button', { name: 'Cargar y comprobar el PDF' }).click();
  await expect(region.getByText('PDF comprobado')).toBeVisible();
  await expect(region.locator('iframe[title="PDF exacto del contrato"]')).toBeVisible();
  // The browser PDF viewer loads only in a frame without sandbox.
  await expect
    .poll(() => page.frames().some((frame) => frame.url().startsWith('chrome-extension://')))
    .toBe(true);
  await expect(region.getByLabel(/Tu firma solo formaliza el contrato/)).toBeVisible();
  await consent.check();
  const local = region.getByRole('region', { name: /^Firma local:/ });
  await expect(local).toHaveCount(1);
  expect(state.writes.map((row) => row.path.split('/').at(-1))).toEqual(['prepare']);
  await signLocally(local, material);
  await expect(local.getByText('Firma local preparada')).toBeVisible();
  expect(state.writes.filter((row) => row.path.endsWith('/sign'))).toEqual([]);
  await region.getByRole('button', { name: 'Enviar firma' }).click();
  await expect(
    region.getByText(
      'Tu firma qued\u00f3 registrada; el contrato a\u00fan no est\u00e1 formalizado.',
    ),
  ).toBeVisible();
  const sent = state.writes.find((row) => row.path.endsWith('/sign')).body;
  expect(sent.statement_base64).toBe(state.prepared.statement_base64);
  expect(
    verify(
      'sha256',
      Buffer.from(sent.statement_base64, 'base64'),
      material.publicKey,
      Buffer.from(sent.signature_base64, 'base64'),
    ),
  ).toBe(true);
  const downloaded = page.waitForEvent('download');
  await region.getByRole('button', { name: 'Descargar acuse parcial' }).click();
  expect((await downloaded).suggestedFilename()).toBe('contrato-acuse-parcial.zip');
  await expect(region.getByRole('button', { name: 'Rechazar contrato' })).toHaveCount(0);
});

test('a client rejects a contract only after a separate confirmation', async ({ page }) => {
  const state = await contractSetup(page, 'client', localSignatureMaterial());
  await clientLogin(page);
  const region = await openRequest(page, 'Mis contratos');
  await region.getByRole('button', { name: 'Rechazar contrato' }).click();
  expect(state.writes).toEqual([]);
  await region.getByLabel('Motivo (opcional)').fill('No estoy de acuerdo');
  await region.getByRole('button', { name: 'Confirmar rechazo' }).click();
  await expect(region.getByText('Rechazaste el contrato.')).toBeVisible();
  const closed = state.writes.find((row) => row.path.endsWith('/close')).body;
  expect(closed.outcome).toBe('reject');
  expect(closed.reason).toBe('No estoy de acuerdo');
  await expect(region.getByText('Rechazado por el Cliente')).toBeVisible();
});

test('a client without a binding is guided to its signature before signing', async ({ page }) => {
  await contractSetup(page, 'client', localSignatureMaterial(), { binding: null });
  await clientLogin(page);
  const region = await openRequest(page, 'Mis contratos');
  await expect(
    region.getByRole('heading', { name: 'Antes de firmar, prepara tu firma' }),
  ).toBeVisible();
  await expect(region.getByRole('button', { name: 'Cargar y comprobar el PDF' })).toHaveCount(0);
  await region.getByRole('button', { name: 'Ir a Mi firma' }).click();
  await expect(page.getByRole('heading', { name: 'Mi firma', exact: true })).toBeVisible();
  await expect(page.getByLabel('Organizaci\u00f3n (O)')).toHaveAttribute('readonly', '');
});
