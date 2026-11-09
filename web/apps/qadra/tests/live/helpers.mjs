import { readFileSync } from 'node:fs';
import { test, expect } from '@playwright/test';
import { watchApiFailures } from '../api-diagnostics.mjs';

export const fixture = JSON.parse(readFileSync(process.env.TT_WEB_FIXTURES, 'utf8'));

// Logout lives in the account menu of the top bar.
export async function signOut(page) {
  const trigger = page
    .getByRole('button', { name: 'Tu cuenta', exact: true })
    .filter({ visible: true });
  if ((await trigger.getAttribute('aria-expanded')) !== 'true') await trigger.click();
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
}
export async function login(page, recoveryCode, account = fixture) {
  watchApiFailures(page, test.info());
  await page.getByLabel('Correo electr\u00f3nico').fill(account.email);
  await page.getByLabel('Contrase\u00f1a', { exact: true }).fill(account.password);
  const [started] = await Promise.all([
    page.waitForResponse(
      (response) =>
        new URL(response.url()).pathname === '/api/v1/auth/login' &&
        response.request().method() === 'POST',
    ),
    page.getByRole('button', { name: 'Continuar', exact: true }).click(),
  ]);
  const submitted = started.request().postDataJSON();
  expect(submitted.email === account.email, 'login submitted the selected fixture email').toBe(
    true,
  );
  expect(submitted.password === account.password, 'login submitted the fixture password').toBe(
    true,
  );
  expect(started.status(), 'password login must succeed before MFA').toBe(200);
  await page.getByRole('button', { name: 'Usar c\u00f3digo de recuperaci\u00f3n' }).click();
  await page.getByLabel('C\u00f3digo de recuperaci\u00f3n', { exact: true }).fill(recoveryCode);
  const initial = new Map();
  const paths = ['/api/v1/dashboard', '/api/v1/document-integrity-incidents'];
  const matches = (response, path) =>
    new URL(response.url()).pathname === path && response.request().method() === 'GET';
  const observe = (response) => {
    const path = paths.find((candidate) => matches(response, candidate));
    if (path && !initial.has(path)) initial.set(path, response);
  };
  page.on('response', observe);
  try {
    const [verified] = await Promise.all([
      page.waitForResponse(
        (response) =>
          new URL(response.url()).pathname === '/api/v1/auth/mfa/recovery' &&
          response.request().method() === 'POST',
      ),
      page.getByRole('button', { name: 'Verificar y entrar' }).click(),
    ]);
    expect(verified.status()).toBe(200);
    // The authenticated role can differ from the account's original fixture.
    const role = (await verified.json()).user.role;
    expect(['owner', 'litigator', 'paralegal', 'client']).toContain(role);
    const required = ['owner', 'litigator'].includes(role) ? [paths[0]] : [];
    if (role === 'owner') required.push(paths[1]);
    for (const path of required) {
      const response = initial.get(path) ?? (await page.waitForResponse((r) => matches(r, path)));
      expect(response.status()).toBe(200);
      expect(await response.finished()).toBeNull();
    }
    await expect(
      page.getByRole('heading', {
        name: role === 'client' ? 'Mis contratos' : 'Tu mesa de trabajo',
      }),
    ).toBeVisible();
  } finally {
    page.off('response', observe);
  }
}

// A client account lands on its contracts, has no case navigation and every
// forced hash resolves back to its contracts.
export async function expectClientWorkspace(page, hashes = []) {
  const heading = page.getByRole('heading', { name: 'Mis contratos', exact: true });
  await expect(heading).toBeVisible();
  await expect(
    page
      .getByRole('navigation', { name: 'Navegaci\u00f3n principal' })
      .getByRole('button', { name: 'Expedientes', exact: true }),
  ).toHaveCount(0);
  for (const hash of hashes) {
    await page.evaluate((value) => {
      location.hash = value;
    }, hash);
    await expect(page).toHaveURL(/#contracts$/);
    await expect(heading).toBeVisible();
  }
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

// Sections of a page are links on wide screens and a select on phones.
export async function openSection(page, name) {
  // SectionTabs renders links (nav mode) or tabs (tabs mode), and a select on phones.
  const link = page
    .getByRole('link', { name, exact: true })
    .or(page.getByRole('tab', { name, exact: true }))
    .filter({ visible: true });
  const select = page.getByRole('combobox', { name: /^Secciones / }).filter({ visible: true });
  // Sections that do not fit the row wait in the menu at its end.
  const more = page.getByRole('button', { name: /^M\u00e1s secciones / }).filter({ visible: true });
  // Wait until the page shows one of them before choosing.
  await expect(link.or(select).or(more).first()).toBeVisible();
  if (await link.count()) return link.first().click();
  if (await select.count()) return select.selectOption({ label: name });
  await more.click();
  await link.first().click();
}
