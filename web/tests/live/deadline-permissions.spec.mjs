import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import { navigate } from '../case-administration-workflow.mjs';
import {
  accounts,
  openDeadlines,
  openDeadline,
  editor,
  detail,
  prepare,
  submit,
  accountAction,
} from './deadline-helpers.mjs';

test('real deadline roles retain closed history and clear private state after membership revocation', async ({
  page,
  browser,
}, testInfo) => {
  test.setTimeout(90000);
  await page.goto('/');
  await loginAs(page, accounts.litigator, 0);
  await openDeadlines(page, accounts.policyCase);
  await accountAction(accounts.litigator, 1, async (call) => {
    const foreign = `/cases/${accounts.hiddenCase.id}/deadlines`;
    expect((await call('GET', foreign, undefined, 404)).error.code).toBe('case_not_found');
    expect(
      (await call('GET', `${foreign}/${accounts.hiddenCaseDeadline.id}`, undefined, 404)).error
        .code,
    ).toBe('case_not_found');
  });
  await openDeadline(page, accounts.policyCaseDeadline);
  await detail(page).getByRole('button', { name: 'Corregir plazo', exact: true }).click();
  await editor(page)
    .getByLabel('T\u00edtulo del plazo', { exact: true })
    .fill('Plazo corregido por el litigante');
  await editor(page).getByLabel('Motivo', { exact: true }).fill('Precision de titulo declarada');
  const corrected = await submit(page, await prepare(page));
  expect(corrected.recorded_by.id).toBe(accounts.litigator.id);
  await detail(page).getByRole('button', { name: 'Corregir plazo', exact: true }).click();
  await editor(page)
    .getByLabel('T\u00edtulo del plazo', { exact: true })
    .fill('Borrador conservado al cerrar el expediente');
  await editor(page)
    .getByLabel('Motivo', { exact: true })
    .fill('Cambio preparado antes del cierre');
  await accountAction(accounts.owner, 3, async (call) => {
    await call('PUT', `/cases/${accounts.policyCase.id}/administrative-status`, {
      expected_revision: 1,
      administrative_status: 'closed',
    });
    const failure = await prepare(page, 409);
    expect(failure.error.code).toBe('case_closed');
    await expect(editor(page).getByLabel('T\u00edtulo del plazo', { exact: true })).toHaveValue(
      'Borrador conservado al cerrar el expediente',
    );
    await expect(
      editor(page).getByRole('button', { name: 'Preparar plazo', exact: true }),
    ).toBeDisabled();
    await editor(page).getByRole('button', { name: 'Cerrar formulario', exact: true }).click();
    for (const role of ['paralegal', 'client']) {
      const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
      try {
        const other = await context.newPage(),
          requests = [];
        other.on('request', (request) => requests.push(new URL(request.url()).pathname));
        await other.goto('/');
        await loginAs(other, accounts[role], 0);
        if (role === 'paralegal') {
          await openDeadlines(other, accounts.policyCase);
          await openDeadline(other, corrected);
          await expect(detail(other)).toContainText(corrected.definition.title);
          await expect(
            other.getByRole('button', { name: 'Registrar plazo', exact: true }),
          ).toHaveCount(0);
          await expect(
            detail(other).getByRole('button', { name: 'Corregir plazo', exact: true }),
          ).toHaveCount(0);
          await detail(other)
            .getByRole('button', { name: 'Ver historial de plazo', exact: true })
            .click();
          await other
            .getByRole('button', { name: 'Consultar plazo revisi\u00f3n 1', exact: true })
            .click();
          await expect(detail(other)).toContainText(accounts.policyCaseDeadline.definition.title);
        } else {
          await navigate(other, 'Expedientes');
          await other.getByRole('button', { name: new RegExp(accounts.policyCase.title) }).click();
          await expect(
            other.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
          ).toBeVisible();
          await expect(other.getByRole('link', { name: 'Plazos', exact: true })).toHaveCount(0);
          expect(
            requests.filter((path) => /\/(deadlines|deadline-profiles)(\/|$)/.test(path)),
          ).toEqual([]);
          await accountAction(accounts.client, 1, async (client) => {
            expect(
              (await client('GET', `/cases/${accounts.policyCase.id}/deadlines`, undefined, 403))
                .error.code,
            ).toBe('permission_denied');
          });
        }
      } finally {
        await context.close();
      }
    }
    expect(
      (await call('GET', `/cases/${accounts.hiddenCase.id}/deadlines`)).deadlines,
    ).toHaveLength(1);
    await call(
      'DELETE',
      `/cases/${accounts.policyCase.id}/members/${accounts.litigator.id}`,
      undefined,
      204,
    );
  });
  await page.getByRole('button', { name: 'Actualizar plazos', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
  ).toBeVisible();
  await expect(detail(page)).toHaveCount(0);
  await expect(editor(page)).toHaveCount(0);
});
