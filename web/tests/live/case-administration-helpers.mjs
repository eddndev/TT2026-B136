import { expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { fixture, loginAs } from './helpers.mjs';
export const accounts = fixture.caseAdministration;
export async function navigate(page, name) {
  const menu = page.getByRole('button', { name: 'Abrir men\u00fa', exact: true });
  if (await menu.isVisible()) await menu.click();
  await page
    .getByRole('navigation', { name: 'Navegaci\u00f3n principal' })
    .getByRole('button', { name, exact: true })
    .click();
}
export async function fillProfile(page, prefix) {
  for (const [label, value] of [
    ['NUC', `${prefix}-NUC`],
    ['Autoridad emisora del NUC', 'Fiscalia registrada'],
    ['Carpeta judicial', `${prefix}-CJ`],
    ['\u00d3rgano emisor de la carpeta', 'Organo registrado'],
  ])
    await page.getByLabel(label, { exact: true }).fill(value);
  await page
    .getByLabel('Nueva descripci\u00f3n de delito', { exact: true })
    .fill('Descripci\u00f3n manual, sin cat\u00e1logo');
  await page
    .getByLabel('Informaci\u00f3n general (opcional)', { exact: true })
    .fill('Primera linea\nSegunda linea');
}
export async function createPenal(page, title, prefix) {
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: 'Nuevo expediente penal', exact: true }).click();
  await page.getByLabel('T\u00edtulo del expediente', { exact: true }).fill(title);
  await page.getByLabel('Referencia interna', { exact: true }).fill(prefix);
  await fillProfile(page, prefix);
  const response = page.waitForResponse(
    (r) => r.url().endsWith('/api/v1/penal-cases') && r.request().method() === 'POST',
  );
  await page.getByRole('button', { name: 'Crear expediente penal', exact: true }).click();
  expect((await response).status()).toBe(201);
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
  return (await response).json();
}
export async function openCase(page, record = accounts.basicCase) {
  await navigate(page, 'Expedientes');
  await page
    .getByRole('combobox', { name: 'Estado administrativo', exact: true })
    .selectOption('all');
  await page.getByRole('button', { name: 'Buscar expedientes', exact: true }).click();
  await page.getByRole('button', { name: new RegExp(record.title) }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
}
export async function capture(page, testInfo, name, locator) {
  await page.evaluate(() => {
    document.activeElement?.blur();
    scrollTo(0, 0);
  });
  await page.screenshot({ path: testInfo.outputPath(`${name}.png`), fullPage: true });
  if (locator) {
    await locator.scrollIntoViewIfNeeded();
    await page.screenshot({ path: testInfo.outputPath(`${name}-detail.png`) });
  }
}
export async function archive(page, testInfo, name) {
  const promise = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Descargar evidencia', exact: true }).click();
  const path = testInfo.outputPath(name);
  await (await promise).saveAs(path);
  return readFileSync(path);
}
export async function revoke() {
  let token;
  const call = async (method, path, data) => {
    const result = await fetch(`${process.env.API_PROXY_TARGET}/api/v1${path}`, {
      method,
      headers: {
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
        ...(data ? { 'Content-Type': 'application/json' } : {}),
      },
      body: data ? JSON.stringify(data) : undefined,
    });
    expect(result.ok).toBe(true);
    return result.status === 204 ? null : result.json();
  };
  const challenge = await call('POST', '/auth/login', {
    email: accounts.owner.email,
    password: accounts.owner.password,
  });
  const session = await call('POST', '/auth/mfa/recovery', {
    challenge_token: challenge.challenge_token,
    code: accounts.owner.recoveryCodes[3],
  });
  token = session.access_token;
  try {
    await call('DELETE', `/cases/${accounts.basicCase.id}/members/${accounts.litigator.id}`);
  } finally {
    await call('POST', '/auth/logout');
  }
}
export async function permissions(browser, testInfo, errors) {
  const contexts = [];
  let litigator;
  try {
    for (const role of ['litigator', 'paralegal', 'client']) {
      const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
      contexts.push(context);
      const page = await context.newPage();
      const paths = [];
      page.on('pageerror', (error) => errors.push(error.message));
      page.on('request', (request) => {
        const path = new URL(request.url()).pathname;
        if (path.startsWith('/api/v1/')) paths.push(path);
      });
      await page.goto('/');
      await loginAs(page, accounts[role], 0);
      await navigate(page, 'Expedientes');
      await expect(
        page.getByRole('button', { name: new RegExp(accounts.hiddenCase.title) }),
      ).toHaveCount(0);
      await page.getByRole('button', { name: new RegExp(accounts.basicCase.title) }).click();
      await expect(
        page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
      ).toBeVisible();
      if (role === 'client') {
        expect(
          paths.filter((path) => /case-administrations|\/administration(?:\/|$)/.test(path)),
        ).toEqual([]);
        await expect(page.getByText('Ficha penal completa', { exact: true })).toHaveCount(0);
      } else if (role === 'litigator') {
        litigator = page;
        await page.getByRole('button', { name: 'Editar ficha penal', exact: true }).click();
        await page
          .getByLabel('Informaci\u00f3n general (opcional)', { exact: true })
          .fill('Nota del litigante');
        await page.getByRole('button', { name: 'Guardar ficha penal', exact: true }).click();
        await expect(
          page.locator('.case-summary').getByText('Nota del litigante', { exact: true }),
        ).toBeVisible();
      } else {
        await expect(
          page.getByRole('button', { name: 'Editar ficha penal', exact: true }),
        ).toHaveCount(0);
        await expect(
          page.locator('.case-summary').getByText('Nota del litigante', { exact: true }),
        ).toBeVisible();
        await page
          .getByRole('button', { name: 'Ver historial administrativo', exact: true })
          .click();
        await expect(page.locator('.case-history details')).toHaveCount(7);
      }
    }
    await revoke();
    await litigator.getByRole('button', { name: 'Actualizar resumen', exact: true }).click();
    await expect(
      litigator.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
    ).toBeVisible();
    await expect(litigator.locator('.case-summary')).toHaveCount(0);
  } finally {
    for (const context of contexts) await context.close();
  }
}
