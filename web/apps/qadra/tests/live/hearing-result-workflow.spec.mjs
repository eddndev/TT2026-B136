import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import { capture } from './case-administration-helpers.mjs';
import {
  accounts,
  editor,
  detail,
  openResults,
  start,
  fill,
  prepare,
  submit,
  selectAnchor,
  attendee,
  selectSupport,
  accountAction,
} from './hearing-result-helpers.mjs';

test('real sessions retain exact sources through lost response correction withdrawal and cross-hearing continuation', async ({
  page,
}, testInfo) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/');
  await loginAs(page, accounts.owner, 0);
  await openResults(page);
  await start(page);
  await expect(editor(page).getByRole('combobox', { name: 'Ocurrencia', exact: true })).toHaveValue(
    '',
  );
  await expect(
    editor(page).getByRole('combobox', { name: 'Alcance declarado', exact: true }),
  ).toHaveValue('');
  await expect(editor(page).getByLabel('Fecha', { exact: true })).toHaveValue('');
  await selectAnchor(page, accounts.hearing, 1);
  await fill(page);
  await attendee(page, accounts.manual, 'Testigo compareciente');
  await attendee(page, accounts.typed, 'Imputado compareciente');
  await selectSupport(page);
  await editor(page).getByText('Acuerdos declarados (0/16)', { exact: true }).click();
  await editor(page)
    .getByRole('button', { name: 'Agregar acuerdo declarado', exact: true })
    .click();
  await editor(page)
    .getByLabel('Texto del acuerdo 1', { exact: true })
    .fill('Acuerdo comunicado por el operador');
  const prepared = await prepare(page);
  expect(prepared.anchor.revision).toBe(1);
  expect(prepared.anchor.status).toBe('scheduled');
  expect(prepared.values.provenance.support).toEqual(accounts.support);
  const path = `**/api/v1/cases/${accounts.case.id}/hearings/${accounts.hearing.id}/results`;
  let actual,
    writes = 0;
  await page.route(path, async (route) => {
    if (route.request().method() !== 'POST') return route.continue();
    writes++;
    const response = await route.fetch();
    expect(response.status()).toBe(201);
    actual = await response.json();
    await route.abort('failed');
  });
  await editor(page).getByRole('button', { name: 'Confirmar resultado', exact: true }).click();
  await expect(editor(page)).toContainText('Resultado incierto');
  await editor(page)
    .getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true })
    .click();
  await expect(editor(page)).toHaveCount(0);
  await expect(detail(page)).toContainText('Consultada exactamente');
  expect(writes).toBe(1);
  expect(actual.receipt.submission_digest).toBe(prepared.submission_digest);
  expect(actual.recorded_by.id).toBe(accounts.owner.id);
  expect(actual.attendees.find((row) => row.id === accounts.manual.id).display_name).toBe(
    accounts.manual.display_name,
  );
  const typed = actual.attendees.find((row) => row.id === accounts.typed.id);
  expect(typed.subject.revision).toBe(1);
  expect(typed.subject_digest).toBe(accounts.typed.subject.values_digest);
  await page.unroute(path);
  await detail(page)
    .getByRole('button', { name: 'Consultar resultado actual', exact: true })
    .click();
  await detail(page).getByRole('button', { name: 'Rectificar registro', exact: true }).click();
  await editor(page)
    .getByLabel('Relato del operador', { exact: true })
    .fill('Sesion parcial precisada');
  await editor(page).getByLabel('Motivo', { exact: true }).fill('Precision del relato comunicado');
  const corrected = await submit(page, await prepare(page));
  expect(corrected.revision).toBe(2);
  expect(corrected.values.agreements).toEqual(actual.values.agreements);
  expect(corrected.attendees).toEqual(actual.attendees);
  expect(corrected.support).toEqual(actual.support);
  await detail(page).getByRole('button', { name: 'Retirar registro', exact: true }).click();
  await editor(page).getByLabel('Motivo', { exact: true }).fill('Retiro administrativo de captura');
  const withdrawn = await submit(page, await prepare(page));
  expect(withdrawn.revision).toBe(3);
  expect(withdrawn.values).toEqual(corrected.values);
  expect(withdrawn.support).toEqual(corrected.support);
  await detail(page)
    .getByRole('button', { name: 'Ver historial del registro', exact: true })
    .click();
  await page
    .getByRole('button', { name: 'Consultar resultado revisi\u00f3n 1', exact: true })
    .click();
  await expect(detail(page)).toContainText('Sesion parcial comunicada');
  await detail(page)
    .getByRole('button', { name: 'Consultar resultado actual', exact: true })
    .click();
  await detail(page)
    .getByRole('button', { name: 'Registrar continuaci\u00f3n', exact: true })
    .click();
  await selectAnchor(page, accounts.nextHearing, 2);
  await fill(page, 'Comparecencia comunicada sin inicio de continuacion');
  await editor(page)
    .getByRole('combobox', { name: 'Ocurrencia', exact: true })
    .selectOption('not_started');
  await editor(page)
    .getByRole('combobox', { name: 'Alcance declarado', exact: true })
    .selectOption('unspecified');
  await attendee(page, accounts.manual, 'Compareciente sin inicio');
  const continuation = await submit(page, await prepare(page));
  expect(continuation.id).not.toBe(actual.id);
  expect(continuation.anchor.status).toBe('cancelled');
  expect(continuation.continuation).toMatchObject({
    result_id: actual.id,
    revision: 3,
    status: 'withdrawn',
  });
  await expect(detail(page)).toContainText('Comparecencia comunicada sin inicio de continuacion');
  await detail(page)
    .getByRole('button', { name: 'Consultar antecedente exacto', exact: true })
    .click();
  await expect(detail(page)).toContainText('Registro retirado');
  await expect(detail(page)).toContainText('Consultada exactamente');
  await capture(page, testInfo, 'result-history-desktop');
  await page.setViewportSize({ width: 390, height: 844 });
  await capture(page, testInfo, 'result-history-mobile');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await accountAction(accounts.owner, 4, async (call) => {
    const scope = `/cases/${accounts.case.id}`;
    expect((await call('GET', `${scope}/hearings/${accounts.hearing.id}`)).revision).toBe(3);
    expect(
      (await call('GET', `${scope}/participants/${accounts.manual.id}`)).directory_status,
    ).toBe('archived');
    const document = await call('GET', `${scope}/documents/${accounts.support.document_id}`);
    expect(document.version).toBe(2);
    expect(
      (await call('GET', `${scope}/documents/${accounts.support.document_id}/versions/1`)).sealed,
    ).toBe(true);
  });
  expect(errors).toEqual([]);
});
