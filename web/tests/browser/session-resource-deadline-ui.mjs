import { expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { editor, fillDeadline } from './deadline-editor-helpers.mjs';
import { openActivityScope, activityPanel, chooseOldAct } from './session-activity-draft-ui.mjs';
import {
  holdDeadlineRequest,
  compositePath,
  activitiesPath,
} from './session-resource-deadline-fixtures.mjs';
import { deadlinesPath } from './session-resource-deadline-reads.mjs';

export { editor, openActivityScope, activityPanel, chooseOldAct };
export const rawDeadline = {
  title: '  Plazo contextual pendiente  ',
  source: '  Fuente pendiente\n conservar declaracion  ',
  statement: '  Aplicabilidad sin completar\n  ',
  locator: '  pagina pendiente  ',
};
export const prepareDeadlineButton = (form) =>
  form.getByRole('button', { name: 'Preparar plazo y vinculo', exact: true });
export const confirmDeadlineButton = (form) =>
  form.getByRole('button', { name: 'Confirmar plazo y vinculo', exact: true });
export const retryDeadlineButton = (form) =>
  form.getByRole('button', { name: 'Reintentar envio exacto', exact: true });
export const originDeadlineButton = (form) =>
  form.getByRole('button', { name: 'Confirmar origen del env\u00edo', exact: true });
export const titleField = (form) => form.getByLabel('T\u00edtulo del plazo', { exact: true });

export async function beginResourceDeadline(page, resume = false) {
  await activityPanel(page)
    .getByRole('button', {
      name: resume ? 'Retomar borrador de plazo' : 'Crear plazo',
      exact: true,
    })
    .click();
  await expect(editor(page)).toBeVisible();
  return editor(page);
}
export async function fillResourceDeadline(page, title = rawDeadline.title) {
  await fillDeadline(page, title);
  await editor(page)
    .getByRole('combobox', { name: 'Cuando cambie el perfil', exact: true })
    .selectOption('fixed');
}
export async function prepareResourceDeadline(state, form, approve = true) {
  state.compositePrepareBudget++;
  await prepareDeadlineButton(form).click();
  await expect(confirmDeadlineButton(form)).toBeDisabled();
  const acknowledgement = form.getByRole('checkbox', {
    name: 'Reconozco el resultado y las capturas seleccionadas',
    exact: true,
  });
  await expect(acknowledgement).not.toBeChecked();
  if (approve) {
    await acknowledgement.check();
    await expect(confirmDeadlineButton(form)).toBeEnabled();
  }
}
export async function startResourceDeadline(page, state) {
  await login(page);
  state.seedResourceActs();
  await openActivityScope(page, state, 1);
  const form = await beginResourceDeadline(page);
  await fillResourceDeadline(page);
  return form;
}
export async function interruptResourceDeadline(page, state, { commit = false } = {}) {
  const form = await startResourceDeadline(page, state);
  await prepareResourceDeadline(state, form);
  const sent = holdDeadlineRequest(state, 'POST', `${compositePath(state.original.id)}/submit`);
  state.nextCompositeWrite = { commit, status: 503 };
  await confirmDeadlineButton(form).click();
  await expect.poll(() => sent.entered).toBe(true);
  const submitted = structuredClone(state.compositePosts[0].values);
  const draft = structuredClone(state.compositeDrafts.get(submitted.command.deadline.operation_id));
  await expire(page, state, await form.elementHandle());
  return { sent, submitted, draft };
}
export async function reopenResourceDeadline(page, state, resume = false) {
  await login(page);
  await openActivityScope(page, state, 1);
  return beginResourceDeadline(page, resume);
}
export async function checkBothReceipts(state, form, submitted) {
  const start = state.calls.length;
  await form.getByRole('button', { name: 'Consultar resultado', exact: true }).click();
  const paths = [
    `${deadlinesPath}/${submitted.command.deadline.deadline_id}/revisions/1`,
    `${activitiesPath(submitted.command.resource_id)}/${submitted.command.association_id}/revisions/1`,
  ];
  for (const path of paths) {
    await expect
      .poll(
        () =>
          state.calls.slice(start).filter((call) => call.path === path && call.method === 'GET')
            .length,
      )
      .toBe(1);
  }
}
