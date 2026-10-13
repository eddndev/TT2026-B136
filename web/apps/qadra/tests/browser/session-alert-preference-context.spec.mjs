import { test, expect } from '@playwright/test';
import { login, navigate } from './helpers.mjs';
import { expire, signInOther } from './session-inactivity-helpers.mjs';
import {
  alertPreferenceDraftSetup,
  checkAlertPreferenceRequests,
  preferencesPath,
  principalPath,
} from './session-alert-preference-fixtures.mjs';
import {
  openPreferenceDraft,
  enterPreferenceInbox,
  beginPreferences,
  fillPreferenceDraft,
  preferenceForm,
  inbox,
  hearingHours,
} from './session-alert-preference-ui.mjs';

test.afterEach(async ({ page }) => checkAlertPreferenceRequests(page));

test('fresh loss of the personal preference role discards the capture before reading or exposing it and later permission cannot revive it', async ({
  page,
}) => {
  const state = await alertPreferenceDraftSetup(page);
  let form = await openPreferenceDraft(page);
  await fillPreferenceDraft(form);
  await expire(page, state, await form.elementHandle());
  await login(page, false, false);
  await enterPreferenceInbox(page);
  state.identityOverride = { ...state.current.user, role: 'client' };
  const start = state.calls.length;
  await inbox(page).getByRole('button', { name: 'Preferencias de alertas', exact: true }).click();
  await expect
    .poll(() => state.calls.slice(start).some((call) => call.path === principalPath))
    .toBe(true);
  await expect(inbox(page).getByRole('alert')).toContainText(/acceso|permiso/i);
  await expect(preferenceForm(page)).toHaveCount(0);
  expect(state.calls.slice(start).filter((call) => call.path === preferencesPath)).toEqual([]);
  state.identityOverride = null;
  await navigate(page, 'Inicio');
  await enterPreferenceInbox(page);
  form = await beginPreferences(page);
  await expect(hearingHours(form)).toHaveValue('48,24');
  expect(state.preferenceWrites).toEqual([]);
});

test('cancel, explicit logout with a pending capture and a different MFA principal discard personal preference drafts without a save', async ({
  page,
}) => {
  const state = await alertPreferenceDraftSetup(page);
  let form = await openPreferenceDraft(page);
  await fillPreferenceDraft(form);
  await form.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await expect(preferenceForm(page)).toHaveCount(0);
  form = await beginPreferences(page);
  await expect(hearingHours(form)).toHaveValue('48,24');
  await fillPreferenceDraft(form);
  await expire(page, state, await form.elementHandle());
  await login(page, false, false);
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  await login(page, false, false);
  await enterPreferenceInbox(page);
  form = await beginPreferences(page);
  await expect(hearingHours(form)).toHaveValue('48,24');
  await fillPreferenceDraft(form);
  await expire(page, state, await form.elementHandle());
  await signInOther(page);
  await enterPreferenceInbox(page);
  form = await beginPreferences(page);
  await expect(hearingHours(form)).toHaveValue('48,24');
  await hearingHours(form).fill('  500, texto de otra cuenta ');
  await expire(page, state, await form.elementHandle());
  await login(page, false, false);
  await enterPreferenceInbox(page);
  form = await beginPreferences(page);
  await expect(hearingHours(form)).toHaveValue('48,24');
  expect(state.preferenceWrites).toEqual([]);
});
