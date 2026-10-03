import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { calendarRecord, calendarId } from '../fixtures/judicial-calendars.mjs';
import {
  calendarDraftSetup,
  checkCalendarDraftRequests,
  holdCalendarRequest,
  calendarsPath,
} from './session-calendar-draft-fixtures.mjs';
import {
  enterCalendars,
  beginCalendar,
  fillCalendar,
  rawCalendar,
  reviewCalendar,
  prepareCalendar,
  confirmCalendar,
  exactCalendar,
} from './session-calendar-draft-ui.mjs';

test.afterEach(async ({ page }) => checkCalendarDraftRequests(page));

test('an interrupted calendar retirement remains uncertain after absence and a foreign exact receipt never confirms it', async ({
  page,
}) => {
  const state = await calendarDraftSetup(page, { records: [calendarRecord()] });
  await login(page, false, false);
  await enterCalendars(page);
  let form = await beginCalendar(page, 'retire');
  await form.getByLabel('Motivo', { exact: true }).fill(rawCalendar.reason);
  await prepareCalendar(state, form);
  const sent = holdCalendarRequest(state, 'POST', `${calendarsPath}/${calendarId}/retirement`);
  state.calendarWrite = { commit: false, status: 503 };
  await confirmCalendar(form).click();
  await expect.poll(() => sent.entered).toBe(true);
  const submission = structuredClone(state.calendarPosts[0]);
  await expire(page, state, await form.elementHandle());
  await login(page, false, false);
  await enterCalendars(page);
  form = await beginCalendar(page, 'retire');
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(rawCalendar.reason);
  await expect(reviewCalendar(form)).toBeDisabled();
  const before = state.calls.length;
  await exactCalendar(form).click();
  await expect(form.getByRole('alert')).toContainText('incierto');
  expect(
    state.calls
      .slice(before)
      .filter(
        (call) =>
          call.method === 'GET' && call.path === `${calendarsPath}/${calendarId}/revisions/2`,
      ),
  ).toHaveLength(1);
  await expect(reviewCalendar(form)).toBeDisabled();
  const foreign = structuredClone(submission);
  foreign.command.operation_id = '71000000-0000-4000-8000-000000000002';
  state.commitCalendar(foreign);
  await exactCalendar(form).click();
  await expect(form.getByRole('alert')).toContainText('otro env\u00edo');
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(rawCalendar.reason);
  await expect(confirmCalendar(form)).toHaveCount(0);
  expect(state.calendarPosts).toEqual([submission]);
  expect(state.calendarPrepares).toHaveLength(1);
  sent.release();
});

test('the exact calendar receipt removes a publication draft before a held list refresh and another expiry', async ({
  page,
}) => {
  const state = await calendarDraftSetup(page);
  await login(page, false, false);
  await enterCalendars(page);
  let form = await beginCalendar(page);
  await fillCalendar(page);
  await prepareCalendar(state, form);
  const sent = holdCalendarRequest(state, 'POST', calendarsPath);
  state.calendarWrite = { status: 503 };
  await confirmCalendar(form).click();
  await expect.poll(() => sent.entered).toBe(true);
  const submission = structuredClone(state.calendarPosts[0]);
  await expire(page, state, await form.elementHandle());
  await login(page, false, false);
  await enterCalendars(page);
  form = await beginCalendar(page);
  await expect(form.getByLabel('T\u00edtulo del calendario', { exact: true })).toHaveValue(
    'Calendario declarado',
  );
  await expect(reviewCalendar(form)).toBeDisabled();
  const refresh = holdCalendarRequest(state, 'GET', calendarsPath);
  const before = state.calls.length;
  await exactCalendar(form).click();
  await expect.poll(() => refresh.entered).toBe(true);
  expect(
    state.calls
      .slice(before)
      .some(
        (call) =>
          call.method === 'GET' &&
          call.path === `${calendarsPath}/${submission.command.calendar_id}/revisions/1`,
      ),
  ).toBe(true);
  await expire(page, state, await form.elementHandle());
  await login(page, false, false);
  await enterCalendars(page);
  form = await beginCalendar(page);
  await expect(form.getByLabel('T\u00edtulo del calendario', { exact: true })).toHaveValue('');
  await form
    .getByLabel('T\u00edtulo del calendario', { exact: true })
    .fill('Otro calendario independiente');
  refresh.release();
  sent.release();
  await expect(form.getByLabel('T\u00edtulo del calendario', { exact: true })).toHaveValue(
    'Otro calendario independiente',
  );
  expect(state.calendarPosts).toEqual([submission]);
  expect(state.calendarPrepares).toHaveLength(1);
});
