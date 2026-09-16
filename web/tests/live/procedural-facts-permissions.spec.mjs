import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import { navigate } from '../case-administration-workflow.mjs';
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

test('real fact permissions retain closed history and clear private data on revocation', async ({
  page,
  browser,
}, testInfo) => {
  await page.goto('/');
  await loginAs(page, accounts.litigator, 0);
  await openFacts(page, accounts.policyCase);
  await openResolution(page, accounts.policyResolution);
  await factDetail(page)
    .getByRole('button', { name: 'Corregir resoluci\u00f3n', exact: true })
    .click();
  await factEditor(page)
    .getByLabel('Resumen de la resoluci\u00f3n', { exact: true })
    .fill('Precision autorizada del litigante');
  await factEditor(page).getByLabel('Motivo', { exact: true }).fill('Detalle comunicado');
  const corrected = await submit(page, await prepare(page));
  expect(corrected.recorded_by.id).toBe(accounts.litigator.id);
  await factDetail(page)
    .getByRole('button', { name: 'Corregir resoluci\u00f3n', exact: true })
    .click();
  await factEditor(page)
    .getByLabel('Motivo', { exact: true })
    .fill('Nuevo detalle antes del cierre');
  await accountAction(accounts.owner, 3, async (call) => {
    await call('PUT', `/cases/${accounts.policyCase.id}/administrative-status`, {
      expected_revision: 1,
      administrative_status: 'closed',
    });
    const failure = await prepare(page, 'resolution', 409);
    expect(failure.error.code).toBe('case_closed');
    await expect(
      factEditor(page).getByLabel('Resumen de la resoluci\u00f3n', { exact: true }),
    ).toHaveValue('Precision autorizada del litigante');
    await expect(
      factEditor(page).getByRole('button', { name: 'Preparar registro', exact: true }),
    ).toBeDisabled();
    await factEditor(page).getByRole('button', { name: 'Cerrar formulario', exact: true }).click();
    for (const role of ['paralegal', 'client']) {
      const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
      try {
        const other = await context.newPage(),
          requests = [];
        other.on('request', (request) => requests.push(new URL(request.url()).pathname));
        await other.goto('/');
        await loginAs(other, accounts[role], 0);
        if (role === 'paralegal') {
          await openFacts(other, accounts.policyCase);
          await openResolution(other, corrected);
          await expect(factDetail(other)).toContainText('Precision autorizada del litigante');
          await expect(
            other.getByRole('button', { name: 'Registrar resoluci\u00f3n', exact: true }),
          ).toHaveCount(0);
          await factDetail(other)
            .getByRole('button', { name: 'Ver historial de resoluci\u00f3n', exact: true })
            .click();
          await other
            .getByRole('button', { name: 'Consultar resoluci\u00f3n revisi\u00f3n 1', exact: true })
            .click();
          await expect(factDetail(other)).toContainText(accounts.policyResolution.values.summary);
        } else {
          await navigate(other, 'Expedientes');
          await other.getByRole('button', { name: new RegExp(accounts.policyCase.title) }).click();
          await expect(
            other.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
          ).toBeVisible();
          await expect(other.getByRole('link', { name: 'Resoluciones', exact: true })).toHaveCount(
            0,
          );
          expect(requests.filter((path) => path.includes('/resolutions'))).toEqual([]);
          await accountAction(accounts.client, 1, async (clientCall) => {
            const denied = await clientCall(
              'GET',
              `/cases/${accounts.policyCase.id}/resolutions`,
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
  await page.getByRole('button', { name: 'Actualizar registros', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
  ).toBeVisible();
  await expect(factDetail(page)).toHaveCount(0);
});
