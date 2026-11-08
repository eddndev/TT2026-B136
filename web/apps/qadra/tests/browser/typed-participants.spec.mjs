import { test, expect } from '@playwright/test';
import {
  typedSetup,
  fillTyped,
  prepareTyped,
  typed,
  subject,
  subjectId,
} from './typed-participant-helpers.mjs';
import { detail, directory, openParticipant } from './participant-helpers.mjs';
test('creates an unsigned typed participant with exact supports and a reviewed identity', async ({
  page,
}, testInfo) => {
  const state = await typedSetup(page);
  const dialog = await fillTyped(page);
  await prepareTyped(dialog);
  await expect(
    dialog.getByText('Registro preparado sin firma personal', { exact: true }),
  ).toBeVisible();
  await dialog.evaluate((element) => element.scrollTo(0, 0));
  await page.screenshot({ path: testInfo.outputPath('typed-form-desktop.png') });
  await dialog.getByLabel('Motivo de CURP', { exact: true }).scrollIntoViewIfNeeded();
  await page.screenshot({ path: testInfo.outputPath('typed-form-fields-desktop.png') });
  await dialog.evaluate((element) => element.scrollTo(0, element.scrollHeight));
  await page.screenshot({ path: testInfo.outputPath('typed-form-review-desktop.png') });
  await dialog.getByRole('button', { name: 'Confirmar registro', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await expect(
    detail(page).getByRole('heading', { name: 'Persona tipificada', exact: true }),
  ).toBeVisible();
  const commit = state.calls.find((call) => call.path.endsWith('/proposals/commit'));
  expect(commit.body.signature_base64).toBeNull();
  expect(commit.body.prepared.proposal.values.role.profile.kind).toBe('defendant');
  expect(commit.body.prepared.proposal.subject.values.identity_support.locator).toBe('Pagina 1');
  await expect(directory(page)).not.toContainText('No consta en el soporte');
  await expect(directory(page)).toContainText('Imputado');
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({ path: testInfo.outputPath('typed-detail-desktop.png'), fullPage: true });
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.locator('body')).toHaveJSProperty('scrollWidth', 390);
  await page.screenshot({ path: testInfo.outputPath('typed-detail-mobile.png'), fullPage: true });
  await detail(page).getByRole('button', { name: 'Editar participante', exact: true }).click();
  const editor = page.getByRole('dialog', { name: 'Editar ficha tipificada', exact: true });
  const bounds = await editor
    .getByRole('button', { name: 'Revisar identidad y coincidencias', exact: true })
    .boundingBox();
  expect(bounds.x).toBeGreaterThanOrEqual(0);
  expect(bounds.x + bounds.width).toBeLessThanOrEqual(390);
  await editor.evaluate((element) => element.scrollTo(0, 0));
  await page.screenshot({ path: testInfo.outputPath('typed-form-mobile.png') });
  await editor
    .getByLabel('Motivo de Situaci\u00f3n de libertad declarada', { exact: true })
    .scrollIntoViewIfNeeded();
  await page.screenshot({ path: testInfo.outputPath('typed-form-fields-mobile.png') });
});
test('exports exact statement bytes and requires a separate signature for a judge', async ({
  page,
}, testInfo) => {
  const state = await typedSetup(page);
  const dialog = await fillTyped(page, 'control_judge');
  const choosingCertificate = page.waitForEvent('filechooser');
  await dialog.getByLabel('Certificado p\u00fablico PEM o DER').click();
  await (await choosingCertificate).setFiles('tests/fixtures/participant-public-certificate.pem');
  await dialog
    .locator('.participant-credential')
    .evaluate((element) => element.scrollIntoView({ block: 'center' }));
  await page.screenshot({ path: testInfo.outputPath('typed-certificate-desktop.png') });
  await prepareTyped(dialog);
  await expect(
    dialog.getByRole('button', { name: 'Confirmar registro', exact: true }),
  ).toBeDisabled();
  const download = page.waitForEvent('download');
  await dialog
    .getByRole('button', { name: 'Descargar declaraci\u00f3n binaria', exact: true })
    .click();
  const stream = await (await download).createReadStream(),
    chunks = [];
  for await (const chunk of stream) chunks.push(chunk);
  const bytes = Buffer.concat(chunks);
  expect(bytes.length).toBe(218);
  expect(bytes.subarray(0, 6).toString()).toBe('PCRED1');
  await dialog.getByLabel('Firma separada', { exact: true }).setInputFiles({
    name: 'declaration.sig',
    mimeType: 'application/octet-stream',
    buffer: Buffer.alloc(384, 7),
  });
  await expect(dialog).toContainText('declaration.sig / 384 bytes seleccionados');
  await dialog.evaluate((element) => element.scrollTo(0, element.scrollHeight));
  await page.screenshot({ path: testInfo.outputPath('typed-signature-desktop.png') });
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.locator('body')).toHaveJSProperty('scrollWidth', 390);
  await dialog.getByLabel('Firma separada', { exact: true }).scrollIntoViewIfNeeded();
  await page.screenshot({ path: testInfo.outputPath('typed-signature-mobile.png') });
  await dialog.getByLabel('Certificado p\u00fablico PEM o DER').scrollIntoViewIfNeeded();
  await page.screenshot({ path: testInfo.outputPath('typed-certificate-mobile.png') });
  await dialog.evaluate((element) => element.scrollTo(0, element.scrollHeight));
  await page.screenshot({ path: testInfo.outputPath('typed-signature-confirmation-mobile.png') });
  await dialog.getByRole('button', { name: 'Confirmar registro', exact: true }).click();
  await expect(dialog).not.toBeVisible();
  expect(
    Buffer.from(
      state.calls.find((call) => call.path.endsWith('/commit')).body.signature_base64,
      'base64',
    ),
  ).toEqual(Buffer.alloc(384, 7));
});
