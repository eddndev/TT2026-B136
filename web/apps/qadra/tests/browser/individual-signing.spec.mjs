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
  await page.getByRole('button', { name: 'Mi firma digital', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Mi firma digital' })).toBeVisible();
  await page.getByLabel('Certificado p\u00fablico PEM').setInputFiles({
    name: 'private.key',
    mimeType: 'application/octet-stream',
    buffer: Buffer.from('-----BEGIN PRIVATE KEY-----'),
  });
  await expect(page.getByRole('alert')).toBeVisible();
  expect(state.writes).toHaveLength(0);
  await page.getByLabel('Certificado p\u00fablico PEM').setInputFiles({
    name: 'public.pem',
    mimeType: 'application/x-pem-file',
    buffer: Buffer.from(
      '-----BEGIN CERTIFICATE-----\n' +
        certificate.toString('base64') +
        '\n-----END CERTIFICATE-----\n',
    ),
  });
  await page.getByRole('button', { name: 'Preparar v\u00ednculo', exact: true }).click();
  const downloaded = page.waitForEvent('download');
  await page
    .getByRole('button', { name: 'Descargar declaraci\u00f3n del v\u00ednculo', exact: true })
    .click();
  expect((await bytes(await downloaded)).subarray(0, 8).toString()).toBe('DOCBIND1');
}
async function register(page, state) {
  await prepareBinding(page, state);
  await page.getByLabel('Firma externa del v\u00ednculo').setInputFiles(detached);
  await page.getByLabel('Confirmo vincular este certificado a mi cuenta').check();
  state.lostBinding = true;
  await page.getByRole('button', { name: 'Registrar v\u00ednculo', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Consultar recibo del v\u00ednculo', exact: true }),
  ).toBeVisible();
  expect(state.writes.filter((row) => row.path.endsWith('/register'))).toHaveLength(1);
  await page
    .getByRole('button', { name: 'Consultar recibo del v\u00ednculo', exact: true })
    .click();
  await expect(page.getByText('V\u00ednculo registrado', { exact: true })).toBeVisible();
}

async function prepareDocument(page) {
  await selectCase(page);
  await openDocument(page);
  await page.getByRole('button', { name: 'Firmar personalmente', exact: true }).click();
  const downloaded = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Descargar PDF para firmar', exact: true }).click();
  expect(await bytes(await downloaded)).toEqual(pdf);
  await expect(page.getByTitle('PDF exacto de esta versi\u00f3n')).toBeVisible();
  await page.getByLabel('He revisado este PDF y confirmo su contenido').check();
  await page.getByLabel('Firma externa del PDF').setInputFiles(detached);
  await page.getByRole('button', { name: 'Preparar consentimiento', exact: true }).click();
  const consentDownload = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Descargar consentimiento', exact: true }).click();
  expect((await bytes(await consentDownload)).subarray(0, 8).toString()).toBe('DOCSIGN1');
  await page.getByLabel('Firma externa del consentimiento').setInputFiles(detached);
  await page.getByLabel('Confirmo firmar esta versi\u00f3n con mi certificado').check();
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
  await expect(
    page.getByText('Validez hist\u00f3rica: v\u00e1lida', { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText('Estado actual del certificado: rechazado', { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText('V\u00ednculo retirado posteriormente', { exact: true }),
  ).toBeVisible();
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
    // An Owner may bind a document-signing certificate to sign contracts,
    // but never signs a document individually.
    await expect(page.getByRole('button', { name: 'Mi firma digital', exact: true })).toHaveCount(
      role === 'owner' ? 1 : 0,
    );
  });

test('mobile certificate withdrawal requires its own confirmation and keeps its receipt', async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const state = await signingSetup(page);
  await login(page, false, false);
  await register(page, state);
  await page.getByRole('button', { name: 'Retirar v\u00ednculo', exact: true }).click();
  expect(state.writes.filter((row) => row.path.endsWith('/withdraw'))).toHaveLength(0);
  await page.getByLabel('Confirmo retirar el v\u00ednculo para futuras firmas').check();
  await page.getByRole('button', { name: 'Confirmar retiro', exact: true }).click();
  await expect(page.getByText('V\u00ednculo retirado', { exact: true })).toBeVisible();
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
  await page.getByLabel('Firma externa del v\u00ednculo').setInputFiles(detached);
  await page.getByLabel('Confirmo vincular este certificado a mi cuenta').check();
  await page.getByRole('button', { name: 'Registrar v\u00ednculo', exact: true }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(page.getByLabel('Firma externa del v\u00ednculo')).toBeEnabled();
  await expect(page.getByLabel('Confirmo vincular este certificado a mi cuenta')).not.toBeChecked();
  await expect(
    page.getByRole('button', { name: 'Registrar v\u00ednculo', exact: true }),
  ).toBeDisabled();
  expect(attempts).toBe(1);
  await page.getByLabel('Firma externa del v\u00ednculo').setInputFiles(detached);
  await page.getByLabel('Confirmo vincular este certificado a mi cuenta').check();
  await page.getByRole('button', { name: 'Registrar v\u00ednculo', exact: true }).click();
  await expect(page.getByText('V\u00ednculo registrado', { exact: true })).toBeVisible();
  expect(attempts).toBe(2);
});
