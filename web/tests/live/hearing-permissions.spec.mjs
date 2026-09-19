import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import { navigate, openCase } from '../case-administration-workflow.mjs';
import {
  accounts,
  editor,
  detail,
  openHearings,
  prepare,
  submit,
  ownerAction,
  queryAgenda,
} from './hearing-helpers.mjs';
test('real agenda honors assignments roles closure and revocation without a browser case fanout', async ({
  page,
  browser,
}, testInfo) => {
  const errors = [],
    paths = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('request', (request) => paths.push(new URL(request.url()).pathname));
  await page.goto('/');
  await loginAs(page, accounts.litigator, 0);
  await navigate(page, 'Agenda');
  await queryAgenda(page);
  await expect(
    page.getByRole('button', {
      name: `Consultar audiencia ${accounts.policyCaseHearing.id}`,
      exact: true,
    }),
  ).toBeVisible();
  await expect(
    page.getByRole('button', {
      name: `Consultar audiencia ${accounts.hiddenCaseHearing.id}`,
      exact: true,
    }),
  ).toHaveCount(0);
  await expect(page.locator('.hearing-agenda')).not.toContainText('Sede sintetica privada');
  expect(
    paths.filter((path) => path === '/api/v1/cases' || path === '/api/v1/case-administrations'),
  ).toEqual([]);
  await page
    .getByRole('button', {
      name: `Consultar audiencia ${accounts.policyCaseHearing.id}`,
      exact: true,
    })
    .click();
  await expect(detail(page)).toContainText('consultada exactamente');
  await page.getByRole('button', { name: 'Consultar registro actual', exact: true }).click();
  await page.getByRole('button', { name: 'Corregir o reprogramar', exact: true }).click();
  await editor(page)
    .getByLabel('Sede o conexi\u00f3n', { exact: true })
    .fill('Sede declarada por litigante');
  await editor(page).getByLabel('Motivo del cambio', { exact: true }).fill('Correccion autorizada');
  expect((await submit(page, await prepare(page))).recorded_by.id).toBe(accounts.litigator.id);
  await page.getByRole('button', { name: 'Corregir o reprogramar', exact: true }).click();
  await editor(page)
    .getByLabel('Motivo del cambio', { exact: true })
    .fill('Borrador antes del cierre');
  await ownerAction(3, async (call) => {
    await call('PUT', `/cases/${accounts.policyCase.id}/administrative-status`, {
      expected_revision: 1,
      administrative_status: 'closed',
    });
    await editor(page).getByRole('button', { name: 'Revisar registro', exact: true }).click();
    await expect(editor(page)).toContainText('Tu formulario se conserva');
    await expect(editor(page).getByLabel('Motivo del cambio', { exact: true })).toHaveValue(
      'Borrador antes del cierre',
    );
    await expect(
      editor(page).getByRole('button', { name: 'Revisar registro', exact: true }),
    ).toBeDisabled();
    await editor(page)
      .getByRole('button', { name: 'Cerrar formulario de audiencia', exact: true })
      .click();
    for (const role of ['paralegal', 'client']) {
      const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
      try {
        const other = await context.newPage(),
          reads = [];
        other.on('pageerror', (error) => errors.push(error.message));
        other.on('request', (request) => {
          const path = new URL(request.url()).pathname;
          if (path.startsWith('/api/v1/')) reads.push(path);
        });
        await other.goto('/');
        await loginAs(other, accounts[role], 0);
        if (role === 'paralegal') {
          await navigate(other, 'Agenda');
          await queryAgenda(other);
          await expect(other.locator('.hearing-agenda')).toContainText('Expediente cerrado');
          await other
            .getByRole('button', {
              name: `Consultar audiencia ${accounts.policyCaseHearing.id}`,
              exact: true,
            })
            .click();
          await expect(detail(other)).toContainText('Sede declarada por litigante');
          await expect(
            other.getByRole('button', { name: 'Programar audiencia', exact: true }),
          ).toHaveCount(0);
          await other
            .getByRole('button', { name: 'Ver historial de audiencias', exact: true })
            .click();
          await expect(other.locator('.hearing-revision')).toHaveCount(2);
        } else {
          await expect(other.getByRole('button', { name: 'Alertas', exact: true })).toHaveCount(0);
          await navigate(other, 'Expedientes');
          await other.getByRole('button', { name: new RegExp(accounts.policyCase.title) }).click();
          await expect(
            other.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
          ).toBeVisible();
          await expect(other.getByRole('link', { name: 'Audiencias', exact: true })).toHaveCount(0);
          await other.evaluate(() => {
            location.hash = 'agenda';
          });
          await expect(
            other.getByRole('heading', { name: 'Tu mesa de trabajo', exact: true }),
          ).toBeVisible();
          await other.evaluate(() => {
            location.hash = 'alerts';
          });
          await expect(
            other.getByRole('heading', { name: 'Tu mesa de trabajo', exact: true }),
          ).toBeVisible();
          expect(
            reads.filter(
              (path) =>
                path.includes('/hearings') ||
                path === '/api/v1/agenda' ||
                path.startsWith('/api/v1/alerts') ||
                path === '/api/v1/alert-preferences',
            ),
          ).toEqual([]);
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
  await page.getByRole('button', { name: 'Actualizar audiencias', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
  ).toBeVisible();
  await expect(detail(page)).toHaveCount(0);
  expect(errors).toEqual([]);
});
