import { test, expect } from '@playwright/test';
import { setup, login } from './helpers.mjs';
import { visibility } from './session-visibility-helpers.mjs';
import {
  caseDraftSetup,
  checkCaseDraftRequests,
  createEditor,
  fillPartialCase,
  titleLabel,
  rawTitle,
} from './session-case-drafts-helpers.mjs';

const token = Buffer.alloc(32, 72).toString('base64url');
const heading = 'Elige una nueva contrase\u00f1a.';

test('a recovery link supersedes a hidden MFA session waiting for visibility', async ({ page }) => {
  const requests = await setup(page);
  await page.getByLabel('Correo electr\u00f3nico').fill('hatz@example.com');
  await page.getByLabel('Contrase\u00f1a', { exact: true }).fill('a-long-password');
  await page.getByRole('button', { name: 'Continuar', exact: true }).click();
  await page.getByLabel('C\u00f3digo de 6 d\u00edgitos', { exact: true }).fill('123456');
  await visibility(page, 'hidden');
  await page.getByRole('button', { name: 'Verificar y entrar' }).click();
  await expect(page.getByRole('status')).toContainText('Vuelve a esta pantalla');
  await page.evaluate((value) => {
    location.hash = `#password-reset=${value}`;
  }, token);
  await expect(page.getByRole('heading', { name: heading })).toBeVisible();
  expect(new URL(page.url()).hash).not.toContain(token);
  await visibility(page, 'visible');
  await expect(page.getByRole('heading', { name: heading })).toBeVisible();
  expect(requests.some((call) => call.path === '/api/v1/dashboard')).toBe(false);
  expect(requests.some((call) => call.path.includes('/password-reset/'))).toBe(false);
});

test('opening recovery in an active session captures its draft before reentry', async ({
  page,
}) => {
  const state = await caseDraftSetup(page);
  try {
    await login(page, false, false);
    await fillPartialCase(await createEditor(page));
    await page.evaluate((value) => {
      location.hash = `#password-reset=${value}`;
    }, token);
    await expect(page.getByRole('heading', { name: heading })).toBeVisible();
    expect(new URL(page.url()).hash).not.toContain(token);
    expect(state.confirmedWrites).toEqual([]);
    await page
      .getByRole('button', { name: 'Volver al inicio de sesi\u00f3n', exact: true })
      .click();
    await login(page, true, false);
    await expect((await createEditor(page)).getByLabel(titleLabel, { exact: true })).toHaveValue(
      rawTitle,
    );
    expect(state.confirmedWrites).toEqual([]);
    expect(state.calls.filter((call) => call.path.includes('/password-reset/'))).toEqual([]);
  } finally {
    await checkCaseDraftRequests(page);
  }
});

test('opening recovery invalidates an MFA response already in flight', async ({ page }) => {
  const requests = await setup(page);
  let release,
    entered = false;
  const held = new Promise((resolve) => {
    release = resolve;
  });
  await page.route('**/api/v1/auth/mfa/totp', async (route) => {
    entered = true;
    await held;
    await route.fallback();
  });
  try {
    await page.getByLabel('Correo electr\u00f3nico').fill('hatz@example.com');
    await page.getByLabel('Contrase\u00f1a', { exact: true }).fill('a-long-password');
    await page.getByRole('button', { name: 'Continuar', exact: true }).click();
    await page.getByLabel('C\u00f3digo de 6 d\u00edgitos', { exact: true }).fill('123456');
    await page.getByRole('button', { name: 'Verificar y entrar' }).click();
    await expect.poll(() => entered).toBe(true);
    await page.evaluate((value) => {
      location.hash = `#password-reset=${value}`;
    }, token);
    await expect(page.getByRole('heading', { name: heading })).toBeVisible();
    const response = page.waitForResponse('**/api/v1/auth/mfa/totp');
    release();
    await (await response).finished();
    await page.evaluate(
      () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
    );
    await expect(page.getByRole('heading', { name: heading })).toBeVisible();
    expect(requests.some((call) => call.path === '/api/v1/dashboard')).toBe(false);
    await page
      .getByRole('button', { name: 'Volver al inicio de sesi\u00f3n', exact: true })
      .click();
    await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Continuar', exact: true })).toBeEnabled();
  } finally {
    release();
  }
});
