import { expect } from '@playwright/test';
import { hearingId } from './session-hearing-draft-fixtures.mjs';
import { resultId } from '../fixtures/hearing-results.mjs';

export const hearingForm = (page) =>
  page.getByRole('region', { name: 'Formulario de audiencia', exact: true });
export const resultForm = (page) =>
  page.getByRole('region', { name: 'Formulario de sesi\u00f3n o acto', exact: true });
export const reviewHearing = (form) =>
  form.getByRole('button', { name: 'Revisar registro', exact: true });
export const reviewResult = (form) =>
  form.getByRole('button', { name: 'Revisar resultado', exact: true });
export const rawHearing = {
  venue: '  Sede declarada todavia parcial  ',
  note: '  Nota sin terminar\n  segunda linea  ',
  reason: '  Motivo por terminar  ',
  statement: '  Antecedente comunicado, no acreditado  ',
};
export const rawResult = {
  summary: '  Relato parcial\n  sin conclusion  ',
  source: '  Localizador por comprobar  ',
  capacity: '  Calidad comunicada  ',
  observation: '  Observacion sin terminar  ',
  first: '  Primer acuerdo parcial  ',
  second: '  Segundo acuerdo parcial  ',
};

export async function enterHearings(page) {
  await page.getByRole('link', { name: 'Audiencias', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Audiencias del expediente', exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Actualizar audiencias', exact: true }),
  ).toBeEnabled();
}
export async function beginHearing(page, action = 'schedule') {
  if (action !== 'schedule') {
    await page
      .getByRole('button', { name: `Consultar audiencia ${hearingId}`, exact: true })
      .click();
  }
  const name =
    action === 'schedule'
      ? 'Programar audiencia'
      : action === 'replace'
        ? 'Corregir o reprogramar'
        : 'Cancelar audiencia';
  await page.getByRole('button', { name, exact: true }).click();
  await expect(hearingForm(page)).toBeVisible();
  return hearingForm(page);
}
export async function openResultList(page) {
  await page.getByRole('button', { name: `Consultar audiencia ${hearingId}`, exact: true }).click();
  await page.getByRole('button', { name: 'Ver sesiones y resultados', exact: true }).click();
  await expect(
    page.getByRole('button', { name: 'Actualizar resultados', exact: true }),
  ).toBeEnabled();
}
export async function beginResult(page, action = 'record') {
  if (action !== 'record')
    await page
      .getByRole('button', { name: `Consultar resultado ${resultId}`, exact: true })
      .click();
  const name =
    action === 'record'
      ? 'Registrar sesi\u00f3n o acto'
      : action === 'correct'
        ? 'Rectificar registro'
        : action === 'continue'
          ? 'Registrar continuaci\u00f3n'
          : 'Retirar registro';
  await page.getByRole('button', { name, exact: true }).click();
  await expect(resultForm(page)).toBeVisible();
  return resultForm(page);
}
export async function fillHearingDraft(form, invalid = false) {
  const time = form.getByRole('group', { name: 'Fecha y hora de la audiencia', exact: true });
  await time.getByLabel('Fecha', { exact: true }).fill('2026-10-01');
  await time.getByLabel('Hora', { exact: true }).fill('09:02:03');
  await time.getByLabel('Desfase UTC', { exact: true }).fill(invalid ? '-0' : '-06:00');
  await form.getByLabel('Sede o conexi\u00f3n', { exact: true }).fill(rawHearing.venue);
  await form.getByLabel('Nota declarada (opcional)', { exact: true }).fill(rawHearing.note);
}
export async function expectHearingDraft(form, invalid = false) {
  await expect(form.getByLabel('Sede o conexi\u00f3n', { exact: true })).toHaveValue(
    rawHearing.venue,
  );
  await expect(form.getByLabel('Nota declarada (opcional)', { exact: true })).toHaveValue(
    rawHearing.note,
  );
  await expect(form.getByLabel('Desfase UTC', { exact: true })).toHaveValue(
    invalid ? '-0' : '-06:00',
  );
}
export async function fillResultDraft(form, invalid = false) {
  await form.getByRole('combobox', { name: 'Ocurrencia', exact: true }).selectOption('occurred');
  await form
    .getByRole('combobox', { name: 'Alcance declarado', exact: true })
    .selectOption('partial');
  const time = form.getByRole('group', { name: 'Tiempo del hecho informado', exact: true });
  await time.getByLabel('Precisi\u00f3n', { exact: true }).selectOption('date');
  await time.getByLabel('Fecha', { exact: true }).fill('2026-09-01');
  await time.getByLabel('Desfase UTC', { exact: true }).fill(invalid ? '-0' : '-06:00');
  await form.getByLabel('Relato del operador', { exact: true }).fill(rawResult.summary);
  await form
    .getByRole('combobox', { name: 'Procedencia', exact: true })
    .selectOption('written_record');
  await form.getByLabel('Localizador de la fuente', { exact: true }).fill(rawResult.source);
}
export async function openHearingUpload(page, form, result = false) {
  if (result) {
    const details = form
      .locator('details')
      .filter({ has: page.locator('summary', { hasText: 'Soporte documental (opcional)' }) })
      .first();
    if (!(await details.evaluate((node) => node.open)))
      await details.locator('summary').first().click();
  }
  await form
    .getByRole('button', {
      name: result ? 'Cargar soporte del resultado' : 'Cargar soporte del antecedente',
      exact: true,
    })
    .click();
  const upload = page.getByRole('dialog', { name: 'Subir documento', exact: true });
  await expect(upload).toBeVisible();
  return upload;
}
export async function fillHearingFile(upload, name, text) {
  await upload
    .getByLabel('Archivo', { exact: true })
    .setInputFiles({ name, mimeType: 'application/pdf', buffer: Buffer.from(text) });
  await upload
    .getByLabel('Clasificaci\u00f3n (opcional)', { exact: true })
    .fill('  Soporte declarado  ');
  await upload.getByLabel('Nueva etiqueta', { exact: true }).fill('  pendiente  ');
}
export async function prepareHearing(state, form) {
  state.prepareBudget++;
  await reviewHearing(form).click();
  await expect(form.getByRole('button', { name: 'Confirmar registro', exact: true })).toBeEnabled();
}
export async function prepareResult(state, form) {
  state.prepareBudget++;
  await reviewResult(form).click();
  await expect(
    form.getByRole('button', { name: 'Confirmar resultado', exact: true }),
  ).toBeEnabled();
}
