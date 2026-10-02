import { expect } from '@playwright/test';
import { resolutionId, notificationId } from './session-fact-draft-fixtures.mjs';
import {
  factEditor,
  factList,
  factNoun,
  fillResolution,
  fillNotification,
} from './procedural-facts-helpers.mjs';

export { factEditor, factList, fillResolution, fillNotification };
export const mainSource = 'Procedencia de la notificaci\u00f3n';
export const representationSource = 'Procedencia de la representaci\u00f3n';
export const resolutionSource = 'Procedencia de la resoluci\u00f3n';
export const rawFact = {
  summary: '  Relato incompleto\n  no acredita efectos  ',
  subtype: '  Calificacion parcial  ',
  issuer: '  Emisor sin terminar  ',
  reference: '  Referencia externa por comprobar  ',
  scope: '  Alcance declarado\n  sin acreditar facultades  ',
  reason: '  Motivo todavia parcial  ',
  picker: '  Ficha sin buscar  ',
};
export const prepareButton = (form) =>
  form.getByRole('button', { name: 'Preparar registro', exact: true });
export const confirmButton = (form) =>
  form.getByRole('button', { name: 'Confirmar registro', exact: true });
export async function enterFacts(page) {
  await page.getByRole('link', { name: 'Resoluciones', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Resoluciones y notificaciones', exact: true }),
  ).toBeVisible();
  await expect(
    factList(page).getByRole('button', { name: 'Actualizar registros', exact: true }),
  ).toBeEnabled();
}
export async function openNotifications(page, parentId = resolutionId) {
  await factList(page)
    .getByRole('button', { name: `Consultar resoluci\u00f3n ${parentId}`, exact: true })
    .click();
  await page.getByRole('button', { name: 'Ver notificaciones', exact: true }).click();
  await expect(
    factList(page, 'notification').getByRole('button', {
      name: 'Actualizar registros',
      exact: true,
    }),
  ).toBeEnabled();
}
export async function beginFact(page, family = 'resolution', action = 'record', id) {
  if (action !== 'record') {
    const resource = id ?? (family === 'resolution' ? resolutionId : notificationId);
    await factList(page, family)
      .getByRole('button', { name: `Consultar ${factNoun(family)} ${resource}`, exact: true })
      .click();
  }
  const verb = { record: 'Registrar', correct: 'Corregir', withdraw: 'Retirar' }[action];
  await page.getByRole('button', { name: `${verb} ${factNoun(family)}`, exact: true }).click();
  const form = factEditor(page, family);
  await expect(form).toBeVisible();
  return form;
}
export async function prepareDraft(state, form) {
  state.factPrepareBudget++;
  await prepareButton(form).click();
  await expect(confirmButton(form)).toBeEnabled();
}
export async function externalSource(form, label) {
  await form.getByRole('combobox', { name: label, exact: true }).selectOption('external_reference');
  await form.getByLabel(`Referencia externa: ${label}`, { exact: true }).fill(rawFact.reference);
}
export async function openFactUpload(page, form, label) {
  await form.getByRole('button', { name: `Cargar soporte: ${label}`, exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Subir documento', exact: true });
  await expect(modal).toBeVisible();
  return modal;
}
export async function chooseFactFile(modal, name, bytes) {
  await modal.getByLabel('Archivo', { exact: true }).setInputFiles({
    name,
    mimeType: 'application/pdf',
    buffer: Buffer.from(bytes),
  });
  await modal
    .getByLabel('Clasificaci\u00f3n (opcional)', { exact: true })
    .fill('  Clasificacion parcial  ');
  await modal.getByLabel('Nueva etiqueta', { exact: true }).fill('  etiqueta pendiente  ');
}
export async function declaredRepresentation(form) {
  await form
    .getByRole('combobox', { name: 'Representaci\u00f3n declarada', exact: true })
    .selectOption('declared');
  for (const label of ['Persona representada', 'Persona representante']) {
    await form.getByRole('combobox', { name: label, exact: true }).selectOption('unlinked');
    await form
      .getByLabel(`Nombre declarado: ${label}`, { exact: true })
      .fill(`  ${label} parcial  `);
    await form
      .getByLabel(`Descripci\u00f3n: ${label}`, { exact: true })
      .fill('  Descripcion sin terminar  ');
  }
  await form.getByLabel('Alcance de la representaci\u00f3n', { exact: true }).fill(rawFact.scope);
  await externalSource(form, representationSource);
}
export async function chooseHistoricalPerson(form, participant) {
  await form
    .getByRole('combobox', { name: 'Destinatario declarado', exact: true })
    .selectOption('participant');
  await form
    .getByRole('button', { name: 'Elegir ficha: Destinatario declarado', exact: true })
    .click();
  const picker = form.getByRole('region', { name: 'Elegir ficha hist\u00f3rica', exact: true });
  await picker
    .getByRole('button', { name: `Consultar historia de ficha ${participant.id}`, exact: true })
    .click();
  await picker
    .getByRole('button', {
      name: `Consultar ficha revisi\u00f3n ${participant.revision}`,
      exact: true,
    })
    .click();
  await picker.getByRole('button', { name: 'Vincular esta revisi\u00f3n', exact: true }).click();
}
export async function chooseHistoricalResult(form, label, hearing, result) {
  await form.getByRole('combobox', { name: label, exact: true }).selectOption('hearing_result');
  await form.getByRole('button', { name: `Elegir resultado: ${label}`, exact: true }).click();
  const picker = form.getByRole('region', { name: 'Elegir resultado hist\u00f3rico', exact: true });
  await picker
    .getByRole('button', { name: `Consultar resultados de audiencia ${hearing.id}`, exact: true })
    .click();
  await picker
    .getByRole('button', { name: `Consultar revisiones de resultado ${result.id}`, exact: true })
    .click();
  await picker
    .getByRole('button', {
      name: `Consultar resultado revisi\u00f3n ${result.revision}`,
      exact: true,
    })
    .click();
  await picker
    .getByRole('combobox', { name: 'Acuerdo de origen', exact: true })
    .selectOption(result.values.agreements[0].id);
  await picker.getByRole('button', { name: 'Vincular este resultado exacto', exact: true }).click();
  await form
    .getByLabel(`Localizador en el resultado: ${label}`, { exact: true })
    .fill('  Acuerdo declarado  ');
}
