import { expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { fixture } from './helpers.mjs';
export const accounts = fixture.caseStages;
export const pdf = readFileSync(
  new URL('../../../crates/infrastructure/tests/fixtures/stage-support.pdf', import.meta.url),
);
export async function openStages(page) {
  await page.getByRole('link', { name: 'Etapas', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Etapas del expediente', exact: true }),
  ).toBeVisible();
  await expect(page.locator('.case-stages')).toHaveAttribute('aria-busy', 'false');
}
export async function fillDate(page, name, instant = false) {
  const group = page.getByRole('group', { name, exact: true });
  if (instant) await group.getByLabel('Precisi\u00f3n', { exact: true }).selectOption('instant');
  await group.getByLabel('Fecha', { exact: true }).fill('2026-09-01');
  if (instant) await group.getByLabel('Hora', { exact: true }).fill('10:30:00');
  await group.getByLabel('Desfase UTC', { exact: true }).fill('-06:00');
}
export async function uploadSupport(page, label, name = 'stage-accusation.pdf') {
  const group = page.getByRole('group', { name: label, exact: true });
  await group.getByRole('button', { name: 'Cargar soporte', exact: true }).click();
  const response = page.waitForResponse(
    (r) => r.url().endsWith('/documents/with-metadata') && r.request().method() === 'POST',
  );
  await page
    .getByRole('dialog', { name: 'Subir documento', exact: true })
    .getByLabel('Archivo', { exact: true })
    .setInputFiles({ name, mimeType: 'application/pdf', buffer: pdf });
  await page.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  const result = await response;
  expect(result.status()).toBe(201);
  await expect(
    page.getByRole('dialog', { name: 'Subir documento', exact: true }),
  ).not.toBeVisible();
  await expect(group).toContainText('Documento guardado');
  await expect(page.locator('.stage-form')).toHaveAttribute('aria-busy', 'false');
  return result.json();
}
export async function chooseSupport(page, label, name = 'stage-accusation.pdf', version = 1) {
  const group = page.getByRole('group', { name: label, exact: true });
  await group.getByRole('button', { name: 'Elegir documento', exact: true }).click();
  const picker = group.getByRole('region', { name: 'Seleccionar soporte exacto' });
  await picker.getByRole('button', { name: new RegExp(name) }).click();
  await picker.getByRole('button', { name: new RegExp(`Versi\u00f3n ${version} /`) }).click();
  await picker.getByRole('button', { name: 'Usar esta versi\u00f3n', exact: true }).click();
  await expect(group).toContainText(`Versi\u00f3n ${version}`);
}
export async function submitStage(page, caseId, adoption = false) {
  await page.getByRole('button', { name: 'Revisar registro', exact: true }).click();
  const response = page.waitForResponse(
    (r) =>
      r.url().endsWith(`/cases/${caseId}/stage/${adoption ? 'adoption' : 'transitions'}`) &&
      r.request().method() === 'POST',
  );
  await page
    .locator('.stage-confirmation')
    .getByRole('button', {
      name: adoption ? 'Registrar etapa actual' : 'Registrar transici\u00f3n',
      exact: true,
    })
    .click();
  const result = await response;
  expect(result.status()).toBe(201);
  await expect(page.locator('.stage-form')).toHaveCount(0);
  await expect(page.locator('.case-stages')).toHaveAttribute('aria-busy', 'false');
  return result.json();
}
export async function revokeLitigator() {
  let token;
  async function call(method, path, data) {
    const response = await fetch(`${process.env.API_PROXY_TARGET}/api/v1${path}`, {
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
    await call('DELETE', `/cases/${accounts.legacyCase.id}/members/${accounts.litigator.id}`);
  } finally {
    await call('POST', '/auth/logout');
  }
}
