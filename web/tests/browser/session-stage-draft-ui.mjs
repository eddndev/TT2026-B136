import { expect } from '@playwright/test';
import { chooseSupport, fillDate } from './stage-helpers.mjs';
import { openChildUpload } from './session-subject-draft-ui.mjs';
import { casePath, stagePath } from './session-stage-draft-fixtures.mjs';

export { chooseSupport, fillDate, openChildUpload };
export const stageForm = (page) => page.locator('.stage-form');
export const rawStage = {
  note: '  Nota declarada sin terminar\n  segunda linea  ',
  reason: '  Antecedente declarado todavia parcial  ',
  court: '  Tribunal receptor pendiente  ',
  receipt: '  Referencia escrita por quien captura  ',
  offset: '-0',
};
export const stageGroup = (form, label) => form.getByRole('group', { name: label, exact: true });
export const reviewStage = (form) =>
  form.getByRole('button', { name: 'Revisar registro', exact: true });

export async function enterStages(page) {
  await page.getByRole('link', { name: 'Etapas', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Etapas del expediente', exact: true }),
  ).toBeVisible();
  await expect(page.locator('.case-stages')).toHaveAttribute('aria-busy', 'false');
}
export async function beginStage(page, action = 'intermediate') {
  const name =
    action === 'adoption'
      ? 'Registrar etapa actual'
      : action === 'intermediate'
        ? 'Registrar paso a Intermedia'
        : 'Registrar paso a Juicio';
  await page.locator('.stage-current').getByRole('button', { name, exact: true }).click();
  await expect(stageForm(page)).toBeVisible();
  return stageForm(page);
}
export async function resumeStage(page) {
  await page.getByRole('button', { name: 'Retomar borrador de etapa', exact: true }).click();
  await expect(stageForm(page)).toBeVisible();
  return stageForm(page);
}
export async function fillIntermediate(form, invalid = false) {
  const date = stageGroup(form, 'Fecha de la acusaci\u00f3n');
  await date.getByLabel('Precisi\u00f3n', { exact: true }).selectOption('instant');
  await date.getByLabel('Fecha', { exact: true }).fill('2026-09-01');
  await date.getByLabel('Hora', { exact: true }).fill('11:23:45');
  await date.getByLabel('Desfase UTC', { exact: true }).fill(invalid ? rawStage.offset : '-06:00');
  await form.getByLabel('Nota (opcional)', { exact: true }).fill(rawStage.note);
}
export async function expectIntermediate(form, invalid = false) {
  const date = stageGroup(form, 'Fecha de la acusaci\u00f3n');
  await expect(date.getByLabel('Precisi\u00f3n', { exact: true })).toHaveValue('instant');
  await expect(date.getByLabel('Fecha', { exact: true })).toHaveValue('2026-09-01');
  await expect(date.getByLabel('Hora', { exact: true })).toHaveValue('11:23:45');
  await expect(date.getByLabel('Desfase UTC', { exact: true })).toHaveValue(
    invalid ? rawStage.offset : '-06:00',
  );
  await expect(form.getByLabel('Nota (opcional)', { exact: true })).toHaveValue(rawStage.note);
}
export async function fillTrial(form) {
  await fillDate(form, 'Fecha de emisi\u00f3n del auto');
  await fillDate(form, 'Fecha de recepci\u00f3n');
  await form.getByLabel('Tribunal receptor', { exact: true }).fill(rawStage.court);
  await form
    .getByLabel('Referencia de recepci\u00f3n (opcional)', { exact: true })
    .fill(rawStage.receipt);
  await form.getByLabel('Nota (opcional)', { exact: true }).fill(rawStage.note);
}
export async function fillStageChild(upload, name, text) {
  await upload.getByLabel('Archivo', { exact: true }).setInputFiles({
    name,
    mimeType: 'application/pdf',
    buffer: Buffer.from(text),
  });
  await upload
    .getByLabel('Clasificaci\u00f3n (opcional)', { exact: true })
    .fill('  Acto declarado  ');
  await upload.getByLabel('Nueva etiqueta', { exact: true }).fill('  etiqueta de etapa  ');
  return upload.elementHandle();
}
export async function prepareIntermediate(page, form) {
  await fillIntermediate(form);
  await chooseSupport(form);
  await reviewStage(form).click();
  await expect(form.locator('.stage-confirmation')).toContainText('revisi\u00f3n esperada 1');
}
export function expectFreshStage(state, before) {
  const calls = state.calls.slice(before);
  const first = calls.findIndex((row) => row.path === casePath && row.method === 'GET');
  const second = calls.findIndex(
    (row, index) => index > first && row.path === stagePath && row.method === 'GET',
  );
  expect(first).toBeGreaterThanOrEqual(0);
  expect(second).toBeGreaterThan(first);
  for (const call of [calls[first], calls[second]])
    expect(call).toMatchObject({
      body: null,
      search: '',
      headers: { authorization: `Bearer ${state.current.token}` },
    });
}
