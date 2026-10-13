import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import { navigate } from '../case-administration-workflow.mjs';
import {
  accounts,
  panel,
  editor,
  detail,
  openResults,
  openResult,
  prepare,
  submit,
  accountAction,
} from './hearing-result-helpers.mjs';

test('real result permissions retain closed-case history and discard data after membership revocation', async ({
  page,
  browser,
}, testInfo) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/');
  await loginAs(page, accounts.litigator, 0);
  await openResults(page, accounts.policyCase, accounts.policyHearing);
  await openResult(page, accounts.policyResult);
  await detail(page).getByRole('button', { name: 'Rectificar registro', exact: true }).click();
  await editor(page)
    .getByLabel('Relato del operador', { exact: true })
    .fill('Precision autorizada del litigante');
  await editor(page).getByLabel('Motivo', { exact: true }).fill('Precision declarada');
  const result = await submit(page, await prepare(page));
  expect(result.recorded_by.id).toBe(accounts.litigator.id);
  await detail(page).getByRole('button', { name: 'Rectificar registro', exact: true }).click();
  await editor(page)
    .getByLabel('Relato del operador', { exact: true })
    .fill('Borrador conservado al cerrar');
  await editor(page).getByLabel('Motivo', { exact: true }).fill('Antes de cierre concurrente');
  await accountAction(accounts.owner, 3, async (call) => {
    await call('PUT', `/cases/${accounts.policyCase.id}/administrative-status`, {
      expected_revision: 1,
      administrative_status: 'closed',
    });
    const failure = await prepare(page, 409);
    expect(failure.error.code).toBe('case_closed');
    await expect(editor(page).getByLabel('Relato del operador', { exact: true })).toHaveValue(
      'Borrador conservado al cerrar',
    );
    await expect(
      editor(page).getByRole('button', { name: 'Revisar resultado', exact: true }),
    ).toBeDisabled();
    await editor(page)
      .getByRole('button', { name: 'Cerrar formulario de resultado', exact: true })
      .click();
    for (const role of ['paralegal', 'client']) {
      const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
      try {
        const other = await context.newPage(),
          paths = [];
        other.on('pageerror', (error) => errors.push(error.message));
        other.on('request', (request) => paths.push(new URL(request.url()).pathname));
        await other.goto('/');
        await loginAs(other, accounts[role], 0);
        if (role === 'paralegal') {
          await openResults(other, accounts.policyCase, accounts.policyHearing);
          await openResult(other, result);
          await expect(detail(other)).toContainText('Precision autorizada del litigante');
          await expect(
            panel(other).getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true }),
          ).toHaveCount(0);
          await expect(
            detail(other).getByRole('button', { name: 'Rectificar registro', exact: true }),
          ).toHaveCount(0);
          await detail(other)
            .getByRole('button', { name: 'Ver historial del registro', exact: true })
            .click();
          await other
            .getByRole('button', { name: 'Consultar resultado revisi\u00f3n 1', exact: true })
            .click();
          await expect(detail(other)).toContainText('Sesion declarada de fixture');
        } else {
          await navigate(other, 'Expedientes');
          await other.getByRole('button', { name: new RegExp(accounts.policyCase.title) }).click();
          await expect(
            other.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
          ).toBeVisible();
          await expect(other.getByRole('link', { name: 'Audiencias', exact: true })).toHaveCount(0);
          expect(paths.filter((path) => path.includes('/results'))).toEqual([]);
          await accountAction(accounts.client, 1, async (clientCall) => {
            const denied = await clientCall(
              'GET',
              `/cases/${accounts.policyCase.id}/hearings/${accounts.policyHearing.id}/results`,
              undefined,
              403,
            );
            expect(denied.error.code).toBe('permission_denied');
          });
        }
      } finally {
        await context.close();
      }
    }
    await call(
      'DELETE',
      `/cases/${accounts.policyCase.id}/members/${accounts.litigator.id}`,
      undefined,
      204,
    );
  });
  await panel(page).getByRole('button', { name: 'Actualizar resultados', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
  ).toBeVisible();
  await expect(detail(page)).toHaveCount(0);
  expect(errors).toEqual([]);
});
