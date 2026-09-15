import { readFileSync } from 'node:fs';
import { expect } from '@playwright/test';

export const fixture = JSON.parse(readFileSync(process.env.TT_WEB_FIXTURES, 'utf8'));

export async function login(page, recoveryCode, account = fixture) {
  await page.getByLabel('Correo electr\u00f3nico').fill(account.email);
  await page.getByLabel('Contrase\u00f1a', { exact: true }).fill(account.password);
  await page.getByRole('button', { name: 'Continuar', exact: true }).click();
  await page.getByRole('button', { name: 'Usar c\u00f3digo de recuperaci\u00f3n' }).click();
  await page.getByLabel('C\u00f3digo de recuperaci\u00f3n', { exact: true }).fill(recoveryCode);
  await page.getByRole('button', { name: 'Verificar y entrar' }).click();
  await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
}

export async function capture(page, testInfo, name) {
  await page.evaluate(() => {
    document.activeElement?.blur();
    window.scrollTo(0, 0);
  });
  await page.screenshot({ path: testInfo.outputPath(`${name}.png`), fullPage: true });
  await page
    .locator('.document-focus')
    .evaluate((element) => element.scrollIntoView({ block: 'start' }));
  const skipLink = await page.getByRole('link', { name: 'Saltar al contenido' }).boundingBox();
  expect(skipLink.y + skipLink.height).toBeLessThanOrEqual(0);
  await page.screenshot({ path: testInfo.outputPath(`${name}-detail.png`) });
}

export async function loginAs(page, account, recoveryIndex) {
  return login(page, account.recoveryCodes[recoveryIndex], account);
}
