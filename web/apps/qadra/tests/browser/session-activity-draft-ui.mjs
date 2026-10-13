import { expect } from '@playwright/test';
import { enterResources, selectResource } from './session-resource-draft-ui.mjs';
import {
  activityEditor,
  activityPanel,
  activityDetail,
  selectTarget,
} from './resource-activities-helpers.mjs';
export { activityEditor, activityPanel, activityDetail, selectTarget };
export const rawActivityReason = '  Organizacion pendiente\n  conservar captura exacta  ';
export const prepareActivityButton = (form, action = 'link') =>
  form.getByRole('button', {
    name: action === 'link' ? 'Preparar v\u00ednculo' : 'Preparar desvinculaci\u00f3n',
    exact: true,
  });
export const confirmActivityButton = (form, action = 'link') =>
  form.getByRole('button', {
    name: action === 'link' ? 'Confirmar v\u00ednculo' : 'Confirmar desvinculaci\u00f3n',
    exact: true,
  });
export async function openActivityScope(
  page,
  state,
  revision,
  title = state.resource.values.title,
) {
  await enterResources(page);
  await selectResource(page, title, revision);
  await expect(
    activityPanel(page).getByRole('button', { name: 'Actualizar actividades', exact: true }),
  ).toBeEnabled();
}
export async function beginActivity(page, association = null) {
  if (association) {
    await activityPanel(page)
      .getByRole('button', { name: `Consultar v\u00ednculo ${association.id}`, exact: true })
      .click();
    await activityDetail(page)
      .getByRole('button', { name: 'Desvincular actividad', exact: true })
      .click();
  } else
    await activityPanel(page)
      .getByRole('button', { name: 'Vincular actividad', exact: true })
      .click();
  await expect(activityEditor(page)).toBeVisible();
  return activityEditor(page);
}
export async function prepareActivity(state, form, action = 'link') {
  state.activityPrepareBudget++;
  await prepareActivityButton(form, action).click();
  await expect(confirmActivityButton(form, action)).toBeEnabled();
}
export async function chooseOldAct(form) {
  await form.getByRole('button', { name: 'Elegir acto del recurso', exact: true }).click();
  await form.getByRole('combobox', { name: 'Acto del recurso', exact: true }).selectOption('2');
  await form.getByRole('button', { name: 'Usar este acto', exact: true }).click();
}
