import { test, expect } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { precautionaryBrowserRecord } from '../fixtures/precautionary-hearing-browser.mjs';
import {
  setupMeasureDecisions,
  openMeasures,
  newDecision,
  fillDecisionCommon,
  fillImposition,
  prepareDecision,
  submitDecision,
  decisionEditor,
  decisionDetail,
  clone,
} from './measure-decision-helpers.mjs';

function seedHistoricalAnchor(state) {
  const scheduling = state.scheduling;
  const command = clone(precautionaryBrowserRecord(state.caseId).capture.review.command);
  command.change.values.venue = 'Sala historica de la convocatoria original';
  command.change.values.scheduling_basis.support = {
    document_id: state.support.id,
    version: state.support.version,
    digest: state.support.digest,
  };
  const original = scheduling.commit(scheduling.prepare(command));
  const replacement = clone(command);
  replacement.operation_id = 'f2000000-0000-4000-8000-000000000002';
  replacement.change = {
    action: 'replace',
    expected_revision: 1,
    expected_capture_digest: original.capture.capture_digest,
    context: clone(scheduling.context.expectation),
    values: clone(command.change.values),
    reason: 'Reprogramacion posterior a la convocatoria seleccionada',
  };
  replacement.change.values.scheduled_at = '2026-01-04T09:00:00-06:00';
  replacement.change.values.venue = 'Sala posterior ajena al ancla seleccionada';
  const current = scheduling.commit(scheduling.prepare(replacement));
  state.expectedAnchor = {
    kind: 'precautionary',
    hearing_id: command.hearing_id,
    revision: 1,
    capture_digest: original.capture.capture_digest,
  };
  state.anchorMaterial = { kind: 'precautionary', capture: clone(original.capture) };
  return { original, current, id: command.hearing_id };
}

test('the complete decision form fits 390px and retains a historical precautionary anchor after its appointment changed', async ({
  page,
}) => {
  const width = 390;
  await page.setViewportSize({ width, height: 1000 });
  const state = await setupMeasureDecisions(page);
  const { original, current, id } = seedHistoricalAnchor(state);
  await openMeasures(page);
  await newDecision(page);
  await fillDecisionCommon(page);
  await fillImposition(page);
  const editor = decisionEditor(page);
  await editor
    .getByRole('combobox', { name: 'Vinculo de audiencia', exact: true })
    .selectOption('precautionary');
  const picker = editor.getByRole('region', {
    name: 'Elegir audiencia cautelar de origen',
    exact: true,
  });
  await picker.getByRole('button', { name: /^Consultar convocatoria / }).click();
  await expect(picker).toContainText('Revision 1');
  await expect(picker).toContainText('Revision 2');
  await picker
    .getByRole('button', { name: 'Usar audiencia cautelar revision 1', exact: true })
    .click();
  await expect(picker).toHaveCount(0);
  await expect(editor).toContainText('Audiencia cautelar / Revision 1');
  await expect(editor.getByRole('button', { name: 'Revisar decision', exact: true })).toBeEnabled();
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
  const output = resolve('../output/precautionary-measures/qadra-decisions/visual');
  await mkdir(output, { recursive: true });
  await page.screenshot({ path: `${output}/measure-decision-form-page-390.png`, fullPage: true });
  await prepareDecision(page);
  const anchor = editor.getByRole('region', {
    name: 'Audiencia de origen de la decision',
    exact: true,
  });
  await expect(anchor).toContainText('Sala historica de la convocatoria original');
  await expect(anchor).not.toContainText('Sala posterior ajena al ancla seleccionada');
  expect(state.preparations[0].review.material.anchor).toEqual(state.anchorMaterial);
  await submitDecision(page);
  await expect(decisionDetail(page)).toContainText('Sala historica de la convocatoria original');
  await expect(decisionDetail(page)).not.toContainText(
    'Sala posterior ajena al ancla seleccionada',
  );
  expect(state.submissions[0].command.anchor).toEqual(state.expectedAnchor);
  expect(
    state.scheduling.calls.filter(
      (call) => call.method === 'GET' && call.path === `${state.scheduling.base}/${id}/revisions/1`,
    ),
  ).toHaveLength(1);
  expect(state.scheduling.records.get(id)).toEqual([original, current]);
  expect(state.scheduling.records.get(id).at(-1).capture.review.result_revision).toBe(2);
  expect(state.scheduling.calls.filter((call) => call.method !== 'GET')).toEqual([]);
  expect(state.scheduling.submissions).toEqual([]);
  expect(state.scheduling.hearings.state.submissions).toEqual([]);
  expect(state.preparations).toHaveLength(1);
  expect(state.submissions).toHaveLength(1);
  expect(state.unexpected).toEqual([]);
});
