import { test, expect } from '@playwright/test';
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
  await page.getByRole('link', { name: 'Resumen', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
  await expect(administrationEditor(page)).toHaveCount(0);
  await page.getByRole('link', { name: 'Medidas cautelares', exact: true }).click();
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
  const output = resolve('../output/precautionary-measures/qadra-administrations/visual');
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

test('navigation retains an uncertain identity replacement and recovers its exact original IDs without another POST', async ({
  page,
}) => {
  const state = await setupMeasureAdministrations(page);
  state.loseResponse = true;
  await openAdministration(page, state, 'replace_entered_in_error');
  await fillAdministration(page, 'replace_entered_in_error');
  await prepareAdministration(page);
  await submitAdministration(page);
  const editor = administrationEditor(page);
  await expect(
    editor.getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  const sent = clone(state.submissions[0]),
    prepared = clone(state.preparations[0]);
  const receipt = clone(state.operations.get(sent.command.operation_id));
  const replacementId = sent.command.action.replacement_id;
  const identities = [...state.records.keys()].sort();
  expect(sent.expected_submission_digest).toBe(prepared.submission_digest);
  expect(sent.expected_review_digest).toBe(prepared.review_digest);
  await leaveAndResume(page);
  await expect(
    editor.getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  const check = editor.getByRole('button', { name: 'Consultar resultado', exact: true });
  await expect(check).toBeEnabled();
  expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(2);
  expect(state.preparations).toEqual([prepared]);
  expect(state.submissions).toEqual([sent]);
  await check.click();
  await expect(editor).toHaveCount(0);
  const detail = administrationDetail(page);
  await expect(detail).toContainText(sent.command.operation_id);
  await expect(detail).toContainText(sent.command.target.id);
  await expect(detail).toContainText(replacementId);
  await expect(detail).toContainText('Persona sustituta declarada');
  await expect(detail).toContainText(sent.expected_submission_digest);
  await expect(detail).toContainText(sent.expected_review_digest);
  expect(
    state.calls.filter(
      (call) => call.method === 'GET' && call.path === `${state.base}/${sent.command.operation_id}`,
    ),
  ).toHaveLength(1);
  expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(2);
  expect(state.submissions).toEqual([sent]);
  expect(state.operations.get(sent.command.operation_id)).toEqual(receipt);
  expect([...state.records.keys()].sort()).toEqual(identities);
  expect(state.records.get(replacementId)).toHaveLength(1);
  expect(state.records.get(sent.command.target.id)).toHaveLength(2);
  expect(state.unexpected).toEqual([]);
});

test('case closure after preparation blocks a correction and preserves its draft across navigation', async ({
  page,
}) => {
  const state = await setupMeasureAdministrations(page);
  await openAdministration(page, state, 'correct');
  await fillAdministration(page, 'correct');
  await prepareAdministration(page);
  for (const context of [
    state.context,
    state.decisions.context,
    state.decisions.scheduling.context,
  ]) {
    Object.assign(context.administration, {
      revision: 2,
      administrative_status: 'closed',
      changed_at: '2026-10-09T12:00:00Z',
      values_digest: 'c'.repeat(64),
    });
    context.context_digest = 'd'.repeat(64);
    context.expectation = {
      administration_revision: 2,
      stage_revision: 1,
      context_digest: context.context_digest,
    };
  }
  Object.assign(state.decisions.scheduling.hearings.state.admin.administration, {
    revision: 2,
    administrative_status: 'closed',
  });
  await submitAdministration(page);
  const editor = administrationEditor(page);
  await expect(editor).toContainText('Expediente cerrado administrativamente');
  await expect(
    editor
      .getByRole('button', { name: 'Confirmar rectificacion', exact: true })
      .and(page.locator(':enabled')),
  ).toHaveCount(0);
  expect(state.submissions).toEqual([]);
  await leaveAndResume(page);
  await expect(editor).toContainText('Expediente cerrado administrativamente');
  await expect(editor.getByLabel(/Motivo de rectificaci[o\u00f3]n/, { exact: true })).toHaveValue(
    'Rectificacion administrativa declarada en soporte',
  );
  await expect(editor.getByLabel('Condiciones', { exact: true })).toHaveValue(
    'Texto de condiciones rectificado expresamente',
  );
  await expect(editor.getByLabel(/Texto de supervisi[o\u00f3]n/, { exact: true })).toHaveValue(
    'Texto de supervision rectificado sin nueva ficha',
  );
  await expect(editor).toContainText(state.target.reference.capture_digest);
  await expect(
    editor.getByRole('button', { name: 'Revisar rectificacion', exact: true }),
  ).toBeDisabled();
  expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(1);
  expect(state.preparations).toHaveLength(1);
  expect(state.submissions).toEqual([]);
  expect(state.records.get(state.target.reference.id)).toEqual([state.target]);
  expect(state.unexpected).toEqual([]);
});
