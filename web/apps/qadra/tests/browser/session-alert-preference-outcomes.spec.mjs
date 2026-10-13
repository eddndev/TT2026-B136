import { test, expect } from '@playwright/test';
import { expire } from './session-inactivity-helpers.mjs';
import {
  alertPreferenceDraftSetup,
  checkAlertPreferenceRequests,
  holdPreferenceRequest,
  preferencesPath,
  alertsPath,
} from './session-alert-preference-fixtures.mjs';
import {
  preferenceForm,
  inbox,
  interruptPreferenceSave,
  reopenPreferences,
  expectPreferenceDraft,
  validPreferenceHours,
  retryPreferences,
  savePreferences,
  hearingHours,
} from './session-alert-preference-ui.mjs';

test.afterEach(async ({ page }) => checkAlertPreferenceRequests(page));

test('an interrupted preference save preserves its exact command and requires a new absence check after every expiry before retry', async ({
  page,
}) => {
  const state = await alertPreferenceDraftSetup(page);
  const { sent, command } = await interruptPreferenceSave(page, state);
  let form = await reopenPreferences(page);
  await expectPreferenceDraft(form, validPreferenceHours);
  await expect(hearingHours(form)).toBeDisabled();
  await expect(savePreferences(form)).toBeDisabled();
  await expect(retryPreferences(form)).toHaveCount(0);
  await form.getByRole('button', { name: 'Comprobar guardado', exact: true }).click();
  await expect(retryPreferences(form)).toBeEnabled();
  expect(state.preferenceWrites.map((row) => row.values)).toEqual([command]);
  await expire(page, state, await form.elementHandle());
  form = await reopenPreferences(page);
  await expectPreferenceDraft(form, validPreferenceHours);
  await expect(retryPreferences(form)).toHaveCount(0);
  const check = holdPreferenceRequest(state, 'GET', preferencesPath);
  await form.getByRole('button', { name: 'Comprobar guardado', exact: true }).click();
  await expect.poll(() => check.entered).toBe(true);
  expect(state.preferenceWrites.map((row) => row.values)).toEqual([command]);
  check.release();
  await expect(retryPreferences(form)).toBeEnabled();
  state.nextPreferenceWrite = {};
  await retryPreferences(form).click();
  await expect(preferenceForm(page)).toHaveCount(0);
  expect(state.preferenceWrites.map((row) => row.values)).toEqual([command, command]);
  sent.release();
});

test('only an explicitly checked exact preference receipt confirms the save and clears the draft before a held inbox refresh', async ({
  page,
}) => {
  const state = await alertPreferenceDraftSetup(page);
  const { sent, command } = await interruptPreferenceSave(page, state, true);
  let form = await reopenPreferences(page);
  await expectPreferenceDraft(form, validPreferenceHours);
  await expect(inbox(page).getByText('Preferencias guardadas.', { exact: true })).toHaveCount(0);
  const receipt = holdPreferenceRequest(state, 'GET', preferencesPath);
  const node = await form.elementHandle();
  await form.getByRole('button', { name: 'Comprobar guardado', exact: true }).click();
  await expect.poll(() => receipt.entered).toBe(true);
  await expect(form).toBeVisible();
  receipt.release();
  await expect(preferenceForm(page)).toHaveCount(0);
  await expect(inbox(page).getByText('Preferencias guardadas.', { exact: true })).toBeVisible();
  const refresh = holdPreferenceRequest(state, 'GET', alertsPath);
  await inbox(page).getByRole('button', { name: 'Actualizar alertas', exact: true }).click();
  await expect.poll(() => refresh.entered).toBe(true);
  await expire(page, state, node);
  form = await reopenPreferences(page);
  await expect(hearingHours(form)).toHaveValue('72,12');
  await expect(form.getByRole('button', { name: 'Comprobar guardado', exact: true })).toHaveCount(
    0,
  );
  await expect(retryPreferences(form)).toHaveCount(0);
  await hearingHours(form).fill('  2,  ');
  refresh.release();
  sent.release();
  await expect(hearingHours(form)).toHaveValue('  2,  ');
  expect(state.preferenceWrites.map((row) => row.values)).toEqual([command]);
});

test('equal preference values with a foreign operation receipt require a new comparison decision and never confirm or replay the interrupted command', async ({
  page,
}) => {
  const state = await alertPreferenceDraftSetup(page);
  const { sent, command } = await interruptPreferenceSave(page, state);
  const foreign = {
    ...structuredClone(command),
    operation_id: 'a0000000-0000-4000-8000-000000000072',
  };
  state.commitPreferences(foreign);
  const form = await reopenPreferences(page);
  await expectPreferenceDraft(form, validPreferenceHours);
  await form.getByRole('button', { name: 'Comprobar guardado', exact: true }).click();
  await expect(
    form.getByRole('region', { name: 'Preferencias actuales guardadas', exact: true }),
  ).toContainText('Revisi\u00f3n 1');
  await expect(inbox(page).getByText('Preferencias guardadas.', { exact: true })).toHaveCount(0);
  await expect(retryPreferences(form)).toHaveCount(0);
  await expectPreferenceDraft(form, validPreferenceHours);
  expect(state.preferenceWrites.map((row) => row.values)).toEqual([command]);
  state.nextPreferenceWrite = {};
  await form.getByRole('button', { name: 'Guardar mis preferencias', exact: true }).click();
  await expect(preferenceForm(page)).toHaveCount(0);
  const replacement = state.preferenceWrites[1].values;
  expect(replacement.expected_revision).toBe(1);
  expect(replacement.operation_id).not.toBe(command.operation_id);
  expect(replacement.operation_id).not.toBe(foreign.operation_id);
  expect(replacement.values).toEqual(command.values);
  expect(state.preferenceWrites).toHaveLength(2);
  sent.release();
});
