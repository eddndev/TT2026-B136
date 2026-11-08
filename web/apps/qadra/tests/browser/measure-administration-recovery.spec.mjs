import { test, expect } from '@playwright/test';
import { openSection } from './helpers.mjs';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import {
  setupMeasureAdministrations,
  openAdministration,
  fillAdministration,
  prepareAdministration,
  submitAdministration,
  administrationEditor,
  administrationDetail,
  measurePanel,
  clone,
} from './measure-administration-helpers.mjs';

async function leaveAndResume(page) {
  await openSection(page, 'Resumen');
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
  await expect(administrationEditor(page)).toHaveCount(0);
  await openSection(page, 'Medidas cautelares');
  await expect(measurePanel(page)).toBeVisible();
  const resume = measurePanel(page).getByRole('button', {
    name: 'Retomar rectificacion de medida',
    exact: true,
  });
  await expect(resume).toBeEnabled();
  await resume.click();
  await expect(administrationEditor(page)).toBeVisible();
}

test('a mobile incomplete correction restores raw offset fields and the original target after navigation without POST', async ({
  page,
}) => {
  const width = 390;
  await page.setViewportSize({ width, height: 1000 });
  const state = await setupMeasureAdministrations(page);
  await openAdministration(page, state, 'correct');
  await fillAdministration(page, 'correct');
  const editor = administrationEditor(page);
  const reason = '  Motivo administrativo pendiente\n conservar declaracion  ';
  const conditions = '  Condiciones sin finalizar\n conservar texto declarado  ';
  await editor.getByLabel(/Motivo de rectificaci[o\u00f3]n/, { exact: true }).fill(reason);
  await editor.getByLabel('Condiciones', { exact: true }).fill(conditions);
  await editor
    .getByRole('combobox', { name: /Precisi[o\u00f3]n de inicio de vigencia/, exact: true })
    .selectOption('minute');
  await editor.getByLabel('Fecha de inicio de vigencia', { exact: true }).fill('2026-10-10');
  await editor.getByLabel('Hora de inicio de vigencia', { exact: true }).fill('09:17');
  await editor
    .getByRole('combobox', { name: 'Desfase de inicio de vigencia', exact: true })
    .selectOption('declared');
  await editor.getByLabel('Desfase UTC de inicio de vigencia', { exact: true }).fill('-0');
  await leaveAndResume(page);
  await expect(editor.getByLabel(/Motivo de rectificaci[o\u00f3]n/, { exact: true })).toHaveValue(
    reason,
  );
  await expect(editor.getByLabel('Condiciones', { exact: true })).toHaveValue(conditions);
  await expect(
    editor.getByRole('combobox', { name: /Precisi[o\u00f3]n de inicio de vigencia/, exact: true }),
  ).toHaveValue('minute');
  await expect(editor.getByLabel('Fecha de inicio de vigencia', { exact: true })).toHaveValue(
    '2026-10-10',
  );
  await expect(editor.getByLabel('Hora de inicio de vigencia', { exact: true })).toHaveValue(
    '09:17',
  );
  await expect(editor.getByLabel('Desfase UTC de inicio de vigencia', { exact: true })).toHaveValue(
    '-0',
  );
  await expect(editor).toContainText(state.target.reference.id);
  await expect(editor).toContainText(state.target.reference.capture_digest);
  await expect(
    editor.getByRole('button', { name: 'Confirmar rectificacion', exact: true }),
  ).toHaveCount(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  for (const control of await editor.locator('input, textarea, select, button').all()) {
    if (!(await control.isVisible())) continue;
    const box = await control.boundingBox();
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(width + 1);
  }
  await page.evaluate(async () => {
    document.activeElement?.blur();
    window.scrollTo(0, 0);
    await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  });
  const output = resolve('../../../output/precautionary-measures/qadra-administrations/visual');
  await mkdir(output, { recursive: true });
  await page.screenshot({
    path: `${output}/measure-administration-draft-page-390.png`,
    fullPage: true,
  });
  expect(state.calls.filter((call) => call.method === 'POST')).toEqual([]);
  expect(state.preparations).toEqual([]);
  expect(state.submissions).toEqual([]);
  expect(state.records.get(state.target.reference.id)).toEqual([state.target]);
  expect(state.unexpected).toEqual([]);
});
