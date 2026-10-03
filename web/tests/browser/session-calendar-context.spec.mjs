import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire, signInOther } from './session-inactivity-helpers.mjs';
import {
  calendarDraftSetup,
  checkCalendarDraftRequests,
  holdCalendarRequest,
  ownerPath,
} from './session-calendar-draft-fixtures.mjs';
import {
  enterCalendars,
  beginCalendar,
  calendarEditor,
  calendarPanel,
  rawCalendar,
  reviewCalendar,
  confirmCalendar,
} from './session-calendar-draft-ui.mjs';

test.afterEach(async ({ page }) => checkCalendarDraftRequests(page));
const title = (form) => form.getByLabel('T\u00edtulo del calendario', { exact: true });
const close = (form) =>
  form.getByRole('button', { name: 'Cerrar formulario de calendario', exact: true });

test('closing a calendar draft, another principal and explicit logout discard its global capture', async ({
  page,
}) => {
  const state = await calendarDraftSetup(page);
  await login(page, false, false);
  await enterCalendars(page);
  let form = await beginCalendar(page);
  await title(form).fill(rawCalendar.title);
  await expire(page, state, await form.elementHandle());
  await login(page, false, false);
  await enterCalendars(page);
  form = await beginCalendar(page);
  await expect(title(form)).toHaveValue(rawCalendar.title);
  const original = await form.elementHandle();
  await close(form).click();
  await expire(page, state, original);
  await login(page, false, false);
  await enterCalendars(page);
  form = await beginCalendar(page);
  await expect(title(form)).toHaveValue('');
  await title(form).fill('Captura reservada a la cuenta inicial');
  await expire(page, state, await form.elementHandle());
  await signInOther(page);
  await enterCalendars(page);
  form = await beginCalendar(page);
  await expect(title(form)).toHaveValue('');
  await title(form).fill('Captura de la segunda cuenta');
  await expire(page, state, await form.elementHandle());
  await signInOther(page);
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  await signInOther(page);
  await enterCalendars(page);
  form = await beginCalendar(page);
  await expect(title(form)).toHaveValue('');
  expect(state.calendarPrepares).toEqual([]);
  expect(state.calendarPosts).toEqual([]);
});

test('a late Owner response cannot revive a calendar after another expiry and fresh loss of permission', async ({
  page,
}) => {
  const state = await calendarDraftSetup(page);
  await login(page, false, false);
  await enterCalendars(page);
  let form = await beginCalendar(page);
  await title(form).fill(rawCalendar.title);
  await expire(page, state, await form.elementHandle());
  await login(page, false, false);
  await enterCalendars(page);
  const stale = holdCalendarRequest(state, 'GET', ownerPath);
  form = await beginCalendar(page);
  await expect.poll(() => stale.entered).toBe(true);
  await expect(reviewCalendar(form)).toBeDisabled();
  expect(
    await form.locator('input').evaluateAll((nodes) => nodes.map((node) => node.value)),
  ).not.toContain(rawCalendar.title);
  await expire(page, state, await form.elementHandle());
  await login(page, false, false);
  await enterCalendars(page);
  state.currentRole = 'litigator';
  await calendarPanel(page)
    .getByRole('button', { name: 'Publicar calendario', exact: true })
    .click();
  await expect(page.getByRole('alert')).toContainText(/Owner|permiso/i);
  await expect(calendarEditor(page)).toHaveCount(0);
  stale.release();
  await expect(calendarEditor(page)).toHaveCount(0);
  state.currentRole = null;
  await calendarPanel(page)
    .getByRole('button', { name: 'Actualizar calendarios', exact: true })
    .click();
  form = await beginCalendar(page);
  await expect(title(form)).toHaveValue('');
  await expect(confirmCalendar(form)).toHaveCount(0);
  expect(state.calendarPrepares).toEqual([]);
  expect(state.calendarPosts).toEqual([]);
});
