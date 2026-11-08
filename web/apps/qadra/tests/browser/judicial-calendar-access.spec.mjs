import { test, expect } from '@playwright/test';
import {
  setupCalendars,
  openCalendars,
  calendarPanel,
  calendarDetail,
} from './judicial-calendar-helpers.mjs';
import { login, navigate, selectCase, expectClientWorkspace } from './helpers.mjs';
import { calendarId, calendarRecord } from '../fixtures/judicial-calendars.mjs';
test('Client direct calendar location resolves to its contracts without catalog requests', async ({
  page,
}) => {
  const { state } = await setupCalendars(page, { role: 'client' });
  await login(page, false, false);
  await expectClientWorkspace(page, ['judicial-calendars']);
  expect(state.calls).toEqual([]);
  await expect(page.getByRole('button', { name: /^Calendarios/ })).toHaveCount(0);
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
  await navigate(page, 'Calendarios');
  await expect(calendarPanel(page)).toBeVisible();
});
