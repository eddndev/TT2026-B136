import { test, expect } from '@playwright/test';
import { caseId } from './helpers.mjs';
import { measureRecord, measureCaseId } from '../fixtures/measure-records.mjs';
import {
  setupPrecautionaryScheduling,
  openPrecautionaryForm,
  fillPrecautionaryForm,
  preparePrecautionary,
  submitPrecautionary,
  precautionaryEditor,
  precautionaryDetail,
  clone,
} from './precautionary-hearing-scheduling-helpers.mjs';

const inCase = (value) => JSON.parse(JSON.stringify(value).replaceAll(measureCaseId, caseId));
const selectedTargets = (page) =>
  precautionaryEditor(page).getByRole('region', {
    name: 'Medidas seleccionadas para revision',
    exact: true,
  });

async function setupReviewMeasures(page) {
  const state = await setupPrecautionaryScheduling(page);
  const original = inCase(measureRecord()),
    later = inCase(measureRecord({ family: 'm2' }));
  const measures = { original, later, head: original, calls: [] };
  state.expectedReviewTargets = [clone(original.reference)];
  state.recordHistory = clone(original.record_history);
  const base = `/api/v1/cases/${caseId}/measures`;
  await page.route(`**${base}**`, async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    measures.calls.push({ method: request.method(), path: url.pathname, search: url.search });
    expect(request.method()).toBe('GET');
    if (url.pathname === base)
      return route.fulfill({
        json: { case_id: caseId, items: [measures.head], has_more: false, next_after_id: null },
      });
    if (url.pathname === `${base}/${original.reference.id}`)
      return route.fulfill({ json: measures.head });
    for (const value of [original, later]) {
      const ref = value.reference;
      if (
        url.pathname === `${base}/${ref.id}/revisions/${ref.revision}` &&
        url.searchParams.get('capture_digest') === ref.capture_digest
      )
        return route.fulfill({ json: value });
    }
    return route.fulfill({ status: 404, json: { error: { code: 'measure_record_not_found' } } });
  });
  return { state, measures, base };
}

test('a review appointment selects the listed exact measure and retains it after its head changes and the appointment is replaced', async ({
  page,
}) => {
  const { state, measures, base } = await setupReviewMeasures(page);
  const selected = measures.original,
    reference = selected.reference,
    result = selected.record.capture.result;
  await openPrecautionaryForm(page);
  await fillPrecautionaryForm(page);
  const editor = precautionaryEditor(page);
  await editor.getByLabel(/Prop[o\u00f3]sito/).selectOption('review');
  await editor.getByRole('button', { name: 'Elegir medida', exact: true }).click();
  const picker = editor.getByRole('region', { name: 'Seleccionar medida exacta', exact: true });
  const consult = picker.getByRole('button', {
    name: `Consultar medida ${reference.id}`,
    exact: true,
  });
  await expect(consult).toBeVisible();
  measures.head = clone(measures.later);
  await consult.click();
  await expect(picker).toContainText(result.projection.subject.display_name);
  await expect(picker).toContainText(/Presentaci[o\u00f3]n peri[o\u00f3]dica/);
  await expect(picker).toContainText(result.values.conditions);
  await expect(picker).toContainText(reference.capture_digest);
  await picker.getByRole('button', { name: 'Vincular esta revision', exact: true }).click();
  await expect(picker).toHaveCount(0);
  await expect(selectedTargets(page)).toContainText(reference.capture_digest);
  const exactPath = `${base}/${reference.id}/revisions/${reference.revision}`;
  expect(
    measures.calls.some(
      (call) =>
        call.path === exactPath &&
        new URLSearchParams(call.search).get('capture_digest') === reference.capture_digest,
    ),
  ).toBe(true);
  await preparePrecautionary(page);
  await expect(
    editor.getByRole('region', { name: 'Revision de convocatoria cautelar', exact: true }),
  ).toContainText(reference.capture_digest);
  await submitPrecautionary(page);
  await expect(precautionaryDetail(page)).toContainText(/Revisi[o\u00f3]n exacta consultada: 1/);
  const first = state.submissions[0],
    hearingId = first.command.hearing_id;
  expect(first.command.change.values.purpose).toBe('review');
  expect(first.command.change.values.review_targets).toEqual([reference]);
  expect(state.records.get(hearingId)[0].history.record_history).toEqual(selected.record_history);
  expect(measures.head.reference.revision).toBe(2);
  expect(measures.head.last_action).toBe('revoke');
  await page.getByRole('button', { name: 'Reprogramar audiencia cautelar', exact: true }).click();
  await expect(editor.getByLabel(/Prop[o\u00f3]sito/)).toHaveValue('review');
  await expect(selectedTargets(page)).toContainText(reference.capture_digest);
  await editor.getByLabel('Fecha', { exact: true }).fill('2026-10-12');
  await editor
    .getByLabel('Motivo del cambio', { exact: true })
    .fill('Nueva fecha para la revision declarada');
  await preparePrecautionary(page);
  await submitPrecautionary(page);
  await expect(precautionaryDetail(page)).toContainText(/Revisi[o\u00f3]n exacta consultada: 2/);
  expect(state.submissions[1].command.change.values.review_targets).toEqual([reference]);
  expect(state.submissions[1].command.change.values.review_targets).not.toEqual([
    measures.head.reference,
  ]);
  expect(state.records.get(hearingId).at(-1).history.record_history).toEqual(
    selected.record_history,
  );
  expect(state.preparations).toHaveLength(2);
  expect(state.submissions).toHaveLength(2);
  expect(state.unexpected).toEqual([]);
});
