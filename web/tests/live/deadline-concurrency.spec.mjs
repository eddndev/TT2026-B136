import { randomUUID } from 'node:crypto';
import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import {
  accounts,
  openDeadlines,
  openDeadline,
  editor,
  detail,
  prepare,
  submit,
  acknowledge,
  accountAction,
} from './deadline-helpers.mjs';

test('real deadline conflict preserves a draft and lost committed response reconciles its exact revision without resubmission', async ({
  page,
}) => {
  test.setTimeout(90000);
  await page.goto('/');
  await loginAs(page, accounts.owner, 1);
  await openDeadlines(page, accounts.raceCase);
  await openDeadline(page, accounts.raceCaseDeadline);
  await detail(page).getByRole('button', { name: 'Corregir plazo', exact: true }).click();
  const form = editor(page),
    title = 'Borrador de plazo conservado despues de competencia';
  await form.getByLabel('T\u00edtulo del plazo', { exact: true }).fill(title);
  await form.getByLabel('Motivo', { exact: true }).fill('Precision declarada');
  const first = await prepare(page),
    path = `/cases/${accounts.raceCase.id}/deadlines`;
  await accountAction(accounts.owner, 4, async (call) => {
    const command = structuredClone(first.command);
    command.operation_id = randomUUID();
    command.change.definition.title = 'Correccion de plazo confirmada por otra sesion';
    const draft = await call('POST', `${path}/prepare`, command);
    await call(
      'PUT',
      `${path}/${command.deadline_id}`,
      { command: draft.command, expected_submission_digest: draft.submission_digest },
      201,
    );
  });
  const failure = await submit(page, first, 409);
  expect(failure.error.code).toBe('deadline_revision_conflict');
  await expect(form.getByLabel('T\u00edtulo del plazo', { exact: true })).toHaveValue(title);
  await form.getByRole('button', { name: 'Consultar base actual', exact: true }).click();
  await expect(form).toContainText('Correccion de plazo confirmada por otra sesion');
  await form
    .getByRole('button', { name: 'Usar esta base y conservar borrador', exact: true })
    .click();
  const rebased = await prepare(page);
  expect(rebased.command.change.expected_revision).toBe(2);
  let writes = 0;
  await page.route(`**/api/v1${path}/${first.command.deadline_id}`, async (route) => {
    if (route.request().method() !== 'PUT') return route.continue();
    writes++;
    const response = await route.fetch();
    expect(response.status()).toBe(201);
    await route.abort('failed');
  });
  await acknowledge(page);
  await form.getByRole('button', { name: 'Confirmar plazo', exact: true }).click();
  await expect(form).toContainText('Resultado incierto');
  const exact = page.waitForResponse(
    (response) =>
      response.url().endsWith(`${path}/${first.command.deadline_id}/revisions/3`) &&
      response.request().method() === 'GET',
  );
  await form.getByRole('button', { name: 'Consultar envio exacto', exact: true }).click();
  const receipt = await (await exact).json();
  expect(receipt.receipt.submission_digest).toBe(rebased.submission_digest);
  expect(receipt.receipt.review_digest).toBe(rebased.review_digest);
  expect(receipt.receipt.operation_id).toBe(rebased.command.operation_id);
  expect(receipt.recorded_by.id).toBe(accounts.owner.id);
  await expect(form).toHaveCount(0);
  await expect(detail(page)).toContainText(title);
  await expect(
    detail(page).getByRole('button', { name: 'Corregir plazo', exact: true }),
  ).toHaveCount(0);
  expect(writes).toBe(1);
});
