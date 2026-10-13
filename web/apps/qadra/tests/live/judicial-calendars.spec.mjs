import { test, expect } from '@playwright/test';
import { loginAs } from './helpers.mjs';
import {
  calendars,
  panel,
  editor,
  detail,
  openCalendars,
  fillCalendar,
  prepare,
  submit,
  accountAction,
} from './judicial-calendar-helpers.mjs';

test('real calendar publication replacement retirement and exact uncertain receipt preserve history', async ({
  page,
}) => {
  await page.goto('/');
  await loginAs(page, calendars.owner, 0);
  await openCalendars(page);
  await panel(page).getByRole('button', { name: 'Publicar calendario', exact: true }).click();
  await fillCalendar(page);
  const prepared = await prepare(page);
  await expect(editor(page)).toContainText('Calendario desde navegador');
  const first = await submit(page, prepared);
  expect(first.revision).toBe(1);
  await detail(page)
    .getByRole('button', { name: '28 de febrero de 2000: Sin resolver', exact: true })
    .click();
  await expect(detail(page).getByRole('region', { name: 'Detalle del d\u00eda' })).toContainText(
    'Falta fuente aplicable declarada',
  );
  await detail(page).getByRole('button', { name: 'Reemplazar calendario', exact: true }).click();
  await editor(page).getByLabel('Cobertura hasta', { exact: true }).fill('2000-03-10');
  await editor(page).getByLabel('Motivo', { exact: true }).fill('Ampliacion de prueba');
  const replacement = await prepare(page);
  let writes = 0;
  await page.route(`**/api/v1/judicial-calendars/${first.id}`, async (route) => {
    if (route.request().method() !== 'PUT') return route.continue();
    writes++;
    const response = await route.fetch();
    expect(response.status()).toBe(201);
    await route.abort('failed');
  });
  await editor(page).getByRole('button', { name: 'Confirmar calendario', exact: true }).click();
  await expect(editor(page)).toContainText('Resultado incierto');
  await editor(page)
    .getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true })
    .click();
  await expect(editor(page)).toHaveCount(0);
  expect(writes).toBe(1);
  await expect(detail(page)).toContainText('Revisi\u00f3n 2 hist\u00f3rica');
  await page.unroute(`**/api/v1/judicial-calendars/${first.id}`);
  await detail(page)
    .getByRole('button', { name: 'Consultar calendario actual', exact: true })
    .click();
  await detail(page).getByRole('button', { name: 'Retirar calendario', exact: true }).click();
  await editor(page)
    .getByLabel('Motivo', { exact: true })
    .fill('Retiro de configuracion de prueba');
  const retired = await submit(page, await prepare(page));
  expect(retired.values_digest).toBe(replacement.values_digest);
  await expect(
    detail(page).getByRole('button', { name: 'Reemplazar calendario', exact: true }),
  ).toHaveCount(0);
  await detail(page)
    .getByRole('button', { name: 'Ver historial del calendario', exact: true })
    .click();
  await page
    .getByRole('button', { name: 'Consultar calendario revisi\u00f3n 1', exact: true })
    .click();
  await expect(detail(page)).toContainText('2000-02-27 / 2000-03-04');
  await accountAction(calendars.owner, 3, async (call) => {
    const exact = await call('GET', `/judicial-calendars/${first.id}/revisions/1`);
    expect(exact).toEqual(first);
    const current = await call('GET', `/judicial-calendars/${first.id}`);
    expect(current.status).toBe('retired');
    expect(current.revision).toBe(3);
  });
});

test('real calendar concurrent owners compare before preserving a draft on the new base', async ({
  browser,
}, testInfo) => {
  const contexts = await Promise.all([
    browser.newContext({ baseURL: testInfo.project.use.baseURL }),
    browser.newContext({ baseURL: testInfo.project.use.baseURL }),
  ]);
  try {
    const pages = await Promise.all(contexts.map((c) => c.newPage()));
    for (const [index, page] of pages.entries()) {
      await page.goto('/');
      await loginAs(page, calendars.owner, index + 1);
      await openCalendars(page, calendars.race.id);
      await detail(page)
        .getByRole('button', { name: 'Reemplazar calendario', exact: true })
        .click();
      await editor(page)
        .getByLabel('Cobertura hasta', { exact: true })
        .fill(index ? '2000-03-12' : '2000-03-11');
      await editor(page)
        .getByLabel('Motivo', { exact: true })
        .fill(`Borrador independiente ${index}`);
    }
    const first = await prepare(pages[0]),
      second = await prepare(pages[1]);
    await submit(pages[0], first);
    const conflict = await submit(pages[1], second, 409);
    expect(conflict.error.code).toBe('judicial_calendar_revision_conflict');
    const form = editor(pages[1]);
    await expect(form.getByLabel('Cobertura hasta', { exact: true })).toHaveValue('2000-03-12');
    await form
      .getByRole('button', { name: 'Consultar base actual del calendario', exact: true })
      .click();
    await expect(form).toContainText('Revisi\u00f3n 2 / Publicado');
    await form
      .getByRole('button', { name: 'Usar esta base y conservar borrador', exact: true })
      .click();
    const accepted = await prepare(pages[1]);
    expect(accepted.command.change.expected_revision).toBe(2);
    const final = await submit(pages[1], accepted);
    expect(final.revision).toBe(3);
    expect(final.values.coverage.through).toBe('2000-03-12');
  } finally {
    await Promise.all(contexts.map((c) => c.close()));
  }
});

test('real calendar catalogue is global staff read only and Client is denied', async ({
  browser,
}, testInfo) => {
  for (const role of ['litigator', 'paralegal', 'client']) {
    const context = await browser.newContext({ baseURL: testInfo.project.use.baseURL });
    try {
      const page = await context.newPage();
      await page.goto('/');
      await loginAs(page, calendars[role], 0);
      if (role === 'client') {
        await page.evaluate(() => {
          location.hash = 'judicial-calendars';
        });
        await expect(
          page.getByRole('heading', { name: 'Tu mesa de trabajo', exact: true }),
        ).toBeVisible();
        await expect(page).toHaveURL(/#overview$/);
        await expect(panel(page)).toHaveCount(0);
      } else {
        await openCalendars(page, calendars.read.id);
        await expect(
          panel(page).getByRole('button', { name: 'Publicar calendario', exact: true }),
        ).toHaveCount(0);
        await expect(
          detail(page).getByRole('button', { name: 'Reemplazar calendario', exact: true }),
        ).toHaveCount(0);
        await detail(page)
          .getByRole('button', { name: '29 de febrero de 2000: Excluido', exact: true })
          .click();
        await expect(
          detail(page).getByRole('region', { name: 'Detalle del d\u00eda' }),
        ).toContainText('Excepcion de prueba');
      }
      await accountAction(calendars[role], 1, async (call) => {
        await call('GET', '/judicial-calendars', undefined, role === 'client' ? 403 : 200);
        const forbidden = await call(
          'POST',
          '/judicial-calendars/prepare',
          {
            operation_id: crypto.randomUUID(),
            calendar_id: calendars.read.id,
            change: { action: 'retire', expected_revision: 1, reason: 'Intento sin permiso' },
          },
          403,
        );
        expect(forbidden.error.code).toBe('permission_denied');
      });
    } finally {
      await context.close();
    }
  }
});
