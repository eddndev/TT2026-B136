import { test, expect } from '@playwright/test';
import {
  setupMeasureDecisions,
  openMeasures,
  newDecision,
  fillDecisionCommon,
  fillImposition,
  prepareDecision,
  submitDecision,
  decisionEditor,
  decisionPanel,
  decisionDetail,
  clone,
} from './measure-decision-helpers.mjs';

async function leaveAndResume(page) {
  await page.getByRole('link', { name: 'Resumen', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
  await expect(decisionEditor(page)).toHaveCount(0);
  await page.getByRole('link', { name: 'Medidas cautelares', exact: true }).click();
  await expect(decisionPanel(page)).toBeVisible();
  const resume = decisionPanel(page).getByRole('button', {
    name: 'Retomar decision cautelar',
    exact: true,
  });
  await expect(resume).toBeEnabled();
  await resume.click();
  await expect(decisionEditor(page)).toBeVisible();
}

async function completeDraft(page) {
  await openMeasures(page);
  await newDecision(page);
  await fillDecisionCommon(page);
  await fillImposition(page);
}

test('navigation restores an incomplete decision with exact support unknown reason and partial time input without posting', async ({
  page,
}) => {
  const state = await setupMeasureDecisions(page);
  await completeDraft(page);
  const editor = decisionEditor(page);
  const effect = editor.getByRole('group', { name: 'Efecto 1', exact: true });
  const reason = '  No consta el momento\n conservar motivo declarado  ';
  const justification = '  Justificacion pendiente\n conservar texto original  ';
  await editor
    .getByLabel(/Motivo de tiempo desconocido de decisi[o\u00f3]n/, { exact: true })
    .fill(reason);
  await editor.getByLabel(/Justificaci[o\u00f3]n/, { exact: true }).fill(justification);
  await effect
    .getByLabel(/Precisi[o\u00f3]n de inicio de vigencia/, { exact: true })
    .selectOption('minute');
  await effect.getByLabel('Fecha de inicio de vigencia', { exact: true }).fill('2026-10-10');
  await effect.getByLabel('Hora de inicio de vigencia', { exact: true }).fill('09:17');
  await effect
    .getByRole('combobox', { name: 'Desfase de inicio de vigencia', exact: true })
    .selectOption('declared');
  await effect.getByLabel('Desfase UTC de inicio de vigencia', { exact: true }).fill('-0');
  await leaveAndResume(page);
  await expect(
    editor.getByLabel(/Motivo de tiempo desconocido de decisi[o\u00f3]n/, { exact: true }),
  ).toHaveValue(reason);
  await expect(editor.getByLabel(/Justificaci[o\u00f3]n/, { exact: true })).toHaveValue(
    justification,
  );
  await expect(
    effect.getByLabel(/Precisi[o\u00f3]n de inicio de vigencia/, { exact: true }),
  ).toHaveValue('minute');
  await expect(effect.getByLabel('Fecha de inicio de vigencia', { exact: true })).toHaveValue(
    '2026-10-10',
  );
  await expect(effect.getByLabel('Hora de inicio de vigencia', { exact: true })).toHaveValue(
    '09:17',
  );
  await expect(effect.getByLabel('Desfase UTC de inicio de vigencia', { exact: true })).toHaveValue(
    '-0',
  );
  await expect(editor).toContainText('contrato.pdf');
  await expect(editor).toContainText('Persona declarada');
  await expect(editor.getByRole('button', { name: 'Confirmar decision', exact: true })).toHaveCount(
    0,
  );
  expect(
    state.calls.some(
      (call) =>
        call.method === 'GET' &&
        call.path.endsWith(`/subjects/${state.subject.id}/revisions/${state.subject.revision}`),
    ),
  ).toBe(true);
  expect(state.calls.filter((call) => call.method === 'POST')).toEqual([]);
  expect(state.preparations).toEqual([]);
  expect(state.submissions).toEqual([]);
  expect(state.records.size).toBe(0);
  expect(state.unexpected).toEqual([]);
});

test('navigation retains an uncertain decision operation and both confirmations until exact recovery without another POST', async ({
  page,
}) => {
  const state = await setupMeasureDecisions(page);
  state.loseResponse = true;
  await completeDraft(page);
  await prepareDecision(page);
  await submitDecision(page);
  const editor = decisionEditor(page);
  await expect(
    editor.getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  const sent = clone(state.submissions[0]);
  const receipt = clone(state.operations.get(sent.command.operation_id));
  const prepared = clone(state.preparations[0]);
  expect(sent.expected_submission_digest).toBe(prepared.review.submission_digest);
  expect(sent.expected_review_digest).toBe(prepared.review.review_digest);
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
  await expect(decisionDetail(page)).toContainText('Presentarse cada viernes segun soporte');
  await expect(decisionDetail(page)).toContainText(sent.command.operation_id);
  await expect(decisionDetail(page)).toContainText(sent.expected_submission_digest);
  await expect(decisionDetail(page)).toContainText(sent.expected_review_digest);
  expect(
    state.calls.filter(
      (call) =>
        call.method === 'GET' &&
        call.path === `${state.base}/operations/${sent.command.operation_id}`,
    ),
  ).toHaveLength(1);
  expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(2);
  expect(state.preparations).toEqual([prepared]);
  expect(state.submissions).toEqual([sent]);
  expect(state.operations.get(sent.command.operation_id)).toEqual(receipt);
  expect(state.records.get(sent.command.outcome.effects[0].proposal.id)).toHaveLength(1);
  expect(state.unexpected).toEqual([]);
});

test('case closure after preparation blocks decision submission and preserves its draft across navigation', async ({
  page,
}) => {
  const state = await setupMeasureDecisions(page);
  await completeDraft(page);
  await prepareDecision(page);
  for (const context of [state.context, state.scheduling.context]) {
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
  Object.assign(state.scheduling.hearings.state.admin.administration, {
    revision: 2,
    administrative_status: 'closed',
  });
  await submitDecision(page);
  const editor = decisionEditor(page);
  await expect(editor).toContainText('Expediente cerrado administrativamente');
  await expect(
    editor
      .getByRole('button', { name: 'Confirmar decision', exact: true })
      .and(page.locator(':enabled')),
  ).toHaveCount(0);
  expect(state.preparations).toHaveLength(1);
  expect(state.submissions).toEqual([]);
  await leaveAndResume(page);
  await expect(editor).toContainText('Expediente cerrado administrativamente');
  await expect(editor.getByLabel('Autoridad', { exact: true })).toHaveValue(
    'Juzgado declarado en soporte',
  );
  await expect(
    editor.getByLabel(/Motivo de tiempo desconocido de decisi[o\u00f3]n/, { exact: true }),
  ).toHaveValue('No consta el momento de decision');
  await expect(editor.getByLabel('Condiciones', { exact: true })).toHaveValue(
    'Presentarse cada viernes segun soporte',
  );
  await expect(editor).toContainText('contrato.pdf');
  await expect(editor).toContainText('Persona declarada');
  await expect(
    editor.getByRole('button', { name: 'Revisar decision', exact: true }),
  ).toBeDisabled();
  await expect(
    editor
      .getByRole('button', { name: 'Confirmar decision', exact: true })
      .and(page.locator(':enabled')),
  ).toHaveCount(0);
  expect(state.preparations).toHaveLength(1);
  expect(state.submissions).toEqual([]);
  expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(1);
  expect(state.records.size).toBe(0);
  expect(state.unexpected).toEqual([]);
});
