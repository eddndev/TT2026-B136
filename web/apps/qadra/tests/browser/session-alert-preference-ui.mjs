import { expect } from '@playwright/test';
import { login, navigate } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { holdPreferenceRequest, preferencesPath } from './session-alert-preference-fixtures.mjs';

export const preferenceForm = (page) =>
  page.getByRole('region', { name: 'Preferencias de alertas', exact: true });
export const inbox = (page) => page.getByRole('region', { name: 'Mis alertas', exact: true });
export const hearingHours = (form) =>
  form.getByRole('textbox', { name: 'Anticipaciones de audiencias (horas)', exact: true });
export const deadlineHours = (form) =>
  form.getByRole('textbox', { name: 'Anticipaciones de plazos (horas)', exact: true });
export const savePreferences = (form) =>
  form.getByRole('button', { name: 'Guardar preferencias', exact: true });
export const retryPreferences = (form) =>
  form.getByRole('button', { name: 'Reintentar este guardado', exact: true });
export const comparePreferences = (form) =>
  form.getByRole('button', { name: 'Consultar preferencias actuales', exact: true });
export const rawPreferenceHours = { hearing: ' 72,  , 12 ', deadline: '  48,24,  ' };
export const validPreferenceHours = { hearing: '  72, 12  ', deadline: ' 96, 24 ' };

export async function enterPreferenceInbox(page) {
  await navigate(page, 'Alertas');
  await expect(inbox(page)).toBeVisible();
  await expect(
    inbox(page).getByRole('button', { name: 'Actualizar alertas', exact: true }),
  ).toBeEnabled();
}
export async function beginPreferences(page) {
  await inbox(page).getByRole('button', { name: 'Preferencias de alertas', exact: true }).click();
  await expect(preferenceForm(page)).toBeVisible();
  return preferenceForm(page);
}
export async function openPreferenceDraft(page) {
  await login(page, false, false);
  await enterPreferenceInbox(page);
  const form = await beginPreferences(page);
  await expect(hearingHours(form)).toHaveValue('48,24');
  return form;
}
export async function fillPreferenceDraft(form, hours = rawPreferenceHours) {
  await hearingHours(form).fill(hours.hearing);
  await deadlineHours(form).fill(hours.deadline);
  await form.getByRole('checkbox', { name: 'Correo para audiencias', exact: true }).uncheck();
  await form.getByRole('checkbox', { name: 'Internas para plazos', exact: true }).uncheck();
  await form
    .getByRole('checkbox', { name: 'Correo para revisi\u00f3n requerida', exact: true })
    .uncheck();
}
export async function expectPreferenceDraft(form, hours = rawPreferenceHours) {
  await expect(hearingHours(form)).toHaveValue(hours.hearing);
  await expect(deadlineHours(form)).toHaveValue(hours.deadline);
  await expect(
    form.getByRole('checkbox', { name: 'Correo para audiencias', exact: true }),
  ).not.toBeChecked();
  await expect(
    form.getByRole('checkbox', { name: 'Internas para plazos', exact: true }),
  ).not.toBeChecked();
  await expect(
    form.getByRole('checkbox', { name: 'Correo para revisi\u00f3n requerida', exact: true }),
  ).not.toBeChecked();
}
export async function reopenPreferences(page) {
  await login(page, false, false);
  await enterPreferenceInbox(page);
  return beginPreferences(page);
}
export async function interruptPreferenceSave(page, state, commit = false) {
  const form = await openPreferenceDraft(page);
  await fillPreferenceDraft(form, validPreferenceHours);
  const sent = holdPreferenceRequest(state, 'PUT', preferencesPath);
  state.nextPreferenceWrite = { commit, status: 503 };
  await savePreferences(form).click();
  await expect.poll(() => sent.entered).toBe(true);
  const command = structuredClone(state.preferenceWrites[0].values);
  await expire(page, state, await form.elementHandle());
  return { sent, command };
}
