import { expect } from '@playwright/test';
import { fixture } from './helpers.mjs';
import { openCase } from '../case-administration-workflow.mjs';
import { openHearings, openHearing } from './hearing-helpers.mjs';
export const accounts = fixture.hearingResults;
export const panel = (page) =>
  page.getByRole('region', { name: 'Sesiones y resultados declarados', exact: true });
export const editor = (page) =>
  page.getByRole('region', { name: 'Formulario de sesi\u00f3n o acto', exact: true });
export const detail = (page) =>
  page.getByRole('region', { name: 'Detalle del resultado declarado', exact: true });
export async function openResults(page, record = accounts.case, hearing = accounts.hearing) {
  await openCase(page, record);
  await openHearings(page);
  await openHearing(page, hearing.id);
  await page.getByRole('button', { name: 'Ver sesiones y resultados', exact: true }).click();
  await expect(
    panel(page).getByRole('region', { name: 'Registros de sesiones y actos', exact: true }),
  ).toHaveAttribute('aria-busy', 'false');
}
export async function openResult(page, result) {
  await panel(page)
    .getByRole('button', { name: `Consultar resultado ${result.id}`, exact: true })
    .click();
  await expect(detail(page)).toBeVisible();
}
export async function start(page) {
  await panel(page)
    .getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true })
    .click();
}
export async function fill(page, summary = 'Sesion parcial comunicada') {
  const form = editor(page);
  await form.getByRole('combobox', { name: 'Ocurrencia', exact: true }).selectOption('occurred');
  await form
    .getByRole('combobox', { name: 'Alcance declarado', exact: true })
    .selectOption('partial');
  await form.getByLabel('Fecha', { exact: true }).fill('2026-09-01');
  await form.getByLabel('Desfase UTC', { exact: true }).fill('-06:00');
  await form.getByLabel('Relato del operador', { exact: true }).fill(summary);
  await form
    .getByRole('combobox', { name: 'Procedencia', exact: true })
    .selectOption('operator_note');
}
export async function prepare(page, expected = 200) {
  const response = page.waitForResponse(
    (r) => r.url().endsWith('/results/prepare') && r.request().method() === 'POST',
  );
  await editor(page).getByRole('button', { name: 'Revisar resultado', exact: true }).click();
  const value = await response;
  expect(value.status()).toBe(expected);
  return value.json();
}
export async function submit(page, prepared, expected = 201) {
  const command = prepared.command,
    action = command.change.action;
  const path = `/hearings/${command.hearing_id}/results${action === 'record' ? '' : `/${command.result_id}${action === 'withdraw' ? '/withdrawal' : ''}`}`;
  const response = page.waitForResponse(
    (r) =>
      r.url().endsWith(path) && r.request().method() === (action === 'correct' ? 'PUT' : 'POST'),
  );
  await editor(page).getByRole('button', { name: 'Confirmar resultado', exact: true }).click();
  const value = await response;
  expect(value.status()).toBe(expected);
  if (expected === 201) await expect(editor(page)).toHaveCount(0);
  return value.json();
}
export async function selectAnchor(page, hearing, revision) {
  const form = editor(page);
  await form
    .getByRole('button', { name: 'Elegir programaci\u00f3n de origen', exact: true })
    .click();
  await form
    .getByRole('button', { name: `Consultar programaci\u00f3n ${hearing.id}`, exact: true })
    .click();
  await form
    .getByRole('button', { name: `Usar programaci\u00f3n revisi\u00f3n ${revision}`, exact: true })
    .click();
}
export async function attendee(page, row, capacity) {
  const form = editor(page);
  const disclosure = form
    .locator('details')
    .filter({ has: page.getByText(/^Comparecencias informadas \(/) })
    .first();
  if (!(await disclosure.evaluate((element) => element.open)))
    await disclosure.locator('summary').first().click();
  await form.getByRole('button', { name: 'Agregar comparecencia', exact: true }).click();
  const picker = form.getByRole('region', { name: 'Elegir ficha hist\u00f3rica', exact: true });
  await picker
    .getByRole('button', { name: `Consultar historia de ficha ${row.id}`, exact: true })
    .click();
  await picker
    .getByRole('button', { name: `Consultar ficha revisi\u00f3n ${row.revision}`, exact: true })
    .click();
  await picker
    .getByRole('button', { name: 'Informar comparecencia de esta revisi\u00f3n', exact: true })
    .click();
  await form
    .getByRole('group', { name: `Comparecencia ${row.id}`, exact: true })
    .getByLabel('Calidad en esta sesi\u00f3n', { exact: true })
    .fill(capacity);
}
export async function selectSupport(page) {
  const form = editor(page);
  await form
    .getByRole('combobox', { name: 'Procedencia', exact: true })
    .selectOption('written_record');
  await form
    .getByLabel('Localizador de la fuente', { exact: true })
    .fill('Pagina 1 del antecedente escrito');
  await form.getByText('Soporte documental (opcional)', { exact: true }).click();
  await form.getByRole('button', { name: 'Elegir soporte del resultado', exact: true }).click();
  const picker = form.getByRole('region', { name: 'Seleccionar soporte exacto', exact: true });
  await picker.getByRole('button', { name: /result-support-later.pdf \/ versi/ }).click();
  await picker.getByRole('button', { name: /Versi\u00f3n 1 \/ result-support.pdf/ }).click();
  await picker.getByRole('button', { name: 'Usar esta versi\u00f3n', exact: true }).click();
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
