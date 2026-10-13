import { expect } from '@playwright/test';
import { publicPem, signature } from '../fixtures/owner-certificates.mjs';

export const editor = (page) =>
  page.getByRole('region', { name: 'Registro de mi certificado', exact: true });
export const current = (page) =>
  page.getByRole('region', { name: 'V\u00ednculo de mi cuenta', exact: true });
export const register = (form) =>
  form.getByRole('button', { name: 'Registrar v\u00ednculo', exact: true });

export async function openOwnerCertificates(page) {
  if (await page.getByRole('button', { name: 'Abrir men\u00fa', exact: true }).isVisible())
    await page.getByRole('button', { name: 'Abrir men\u00fa', exact: true }).click();
  await page
    .getByRole('button', { name: 'Mi certificado', exact: true })
    .filter({ visible: true })
    .click();
  await expect(page.getByRole('heading', { name: 'Mi certificado', exact: true })).toBeVisible();
}

export async function beginOwnerRegistration(page) {
  await page.getByRole('button', { name: 'Registrar certificado', exact: true }).click();
  await expect(editor(page)).toBeVisible();
  return editor(page);
}

export async function selectPublicOwner(form) {
  await form.getByLabel('Certificado p\u00fablico PEM', { exact: true }).setInputFiles({
    name: 'owner-public.pem',
    mimeType: 'application/x-pem-file',
    buffer: publicPem,
  });
}

export async function prepareOwnerRegistration(page, state, existing = null) {
  const form = existing ?? (await beginOwnerRegistration(page));
  await selectPublicOwner(form);
  await form.getByRole('button', { name: 'Preparar v\u00ednculo', exact: true }).click();
  await expect(
    form.getByRole('button', { name: 'Descargar bytes para firma', exact: true }),
  ).toBeEnabled();
  const prepared = state.ownerPreparations.at(-1).prepared;
  await form.getByLabel('Firma separada', { exact: true }).setInputFiles({
    name: 'owner-external-signature.sig',
    mimeType: 'application/octet-stream',
    buffer: signature,
  });
  await expect(register(form)).toBeEnabled();
  return { form, prepared };
}

export async function continueOwnerRegistration(page) {
  await page.getByRole('button', { name: 'Continuar registro', exact: true }).click();
  await expect(editor(page)).toBeVisible();
  return editor(page);
}

export async function downloadedBytes(page, button) {
  const pending = page.waitForEvent('download');
  await button.click();
  const stream = await (await pending).createReadStream(),
    chunks = [];
  for await (const chunk of stream) chunks.push(chunk);
  return Buffer.concat(chunks);
}

export async function captureOwnerLayouts(page, testInfo, name) {
  const original = page.viewportSize();
  try {
    for (const [layout, width, height] of [
      ['desktop', 1440, 1000],
      ['mobile', 390, 844],
    ]) {
      await page.setViewportSize({ width, height });
      await page.evaluate(() => {
        document.activeElement?.blur();
        document.getElementById('main-content')?.focus({ preventScroll: true });
        window.scrollTo({ top: 0, left: 0, behavior: 'instant' });
        document
          .querySelector('.desktop-sidebar')
          ?.scrollTo({ top: 0, left: 0, behavior: 'instant' });
      });
      await page.screenshot({
        path: testInfo.outputPath(`${name}-${layout}.png`),
        fullPage: true,
        animations: 'disabled',
      });
    }
  } finally {
    if (original) await page.setViewportSize(original);
  }
}
