import { expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { fixture, loginAs } from './helpers.mjs';
export const accounts = fixture.participants;
export const directory = (page) =>
  page.getByRole('region', { name: 'Directorio del expediente', exact: true });
export const detail = (page) =>
  page.getByRole('region', { name: 'Datos del participante', exact: true });
export async function openCase(page) {
  await page
    .getByRole('navigation', { name: 'Navegaci\u00f3n principal' })
    .getByRole('button', { name: 'Expedientes', exact: true })
    .click();
  await page.getByRole('button', { name: new RegExp(accounts.case.title) }).click();
}
export async function openPerson(page, name) {
  await directory(page)
    .getByRole('button', { name: `Abrir ${name}`, exact: true })
    .click();
  await expect(detail(page).getByRole('heading', { name, exact: true })).toBeVisible();
}
export async function edit(page) {
  await page.getByRole('button', { name: 'Editar participante', exact: true }).click();
  return page.getByRole('dialog', { name: 'Editar participante', exact: true });
}
export async function capture(page, testInfo, name) {
  await page.evaluate(() => {
    document.activeElement?.blur();
    window.scrollTo(0, 0);
  });
  await page.screenshot({ path: testInfo.outputPath(`${name}.png`), fullPage: true });
  await detail(page).evaluate((element) => element.scrollIntoView({ block: 'start' }));
  await page.screenshot({ path: testInfo.outputPath(`${name}-detail.png`) });
}
export async function archive(page, testInfo, name) {
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Descargar evidencia', exact: true }).click();
  const path = testInfo.outputPath(name);
  await (await download).saveAs(path);
  return readFileSync(path);
}
async function revokeLitigator() {
  const base = process.env.API_PROXY_TARGET;
  let token;
  async function call(method, path, data) {
    const response = await fetch(`${base}/api/v1${path}`, {
      method,
      headers: {
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
        ...(data ? { 'Content-Type': 'application/json' } : {}),
      },
      body: data ? JSON.stringify(data) : undefined,
    });
    expect(response.ok).toBe(true);
    return response.status === 204 ? null : response.json();
  }
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
    await call('DELETE', `/cases/${accounts.case.id}/members/${accounts.litigator.id}`);
  } finally {
    await call('POST', '/auth/logout');
  }
}
export async function exercisePermissions(browser, testInfo, name, errors) {
  const contexts = [];
  let litigator;
  try {
    for (const role of ['litigator', 'paralegal', 'client']) {
      const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
      contexts.push(context);
      const page = await context.newPage();
      page.on('pageerror', (error) => errors.push(error.message));
      const paths = [];
      page.on('request', (request) => {
        const path = new URL(request.url()).pathname;
        if (path.startsWith('/api/v1/')) paths.push(path);
      });
      await page.goto('/');
      await loginAs(page, accounts[role], 0);
      await page
        .getByRole('navigation', { name: 'Navegaci\u00f3n principal' })
        .getByRole('button', { name: 'Expedientes', exact: true })
        .click();
      await expect(
        page.getByRole('button', { name: new RegExp(accounts.hiddenCase.title) }),
      ).toHaveCount(0);
      await page.getByRole('button', { name: new RegExp(accounts.case.title) }).click();
      if (role === 'client') {
        await expect(page.getByRole('link', { name: 'Participantes', exact: true })).toHaveCount(0);
        await page.evaluate(() => {
          location.hash = 'participants';
        });
        await expect(
          page.getByRole('heading', { name: 'Tu mesa de trabajo', exact: true }),
        ).toBeVisible();
        expect(
          paths.filter((path) => /^\/api\/v1\/cases\/[^/]+\/participants(?:\/|$)/.test(path)),
        ).toEqual([]);
      } else {
        await page.getByRole('link', { name: 'Participantes', exact: true }).click();
        await openPerson(page, name);
        expect(
          paths.some((path) => /^\/api\/v1\/cases\/[^/]+\/participants(?:\/|$)/.test(path)),
        ).toBe(true);
        if (role === 'litigator') {
          litigator = page;
          const modal = await edit(page);
          await modal
            .getByLabel('Situaci\u00f3n jur\u00eddica registrada (opcional)', { exact: true })
            .fill('Nota del litigante');
          await modal.getByRole('button', { name: 'Guardar participante', exact: true }).click();
          await expect(detail(page).getByText('Revisi\u00f3n 8', { exact: true })).toBeVisible();
        } else {
          await expect(
            page.getByRole('button', { name: 'Agregar participante', exact: true }),
          ).toHaveCount(0);
          await expect(
            page.getByRole('button', { name: 'Editar participante', exact: true }),
          ).toHaveCount(0);
          await expect(detail(page).getByText('Nota del litigante', { exact: true })).toBeVisible();
          await page.getByRole('button', { name: 'Ver historial de cambios', exact: true }).click();
          await expect(page.locator('.participant-revision')).toHaveCount(8);
        }
      }
    }
    await revokeLitigator();
    await directory(litigator).getByRole('button', { name: 'Actualizar', exact: true }).click();
    await expect(litigator.getByRole('alert')).toBeVisible();
    await expect(detail(litigator)).toHaveCount(0);
    await expect(directory(litigator).locator('.participant-row')).toHaveCount(0);
  } finally {
    for (const context of contexts) await context.close();
  }
}
