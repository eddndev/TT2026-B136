import { randomUUID } from 'node:crypto';
import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import {
  accounts,
  editor,
  detail,
  openResults,
  openResult,
  prepare,
  submit,
  accountAction,
} from './hearing-result-helpers.mjs';

test('real result conflicts preserve drafts and allow independent cancellation of their exact scheduling source', async ({
  page,
  browser,
}, testInfo) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/');
  await loginAs(page, accounts.owner, 1);
  await openResults(page, accounts.raceCase, accounts.raceHearing);
  await openResult(page, accounts.raceResult);
  await detail(page).getByRole('button', { name: 'Rectificar registro', exact: true }).click();
  await editor(page)
    .getByLabel('Relato del operador', { exact: true })
    .fill('Primera precision confirmada');
  await editor(page).getByLabel('Motivo', { exact: true }).fill('Precision de primera sesion');
  const first = await prepare(page);
  const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
  try {
    const other = await context.newPage();
    other.on('pageerror', (error) => errors.push(error.message));
    await other.goto('/');
    await loginAs(other, accounts.owner, 2);
    await openResults(other, accounts.raceCase, accounts.raceHearing);
    await openResult(other, accounts.raceResult);
    await detail(other).getByRole('button', { name: 'Rectificar registro', exact: true }).click();
    await editor(other)
      .getByLabel('Relato del operador', { exact: true })
      .fill('Borrador concurrente preservado');
    await editor(other).getByLabel('Motivo', { exact: true }).fill('Precision de otra sesion');
    const stale = await prepare(other);
    await accountAction(accounts.owner, 5, async (call) => {
      const path = `/cases/${accounts.raceCase.id}/hearings`;
      const command = {
        operation_id: randomUUID(),
        hearing_id: accounts.raceHearing.id,
        change: {
          action: 'cancel',
          expected_revision: 1,
          reason: 'Cancelacion de cita independiente del resultado',
        },
      };
      const prepared = await call('POST', `${path}/prepare`, command);
      const cancelled = await call(
        'POST',
        `${path}/${command.hearing_id}/cancellation`,
        {
          command: prepared.command,
          expected_submission_digest: prepared.submission_digest,
        },
        201,
      );
      expect(cancelled.status).toBe('cancelled');
    });
    const confirmed = await submit(page, first);
    expect(confirmed.revision).toBe(2);
    expect(confirmed.anchor.status).toBe('scheduled');
    expect(confirmed.anchor.revision).toBe(1);
    const conflict = await submit(other, stale, 409);
    expect(conflict.error.code).toBe('hearing_result_revision_conflict');
    await expect(editor(other).getByLabel('Relato del operador', { exact: true })).toHaveValue(
      'Borrador concurrente preservado',
    );
    await expect(
      editor(other).getByRole('button', { name: 'Revisar resultado', exact: true }),
    ).toBeDisabled();
    await editor(other)
      .getByRole('button', { name: 'Consultar base actual del resultado', exact: true })
      .click();
    await expect(editor(other)).toContainText('Primera precision confirmada');
    await editor(other)
      .getByRole('button', { name: 'Usar esta base y conservar borrador', exact: true })
      .click();
    const corrected = await submit(other, await prepare(other));
    expect(corrected.revision).toBe(3);
    expect(corrected.anchor).toEqual(confirmed.anchor);
    expect(corrected.values.summary).toBe('Borrador concurrente preservado');
    const historyResponse = other.waitForResponse((response) =>
      response.url().includes('/history?'),
    );
    await detail(other)
      .getByRole('button', { name: 'Ver historial del registro', exact: true })
      .click();
    const history = await (await historyResponse).json();
    expect(history.revisions.map((row) => row.revision)).toEqual([3, 2, 1]);
    for (const row of history.revisions) {
      expect(row).not.toHaveProperty('values');
      expect(row).not.toHaveProperty('attendees');
      expect(row).not.toHaveProperty('support');
    }
  } finally {
    await context.close();
  }
  expect(errors).toEqual([]);
});
