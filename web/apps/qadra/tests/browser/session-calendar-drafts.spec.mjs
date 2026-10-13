import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import {
  calendarRecord,
  calendarFixtureCommand,
  calendarId,
} from '../fixtures/judicial-calendars.mjs';
import {
  calendarDraftSetup,
  checkCalendarDraftRequests,
  holdCalendarRequest,
  ownerPath,
  calendarsPath,
} from './session-calendar-draft-fixtures.mjs';
import {
  enterCalendars,
  beginCalendar,
  partialCalendar,
  rawCalendar,
  reviewCalendar,
  prepareCalendar,
  confirmCalendar,
} from './session-calendar-draft-ui.mjs';

test.afterEach(async ({ page }) => checkCalendarDraftRequests(page));

test('calendar reentry authorizes the current Owner before restoring raw scope, stable sources and partial exceptions', async ({
  page,
}) => {
  const state = await calendarDraftSetup(page);
  await login(page, false, false);
  await enterCalendars(page);
  let form = await beginCalendar(page);
  const identities = await partialCalendar(page);
  await expire(page, state, await form.elementHandle());
  await login(page, true, false);
  await enterCalendars(page);
  const before = state.calls.length;
  const fresh = holdCalendarRequest(state, 'GET', ownerPath);
  form = await beginCalendar(page);
  await expect.poll(() => fresh.entered).toBe(true);
  await expect(reviewCalendar(form)).toBeDisabled();
  expect(
    await form.locator('input').evaluateAll((nodes) => nodes.map((node) => node.value)),
  ).not.toContain(rawCalendar.title);
  fresh.release();
  await expect(form.getByLabel('T\u00edtulo del calendario', { exact: true })).toHaveValue(
    rawCalendar.title,
  );
  await expect(reviewCalendar(form)).toBeEnabled();
  const source = form.getByRole('group', { name: 'Fuente 1', exact: true });
  const exception = form.getByRole('group', { name: 'Excepci\u00f3n 1', exact: true });
  await expect(source.getByLabel('Referencia HTTPS', { exact: true })).toHaveValue(rawCalendar.url);
  await expect(source.locator('details > code')).toHaveText(identities.source);
  await expect(
    exception
      .locator('details')
      .filter({
        hasText: 'Identidad de la excepci\u00f3n',
      })
      .locator('code'),
  ).toHaveText(identities.exception);
  await expect(exception.getByLabel('Excepci\u00f3n hasta', { exact: true })).toHaveValue('');
  await expect(exception.getByLabel('Explicaci\u00f3n', { exact: true })).toHaveValue(
    rawCalendar.explanation,
  );
  await expect(
    exception.getByRole('checkbox', { name: rawCalendar.source.trim(), exact: true }),
  ).toBeChecked();
  const monday = form.getByRole('group', { name: 'Regla de Lunes', exact: true });
  await expect(
    monday.getByRole('checkbox', { name: rawCalendar.source.trim(), exact: true }),
  ).toBeChecked();
  await expect(source.getByRole('button', { name: 'Quitar fuente 1', exact: true })).toBeDisabled();
  expect(state.calendarPrepares).toEqual([]);
  expect(state.calendarPosts).toEqual([]);
  expect(state.calls.slice(before).find((call) => call.path === ownerPath)).toMatchObject({
    method: 'GET',
    body: null,
    search: '',
    headers: { authorization: `Bearer ${state.current.token}` },
  });
  await source.getByLabel('Referencia HTTPS', { exact: true }).fill('https://example.org/calendar');
  await exception.getByLabel('Excepci\u00f3n hasta', { exact: true }).fill('2000-02-29');
  await prepareCalendar(state, form);
  const values = state.calendarPrepares[0].command.change.values;
  expect(values.sources[0].id).toBe(identities.source);
  expect(values.exceptions[0].id).toBe(identities.exception);
  expect(values.weekly_pattern[0].source_ids).toEqual([identities.source]);
  expect(values.exceptions[0].source_ids).toEqual([identities.source]);
  expect(state.calendarPosts).toEqual([]);
});

test('calendar replacement retains its original base and raw reason until current revision comparison is accepted', async ({
  page,
}) => {
  const initial = calendarRecord();
  const state = await calendarDraftSetup(page, { records: [initial] });
  await login(page, false, false);
  await enterCalendars(page);
  let form = await beginCalendar(page, 'replace');
  await form.getByLabel('Cobertura hasta', { exact: true }).fill('2000-03-10');
  await form.getByLabel('Motivo', { exact: true }).fill(rawCalendar.reason);
  await expire(page, state, await form.elementHandle());
  const concurrent = calendarFixtureCommand('replace', 1);
  concurrent.operation_id = '71000000-0000-4000-8000-000000000001';
  concurrent.change.reason = 'Cambio independiente de otro Owner';
  state.commitCalendar(state.prepareCalendar(concurrent));
  await login(page, false, false);
  await enterCalendars(page);
  form = await beginCalendar(page, 'replace');
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(rawCalendar.reason);
  await expect(form.getByLabel('Cobertura hasta', { exact: true })).toHaveValue('2000-03-10');
  await expect(form.getByLabel('T\u00edtulo del calendario', { exact: true })).toBeDisabled();
  await expect(reviewCalendar(form)).toBeDisabled();
  await form
    .getByRole('button', { name: 'Consultar base actual del calendario', exact: true })
    .click();
  await expect(form).toContainText('Revisi\u00f3n 2 / Publicado');
  await expect(reviewCalendar(form)).toBeDisabled();
  await form
    .getByRole('button', { name: 'Usar esta base y conservar borrador', exact: true })
    .click();
  await prepareCalendar(state, form);
  const command = state.calendarPrepares[0].command;
  expect(command.calendar_id).toBe(calendarId);
  expect(command.change.expected_revision).toBe(2);
  expect(command.change.reason).toBe(rawCalendar.reason.trim());
  expect(command.change.values.coverage.through).toBe('2000-03-10');
  expect(command.change.values.scope).toEqual(initial.values.scope);
  expect(state.calendarPosts).toEqual([]);
  await expect(confirmCalendar(form)).toBeEnabled();
  expect(state.calls.some((call) => call.path === `${calendarsPath}/${calendarId}`)).toBe(true);
});
