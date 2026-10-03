import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import {
  alertPreferenceDraftSetup,
  checkAlertPreferenceRequests,
  holdPreferenceRequest,
  preferencesPath,
  principalPath,
} from './session-alert-preference-fixtures.mjs';
import {
  openPreferenceDraft,
  enterPreferenceInbox,
  beginPreferences,
  reopenPreferences,
  fillPreferenceDraft,
  expectPreferenceDraft,
  hearingHours,
  savePreferences,
  comparePreferences,
  validPreferenceHours,
} from './session-alert-preference-ui.mjs';

test.afterEach(async ({ page }) => checkAlertPreferenceRequests(page));

test('raw personal alert hours and channels recover only after fresh principal and preference authority without an automatic save', async ({
  page,
}) => {
  const state = await alertPreferenceDraftSetup(page);
  let form = await openPreferenceDraft(page);
  await fillPreferenceDraft(form);
  await expire(page, state, await form.elementHandle());
  await login(page, false, false);
  await enterPreferenceInbox(page);
  const start = state.calls.length;
  const principal = holdPreferenceRequest(state, 'GET', principalPath);
  const preferences = holdPreferenceRequest(state, 'GET', preferencesPath);
  form = await beginPreferences(page);
  await expect.poll(() => principal.entered).toBe(true);
  await expect(hearingHours(form)).toHaveCount(0);
  expect(state.calls.slice(start).filter((call) => call.path === preferencesPath)).toEqual([]);
  principal.release();
  await expect.poll(() => preferences.entered).toBe(true);
  await expect(hearingHours(form)).toHaveCount(0);
  preferences.release();
  await expectPreferenceDraft(form);
  expect(state.preferenceWrites).toEqual([]);
  await savePreferences(form).click();
  await expect(form.getByRole('alert')).toContainText('horas enteras');
  await expectPreferenceDraft(form);
  expect(state.preferenceWrites).toEqual([]);
});

test('a concurrent preference revision preserves the original base until explicit comparison and loses that comparison at the next expiry', async ({
  page,
}) => {
  const state = await alertPreferenceDraftSetup(page);
  let form = await openPreferenceDraft(page);
  await fillPreferenceDraft(form, validPreferenceHours);
  await expire(page, state, await form.elementHandle());
  const external = {
    operation_id: 'a0000000-0000-4000-8000-000000000071',
    expected_revision: 0,
    values: structuredClone(state.preferenceRecord().values),
  };
  external.values.hearing_upcoming.lead_hours = [144];
  state.commitPreferences(external);
  form = await reopenPreferences(page);
  await expectPreferenceDraft(form, validPreferenceHours);
  await expect(savePreferences(form)).toBeDisabled();
  await expect(
    form.getByRole('region', { name: 'Preferencias actuales guardadas', exact: true }),
  ).toHaveCount(0);
  await comparePreferences(form).click();
  await expect(
    form.getByRole('region', { name: 'Preferencias actuales guardadas', exact: true }),
  ).toContainText('Revisi\u00f3n 1');
  await expect(
    form.getByRole('button', { name: 'Guardar mis preferencias', exact: true }),
  ).toBeEnabled();
  await expire(page, state, await form.elementHandle());
  form = await reopenPreferences(page);
  await expectPreferenceDraft(form, validPreferenceHours);
  await expect(savePreferences(form)).toBeDisabled();
  await expect(
    form.getByRole('region', { name: 'Preferencias actuales guardadas', exact: true }),
  ).toHaveCount(0);
  expect(state.preferenceWrites).toEqual([]);
  await comparePreferences(form).click();
  state.nextPreferenceWrite = {};
  await form.getByRole('button', { name: 'Guardar mis preferencias', exact: true }).click();
  await expect(form).toHaveCount(0);
  expect(state.preferenceWrites).toHaveLength(1);
  expect(state.preferenceWrites[0].values).toMatchObject({
    expected_revision: 1,
    values: {
      hearing_upcoming: { lead_hours: [72, 12], channels: { internal: true, email: false } },
    },
  });
  expect(state.preferenceWrites[0].values.operation_id).not.toBe(external.operation_id);
});

test('a failed fresh preference read retains the draft through explicit retry and a second expiry without applying its late response', async ({
  page,
}) => {
  const state = await alertPreferenceDraftSetup(page);
  let form = await openPreferenceDraft(page);
  await fillPreferenceDraft(form);
  await expire(page, state, await form.elementHandle());
  state.preferenceFailures.set(preferencesPath, 503);
  form = await reopenPreferences(page);
  await expect(form.getByRole('alert')).toBeVisible();
  await expect(hearingHours(form)).toHaveCount(0);
  state.preferenceFailures.delete(preferencesPath);
  const fresh = holdPreferenceRequest(state, 'GET', preferencesPath);
  await form.getByRole('button', { name: 'Consultar preferencias', exact: true }).click();
  await expect.poll(() => fresh.entered).toBe(true);
  await expire(page, state, await form.elementHandle());
  form = await reopenPreferences(page);
  await expectPreferenceDraft(form);
  await hearingHours(form).fill('  otra entrada parcial,  ');
  fresh.release();
  await expect(hearingHours(form)).toHaveValue('  otra entrada parcial,  ');
  expect(state.preferenceWrites).toEqual([]);
});
