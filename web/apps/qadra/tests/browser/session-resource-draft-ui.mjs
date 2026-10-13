import { expect } from '@playwright/test';
import { resourceEditor, resourceDetail } from './procedural-resources-helpers.mjs';
export { resourceEditor, resourceDetail };
export const resolutionSupport = 'la resoluci\u00f3n impugnada';
export const rawResource = {
  title: '  Recurso todavia parcial  ',
  grounds: '  Motivos sin terminar\n  segunda linea  ',
  reason: '  Preciso sin reemplazar historia  ',
  statement: '  Acto historico corregido\n  parcial  ',
};
export const prepareButton = (form) =>
  form.getByRole('button', { name: 'Preparar registro', exact: true });
export const confirmButton = (form) =>
  form.getByRole('button', { name: 'Confirmar registro', exact: true });
export async function enterResources(page) {
  await page.getByRole('link', { name: 'Recursos', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Recursos procesales', exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Actualizar recursos', exact: true }),
  ).toBeEnabled();
}
export async function selectResource(page, title = 'Recurso declarado', revision) {
  await page.getByRole('button', { name: `Consultar recurso ${title}`, exact: true }).click();
  if (revision !== undefined) {
    await resourceDetail(page)
      .getByRole('button', { name: 'Ver historial de recurso', exact: true })
      .click();
    await resourceDetail(page)
      .getByRole('button', { name: `Consultar recurso revision ${revision}`, exact: true })
      .click();
  }
  await expect(resourceDetail(page)).toBeVisible();
}
export async function beginResource(page, action = 'register', title, revision) {
  if (action !== 'register') await selectResource(page, title, revision);
  const name = {
    register: 'Registrar recurso',
    correct: 'Corregir recurso',
    archive: 'Archivar recurso',
    reactivate: 'Reactivar recurso',
    record_act: 'Registrar acto',
    correct_act: 'Corregir este acto',
  }[action];
  await page.getByRole('button', { name, exact: true }).click();
  const form = resourceEditor(page);
  await expect(form).toBeVisible();
  return form;
}
export async function prepareResource(state, form) {
  state.resourcePrepareBudget++;
  await prepareButton(form).click();
  await expect(confirmButton(form)).toBeEnabled();
}
