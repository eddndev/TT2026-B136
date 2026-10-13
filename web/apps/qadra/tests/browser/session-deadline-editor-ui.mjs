import { expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { editor, fillDeadline } from './deadline-editor-helpers.mjs';
import { holdDeadlineRead, deadlinesPath } from './session-deadline-editor-fixtures.mjs';

export { editor };
export const rawDeadline = {
  title: '  Plazo ordinario sin terminar  ',
  source: '  Fuente pendiente\n  conservar texto  ',
  statement: '  Aplicabilidad parcial\n  sin concluir  ',
  locator: '  Localizador incompleto  ',
  reason: '  Motivo pendiente\n  segunda linea  ',
};
export const titleField = (form) => form.getByLabel('T\u00edtulo del plazo', { exact: true });
export const prepareButton = (form) =>
  form.getByRole('button', { name: 'Preparar plazo', exact: true });
export const confirmButton = (form) =>
  form.getByRole('button', { name: 'Confirmar plazo', exact: true });
export const detailPanel = (page) =>
  page.getByRole('region', { name: 'Detalle de plazo', exact: true });

export async function enterDeadlines(page) {
  await page.getByRole('link', { name: 'Plazos', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Actualizar plazos', exact: true })).toBeEnabled();
}
export async function beginDeadline(page, mode = 'register', record = null) {
  if (!['register', 'resume'].includes(mode)) {
    await page.getByRole('button', { name: `Consultar plazo ${record.id}`, exact: true }).click();
    await expect(detailPanel(page)).toBeVisible();
  }
  const name = {
    register: 'Registrar plazo',
    resume: 'Retomar borrador de plazo',
    correct: 'Corregir plazo',
    set_attention: 'Declarar atenci\u00f3n',
    retire: 'Retirar plazo',
  }[mode];
  await page.getByRole('button', { name, exact: true }).click();
  await expect(editor(page)).toBeVisible();
  return editor(page);
}
export async function fillOrdinaryDeadline(page) {
  await fillDeadline(page, rawDeadline.title);
  await editor(page)
    .getByRole('combobox', { name: 'Cuando cambie el perfil', exact: true })
    .selectOption('fixed');
}
export async function prepareDeadline(state, form, approve = true) {
  state.deadlinePrepareBudget++;
  await prepareButton(form).click();
  await expect(confirmButton(form)).toBeDisabled();
  const checkbox = form.getByRole('checkbox', { name: /Revise las declaraciones/ });
  await expect(checkbox).not.toBeChecked();
  if (approve) {
    await checkbox.check();
    await expect(confirmButton(form)).toBeEnabled();
  }
}
export async function reopenDeadline(page, mode = 'resume', record = null) {
  await login(page);
  await enterDeadlines(page);
  return beginDeadline(page, mode, record);
}
export async function interruptDeadline(page, state, { commit = false, record = null } = {}) {
  await login(page);
  await enterDeadlines(page);
  const form = await beginDeadline(page, record ? 'retire' : 'register', record);
  if (record) await form.getByLabel('Motivo', { exact: true }).fill(rawDeadline.reason);
  else await fillOrdinaryDeadline(page);
  await prepareDeadline(state, form);
  const path = record ? `${deadlinesPath}/${record.id}/retirement` : deadlinesPath;
  const sent = holdDeadlineRead(state, 'POST', path);
  state.nextDeadlineWrite = { commit, status: 503 };
  await confirmButton(form).click();
  await expect.poll(() => sent.entered).toBe(true);
  const submitted = structuredClone(state.deadlinePosts[0].values);
  const prepared = structuredClone(state.deadlineDrafts.get(submitted.command.operation_id));
  await expire(page, state, await form.elementHandle());
  return { sent, submitted, prepared };
}
