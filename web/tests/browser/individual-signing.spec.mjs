import { test, expect } from '@playwright/test';
import { login, navigate, selectCase, openDocument } from './helpers.mjs';
import { signingSetup, signature, certificate, pdf } from './individual-signing-fixture.mjs';
const detached = {
  name: 'external.sig',
  mimeType: 'application/octet-stream',
  buffer: Buffer.from(signature, 'base64'),
};
async function bytes(download) {
  const stream = await download.createReadStream(),
    chunks = [];
  for await (const chunk of stream) chunks.push(chunk);
  return Buffer.concat(chunks);
}
async function prepareBinding(page, state) {
  if (await page.getByRole('button', { name: 'Abrir men\u00fa', exact: true }).isVisible())
    await page.getByRole('button', { name: 'Abrir men\u00fa', exact: true }).click();
  await page.getByRole('button', { name: 'Mi firma documental', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Mi firma documental' })).toBeVisible();
  await page.getByLabel('Certificado publico PEM').setInputFiles({
    name: 'private.key',
    mimeType: 'application/octet-stream',
    buffer: Buffer.from('-----BEGIN PRIVATE KEY-----'),
  });
  await expect(page.getByRole('alert')).toBeVisible();
  expect(state.writes).toHaveLength(0);
  await page.getByLabel('Certificado publico PEM').setInputFiles({
    name: 'public.pem',
    mimeType: 'application/x-pem-file',
    buffer: Buffer.from(
      '-----BEGIN CERTIFICATE-----\n' +
        certificate.toString('base64') +
        '\n-----END CERTIFICATE-----\n',
    ),
  });
  await page.getByRole('button', { name: 'Preparar vinculo', exact: true }).click();
  const downloaded = page.waitForEvent('download');
  await page
    .getByRole('button', { name: 'Descargar declaracion del vinculo', exact: true })
    .click();
  expect((await bytes(await downloaded)).subarray(0, 8).toString()).toBe('DOCBIND1');
}
async function register(page, state) {
  await prepareBinding(page, state);
  await page.getByLabel('Firma externa del vinculo').setInputFiles(detached);
  await page.getByLabel('Confirmo vincular este certificado a mi cuenta').check();
  state.lostBinding = true;
  await page.getByRole('button', { name: 'Registrar vinculo', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Consultar recibo del vinculo', exact: true }),
  ).toBeVisible();
  expect(state.writes.filter((row) => row.path.endsWith('/register'))).toHaveLength(1);
  await page.getByRole('button', { name: 'Consultar recibo del vinculo', exact: true }).click();
  await expect(page.getByText('Vinculo registrado', { exact: true })).toBeVisible();
}

async function prepareDocument(page) {
  await selectCase(page);
  await openDocument(page);
  await page.getByRole('button', { name: 'Firmar personalmente', exact: true }).click();
  const downloaded = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Descargar PDF para firmar', exact: true }).click();
  expect(await bytes(await downloaded)).toEqual(pdf);
  await expect(page.getByTitle('PDF exacto de esta version')).toBeVisible();
  await page.getByLabel('He revisado este PDF y confirmo su contenido').check();
  await page.getByLabel('Firma externa del PDF').setInputFiles(detached);
  await page.getByRole('button', { name: 'Preparar consentimiento', exact: true }).click();
  const consentDownload = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Descargar consentimiento', exact: true }).click();
  expect((await bytes(await consentDownload)).subarray(0, 8).toString()).toBe('DOCSIGN1');
  await page.getByLabel('Firma externa del consentimiento').setInputFiles(detached);
  await page.getByLabel('Confirmo firmar esta version con mi certificado').check();
}

test('litigator binds public proof, reviews exact PDF and recovers a single accepted operation', async ({
  page,
}) => {
  const state = await signingSetup(page);
  await login(page, false, false);
  await register(page, state);
  await prepareDocument(page);
  state.lostDocument = true;
  await page.getByRole('button', { name: 'Enviar firma personal', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Consultar recibo de firma', exact: true }),
  ).toBeVisible();
  expect(state.writes.filter((row) => row.path.endsWith('/submit'))).toHaveLength(1);
  await navigate(page, 'Inicio');
  await selectCase(page);
  await openDocument(page);
  await page.getByRole('button', { name: 'Recuperar intento de firma', exact: true }).click();
  expect(state.writes.filter((row) => row.path.endsWith('/submit'))).toHaveLength(1);
  await page.getByRole('button', { name: 'Consultar recibo de firma', exact: true }).click();
  await expect(page.getByText('Firma personal aceptada', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Consultar estado de la firma', exact: true }).click();
  await expect(page.getByText('Validez historica: valida', { exact: true })).toBeVisible();
  await expect(
    page.getByText('Estado actual del certificado: rechazado', { exact: true }),
  ).toBeVisible();
  await expect(page.getByText('Vinculo retirado posteriormente', { exact: true })).toBeVisible();
  expect(state.writes.filter((row) => row.path.endsWith('/submit'))).toHaveLength(1);
});

for (const role of ['owner', 'paralegal'])
  test(role + ' never offers personal signing', async ({ page }) => {
    await signingSetup(page, role);
    await login(page);
    await openDocument(page);
    await expect(
      page.getByRole('button', { name: 'Firmar personalmente', exact: true }),
    ).toHaveCount(0);
    await expect(
      page.getByRole('button', { name: 'Mi firma documental', exact: true }),
    ).toHaveCount(0);
  });

test('mobile certificate withdrawal requires its own confirmation and keeps its receipt', async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const state = await signingSetup(page);
  await login(page, false, false);
  await register(page, state);
  await page.getByRole('button', { name: 'Retirar vinculo', exact: true }).click();
  expect(state.writes.filter((row) => row.path.endsWith('/withdraw'))).toHaveLength(0);
  await page.getByLabel('Confirmo retirar el vinculo para futuras firmas').check();
  await page.getByRole('button', { name: 'Confirmar retiro', exact: true }).click();
  await expect(page.getByText('Vinculo retirado', { exact: true })).toBeVisible();
  const writes = state.writes.filter((row) => row.path.endsWith('/withdraw'));
  expect(writes).toHaveLength(1);
  expect(writes[0].body).toBeNull();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test('explicit credential rejection clears confirmation and allows a corrected external proof', async ({
  page,
}) => {
  const state = await signingSetup(page);
  await login(page, false, false);
  await prepareBinding(page, state);
  let attempts = 0;
  await page.route('**/identity/signing-certificates/*/register', async (route) => {
    attempts++;
    if (attempts === 1)
      return route.fulfill({
        status: 422,
        json: { error: { code: 'signing_credential_rejected' } },
      });
    return route.fallback();
  });
  await page.getByLabel('Firma externa del vinculo').setInputFiles(detached);
  await page.getByLabel('Confirmo vincular este certificado a mi cuenta').check();
  await page.getByRole('button', { name: 'Registrar vinculo', exact: true }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(page.getByLabel('Firma externa del vinculo')).toBeEnabled();
  await expect(page.getByLabel('Confirmo vincular este certificado a mi cuenta')).not.toBeChecked();
  await expect(page.getByRole('button', { name: 'Registrar vinculo', exact: true })).toBeDisabled();
  expect(attempts).toBe(1);
  await page.getByLabel('Firma externa del vinculo').setInputFiles(detached);
  await page.getByLabel('Confirmo vincular este certificado a mi cuenta').check();
  await page.getByRole('button', { name: 'Registrar vinculo', exact: true }).click();
  await expect(page.getByText('Vinculo registrado', { exact: true })).toBeVisible();
  expect(attempts).toBe(2);
});

test('explicit document credential rejection requires a corrected proof and renewed confirmation', async ({
  page,
}) => {
  const state = await signingSetup(page);
  await login(page, false, false);
  await register(page, state);
  await prepareDocument(page);
  let attempts = 0;
  await page.route('**/individual-signing/submit', async (route) => {
    attempts++;
    if (attempts === 1)
      return route.fulfill({
        status: 422,
        json: { error: { code: 'signing_credential_rejected' } },
      });
    return route.fallback();
  });
  await page.getByRole('button', { name: 'Enviar firma personal', exact: true }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(page.getByLabel('Firma externa del consentimiento')).toBeEnabled();
  await expect(
    page.getByLabel('Confirmo firmar esta version con mi certificado'),
  ).not.toBeChecked();
  await expect(
    page.getByRole('button', { name: 'Enviar firma personal', exact: true }),
  ).toBeDisabled();
  expect(attempts).toBe(1);
  await page.getByLabel('Firma externa del consentimiento').setInputFiles(detached);
  await page.getByLabel('Confirmo firmar esta version con mi certificado').check();
  await page.getByRole('button', { name: 'Enviar firma personal', exact: true }).click();
  await expect(page.getByText('Firma personal aceptada', { exact: true })).toBeVisible();
  expect(attempts).toBe(2);
});
