import { randomUUID } from 'node:crypto';
import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import {
  accounts,
  openFacts,
  openResolution,
  factDetail,
  factEditor,
  prepare,
  submit,
  accountAction,
} from './procedural-facts-helpers.mjs';

test('real competing corrections preserve the draft and a lost committed response reconciles without resubmission', async ({
  page,
}) => {
  await page.goto('/');
  await loginAs(page, accounts.owner, 1);
  await openFacts(page, accounts.raceCase);
  await openResolution(page, accounts.raceResolution);
  await factDetail(page)
    .getByRole('button', { name: 'Corregir resoluci\u00f3n', exact: true })
    .click();
  const editor = factEditor(page);
  await editor
    .getByLabel('Resumen de la resoluci\u00f3n', { exact: true })
    .fill('Mi borrador conservado tras competencia');
  await editor.getByLabel('Motivo', { exact: true }).fill('Correccion declarada');
  const first = await prepare(page);
  const path = `/cases/${accounts.raceCase.id}/resolutions`;
  await accountAction(accounts.owner, 4, async (call) => {
    const command = structuredClone(first.command);
    command.operation_id = randomUUID();
    command.change.values.summary = 'Correccion confirmada por otra sesion';
    const draft = await call('POST', `${path}/prepare`, command);
    await call(
      'PUT',
      `${path}/${command.id}`,
      { command: draft.command, expected_submission_digest: draft.submission_digest },
      201,
    );
  });
  const failure = await submit(page, first, 409);
  expect(failure.error.code).toBe('procedural_fact_revision_conflict');
  await expect(editor.getByLabel('Resumen de la resoluci\u00f3n', { exact: true })).toHaveValue(
    'Mi borrador conservado tras competencia',
  );
  await editor.getByRole('button', { name: 'Consultar base actual', exact: true }).click();
  await expect(editor).toContainText('Correccion confirmada por otra sesion');
  await editor
    .getByRole('button', { name: 'Usar esta base y conservar borrador', exact: true })
    .click();
  const rebased = await prepare(page);
  expect(rebased.command.change.expected_revision).toBe(2);
  let writes = 0;
  await page.route(`**/api/v1${path}/${first.command.id}`, async (route) => {
    if (route.request().method() !== 'PUT') return route.continue();
    writes++;
    const response = await route.fetch();
    expect(response.status()).toBe(201);
    await route.abort('failed');
  });
  await editor.getByRole('button', { name: 'Confirmar registro', exact: true }).click();
  await expect(editor).toContainText('Resultado incierto');
  const exact = page.waitForResponse(
    (response) =>
      response.url().endsWith(`${path}/${first.command.id}/revisions/3`) &&
      response.request().method() === 'GET',
  );
  await editor.getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true }).click();
  const receipt = await (await exact).json();
  expect(receipt.receipt.submission_digest).toBe(rebased.submission_digest);
  expect(receipt.receipt.sources_digest).toBe(rebased.sources_digest);
  expect(receipt.receipt.operation_id).toBe(rebased.command.operation_id);
  expect(receipt.recorded_by.id).toBe(accounts.owner.id);
  await expect(editor).toHaveCount(0);
  await expect(factDetail(page)).toContainText('Mi borrador conservado tras competencia');
  await expect(
    factDetail(page).getByRole('button', { name: 'Corregir resoluci\u00f3n', exact: true }),
  ).toHaveCount(0);
  expect(writes).toBe(1);
});
