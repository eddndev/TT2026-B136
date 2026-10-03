import { test, expect } from '@playwright/test';
import { login, navigate } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import {
  alertPreferenceDraftSetup,
  checkAlertPreferenceRequests,
  alertsPath,
  principalPath,
  preferencesPath,
} from './session-alert-preference-fixtures.mjs';
import {
  openPreferenceDraft,
  fillPreferenceDraft,
  enterPreferenceInbox,
  beginPreferences,
  hearingHours,
  deadlineHours,
  inbox,
  preferenceForm,
} from './session-alert-preference-ui.mjs';

test.afterEach(async ({ page }) => checkAlertPreferenceRequests(page));

test('an inbox permission denial discards unopened preference captures before restored permission can expose them', async ({
  page,
}) => {
  const state = await alertPreferenceDraftSetup(page);
  let form = await openPreferenceDraft(page);
  await fillPreferenceDraft(form);
  await expire(page, state, await form.elementHandle());
  await login(page, false, false);
  state.identityOverride = { ...state.current.user, role: 'client' };
  const start = state.calls.length;
  await navigate(page, 'Alertas');
  await expect(inbox(page).getByRole('alert')).toContainText('ya no tiene acceso');
  await expect(
    inbox(page).getByRole('button', {
      name: 'Preferencias de alertas',
      exact: true,
    }),
  ).toBeDisabled();
  await expect(preferenceForm(page)).toHaveCount(0);
  expect(state.calls.slice(start).filter((call) => call.path === alertsPath)).toHaveLength(1);
  expect(
    state.calls.slice(start).filter((call) => [principalPath, preferencesPath].includes(call.path)),
  ).toEqual([]);
  state.identityOverride = null;
  await navigate(page, 'Inicio');
  await enterPreferenceInbox(page);
  form = await beginPreferences(page);
  await expect(hearingHours(form)).toHaveValue('48,24');
  await expect(deadlineHours(form)).toHaveValue('48,24');
  await expect(
    form.getByRole('checkbox', {
      name: 'Correo para audiencias',
      exact: true,
    }),
  ).toBeChecked();
  expect(state.preferenceWrites).toEqual([]);
});
