import { expect } from '@playwright/test';
import { fixture } from './helpers.mjs';
export const calendars = fixture.judicialCalendars;
export const panel = (page) =>
  page.getByRole('region', { name: 'Cat\u00e1logo de calendarios', exact: true });
export const editor = (page) =>
  page.getByRole('region', { name: 'Formulario de calendario', exact: true });
export const detail = (page) =>
  page.getByRole('region', { name: 'Detalle del calendario', exact: true });
export async function openCalendars(page, id) {
  await page.evaluate(() => {
    location.hash = 'judicial-calendars';
  });
  await expect(
    page.getByRole('heading', { name: 'Calendarios jurisdiccionales', exact: true }),
  ).toBeVisible();
  await expect(panel(page)).toHaveAttribute('aria-busy', 'false');
  if (id) {
    await panel(page)
      .getByRole('button', { name: `Consultar calendario ${id}`, exact: true })
      .click();
    await expect(detail(page)).toBeVisible();
  }
}
export async function fillCalendar(page) {
  const form = editor(page);
  await form
    .getByLabel('T\u00edtulo del calendario', { exact: true })
    .fill('Calendario desde navegador');
  await form.getByRole('combobox', { name: 'Fuero', exact: true }).selectOption('local');
  await form.getByLabel('01 Aguascalientes', { exact: true }).check();
  for (const name of ['Autoridad', '\u00d3rgano', 'Territorio'])
    await form.getByLabel(name, { exact: true }).fill(`${name} de prueba`);
  await form.getByLabel('Uso declarado', { exact: true }).fill('Clasificacion sintetica de prueba');
  await form.getByLabel('Cobertura desde', { exact: true }).fill('2000-02-27');
  await form.getByLabel('Cobertura hasta', { exact: true }).fill('2000-03-04');
  for (const day of [
    'Lunes',
    'Martes',
    'Mi\u00e9rcoles',
    'Jueves',
    'Viernes',
    'S\u00e1bado',
    'Domingo',
  ]) {
    const rule = form.getByRole('group', { name: `Regla de ${day}`, exact: true });
    await rule
      .getByRole('combobox', { name: 'Clasificaci\u00f3n', exact: true })
      .selectOption('unresolved');
    await rule
      .getByLabel('Explicaci\u00f3n', { exact: true })
      .fill('Falta fuente aplicable declarada');
  }
}
export async function prepare(page, expected = 200) {
  const response = page.waitForResponse(
    (r) => r.url().endsWith('/judicial-calendars/prepare') && r.request().method() === 'POST',
  );
  await editor(page).getByRole('button', { name: 'Revisar calendario', exact: true }).click();
  const value = await response;
  expect(value.status()).toBe(expected);
  return value.json();
}
export async function submit(page, prepared, expected = 201) {
  const command = prepared.command,
    action = command.change.action;
  const path = `/judicial-calendars${action === 'publish' ? '' : `/${command.calendar_id}${action === 'retire' ? '/retirement' : ''}`}`;
  const response = page.waitForResponse(
    (r) =>
      r.url().endsWith(path) && r.request().method() === (action === 'replace' ? 'PUT' : 'POST'),
  );
  await editor(page).getByRole('button', { name: 'Confirmar calendario', exact: true }).click();
  const value = await response;
  expect(value.status()).toBe(expected);
  if (expected === 201) await expect(editor(page)).toHaveCount(0);
  return value.json();
}
export async function accountAction(account, index, action) {
  let token;
  async function call(method, path, data, expected = 200) {
    const response = await fetch(`${process.env.API_PROXY_TARGET}/api/v1${path}`, {
      method,
      headers: {
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
        ...(data === undefined ? {} : { 'Content-Type': 'application/json' }),
      },
      body: data === undefined ? undefined : JSON.stringify(data),
    });
    expect(response.status, `${method} ${path}`).toBe(expected);
    return response.status === 204 ? null : response.json();
  }
  const challenge = await call('POST', '/auth/login', {
    email: account.email,
    password: account.password,
  });
  token = (
    await call('POST', '/auth/mfa/recovery', {
      challenge_token: challenge.challenge_token,
      code: account.recoveryCodes[index],
    })
  ).access_token;
  try {
    return await action(call);
  } finally {
    await call('POST', '/auth/logout', undefined, 204);
  }
}
