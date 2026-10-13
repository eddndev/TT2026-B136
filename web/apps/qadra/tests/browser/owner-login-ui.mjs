import { expect } from '@playwright/test';
import { publicReceipt, signature } from './owner-login-fixtures.mjs';

export const panel = (page) =>
  page.getByRole('region', { name: 'Acceso con certificado', exact: true });
export const prepare = (page) =>
  panel(page).getByRole('button', { name: 'Preparar acceso', exact: true });
export const proof = (page) =>
  panel(page).getByRole('button', { name: 'Comprobar firma', exact: true });
export const again = (page) =>
  panel(page).getByRole('button', { name: 'Preparar otro intento', exact: true });
export const download = (page) =>
  panel(page).getByRole('button', { name: 'Descargar bytes de acceso', exact: true });
export const signatureInput = (page) =>
  panel(page).getByLabel('Firma separada de acceso', { exact: true });
export const back = (page) =>
  page.getByRole('button', { name: 'Volver a contrase\u00f1a', exact: true });

export async function openOwnerLogin(page) {
  await page.getByRole('button', { name: 'Ingresar con certificado', exact: true }).click();
  await expect(panel(page)).toBeVisible();
}

export async function selectReceipt(page, value = publicReceipt(), name = 'public-receipt.json') {
  await panel(page)
    .getByLabel('Recibo p\u00fablico del v\u00ednculo', { exact: true })
    .setInputFiles({
      name,
      mimeType: 'application/json',
      buffer: Buffer.from(JSON.stringify(value, null, 2)),
    });
}

export async function prepareLogin(page) {
  await selectReceipt(page);
  await expect(prepare(page)).toBeEnabled();
  await prepare(page).click();
  await expect(download(page)).toBeEnabled();
}

export async function selectSignature(page) {
  await signatureInput(page).setInputFiles({
    name: 'external-login-signature.sig',
    mimeType: 'application/octet-stream',
    buffer: Buffer.from(signature, 'base64'),
  });
  await expect(proof(page)).toBeEnabled();
}

export async function enterLoginMfa(page, recovery = false) {
  await expect(page.getByRole('heading', { name: 'Un paso m\u00e1s.', exact: true })).toBeVisible();
  if (recovery)
    await page
      .getByRole('button', { name: 'Usar c\u00f3digo de recuperaci\u00f3n', exact: true })
      .click();
  await page
    .getByLabel(recovery ? 'C\u00f3digo de recuperaci\u00f3n' : 'C\u00f3digo de 6 d\u00edgitos', {
      exact: true,
    })
    .fill(recovery ? 'recovery-one' : '123456');
  await page.getByRole('button', { name: 'Verificar y entrar', exact: true }).click();
}

export async function settled(page) {
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
}

export async function assertNoStoredAttempt(page, values) {
  const stored = await page.evaluate(() =>
    JSON.stringify([Object.entries(localStorage), Object.entries(sessionStorage), location.href]),
  );
  for (const value of values) expect(stored).not.toContain(value);
}
