import { test, expect } from '@playwright/test';
import {
  setupCalendars,
  openCalendars,
  calendarPanel,
  calendarDetail,
} from './judicial-calendar-helpers.mjs';
import { login, navigate, selectCase } from './helpers.mjs';
import { calendarId, calendarRecord } from '../fixtures/judicial-calendars.mjs';
test('Client direct calendar location resolves to overview without catalog requests', async ({
  page,
}) => {
  const { state } = await setupCalendars(page, { role: 'client' });
  await login(page, false, false);
  await page.evaluate(() => {
    location.hash = 'judicial-calendars';
  });
  await expect(
    page.getByRole('heading', { name: 'Tu mesa de trabajo', exact: true }),
  ).toBeVisible();
  await expect(page).toHaveURL(/#overview$/);
  expect(state.calls).toEqual([]);
  await expect(
    page.getByRole('button', { name: 'Calendarios jurisdiccionales', exact: true }),
  ).toHaveCount(0);
});
test('Agenda and administration open the same global catalog without changing selected case', async ({
  page,
}) => {
  const { common } = await setupCalendars(page);
  await login(page, false, false);
  await selectCase(page);
  await navigate(page, 'Agenda');
  await page
    .getByRole('button', { name: 'Calendarios jurisdiccionales', exact: true })
    .last()
    .click();
  await expect(calendarPanel(page)).toBeVisible();
  const before = common.filter((r) => r.path.includes('/cases/')).length;
  await calendarPanel(page)
    .getByRole('button', { name: `Consultar calendario ${calendarId}`, exact: true })
    .click();
  await expect(calendarDetail(page)).toBeVisible();
  expect(common.filter((r) => r.path.includes('/cases/'))).toHaveLength(before);
  await navigate(page, 'Calendarios jurisdiccionales');
  await expect(calendarPanel(page)).toBeVisible();
});
test('revoked permission clears already visible records and editor controls', async ({ page }) => {
  const { state } = await setupCalendars(page);
  await openCalendars(page);
  await calendarPanel(page)
    .getByRole('button', { name: `Consultar calendario ${calendarId}`, exact: true })
    .click();
  await expect(calendarDetail(page)).toBeVisible();
  state.denied = true;
  await calendarPanel(page)
    .getByRole('button', { name: 'Actualizar calendarios', exact: true })
    .click();
  await expect(page.getByRole('alert')).toContainText('No tienes permiso');
  await expect(calendarDetail(page)).toHaveCount(0);
  await expect(
    calendarPanel(page).getByRole('button', { name: 'Publicar calendario', exact: true }),
  ).toHaveCount(0);
});
test('late detail cannot reopen the calendar after navigation elsewhere', async ({ page }) => {
  await setupCalendars(page);
  await openCalendars(page);
  let release;
  const held = new Promise((resolve) => (release = resolve));
  let started;
  const seen = new Promise((resolve) => (started = resolve));
  await page.route(`**/api/v1/judicial-calendars/${calendarId}`, async (route) => {
    started();
    await held;
    await route.fulfill({ json: calendarRecord() });
  });
  await calendarPanel(page)
    .getByRole('button', { name: `Consultar calendario ${calendarId}`, exact: true })
    .click();
  await seen;
  await navigate(page, 'Inicio');
  release();
  await expect(
    page.getByRole('heading', { name: 'Tu mesa de trabajo', exact: true }),
  ).toBeVisible();
  await expect(calendarDetail(page)).toHaveCount(0);
});
