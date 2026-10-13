import { test, expect } from '@playwright/test';
import {
  setupPrecautionaryScheduling,
  openPrecautionaryForm,
  fillPrecautionaryForm,
  preparePrecautionary,
  submitPrecautionary,
  precautionaryEditor,
  precautionaryPanel,
  precautionaryDetail,
  clone,
} from './precautionary-hearing-scheduling-helpers.mjs';

async function leaveAndReturn(page) {
  await page.getByRole('link', { name: 'Resumen', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
  await expect(precautionaryEditor(page)).toHaveCount(0);
  await page.getByRole('link', { name: 'Audiencias', exact: true }).click();
  await expect(precautionaryPanel(page)).toBeVisible();
  const resume = precautionaryPanel(page).getByRole('button', {
    name: 'Retomar convocatoria cautelar',
    exact: true,
  });
  await expect(resume).toBeEnabled();
  await resume.click();
  await expect(precautionaryEditor(page)).toBeVisible();
}

test('same-account navigation restores incomplete precautionary fields and exact support without writing', async ({
  page,
}) => {
  const state = await setupPrecautionaryScheduling(page);
  await openPrecautionaryForm(page);
  await fillPrecautionaryForm(page);
  const editor = precautionaryEditor(page),
    raw = '  Senalamiento pendiente\n conservar declaracion  ';
  await editor.getByLabel(/Base de se[n\u00f1]alamiento/).fill(raw);
  await editor.getByLabel('Desfase UTC', { exact: true }).fill('-0');
  await editor.getByLabel('Localizador', { exact: true }).fill('  Pagina pendiente  ');
  await leaveAndReturn(page);
  await expect(editor.getByLabel(/Base de se[n\u00f1]alamiento/)).toHaveValue(raw);
  await expect(editor.getByLabel('Desfase UTC', { exact: true })).toHaveValue('-0');
  await expect(editor.getByLabel('Localizador', { exact: true })).toHaveValue(
    '  Pagina pendiente  ',
  );
  await expect(editor.getByLabel('Sede o enlace', { exact: true })).toHaveValue(
    'Sala cautelar declarada',
  );
  await expect(editor).toContainText('contrato.pdf');
  await expect(
    editor.getByRole('button', { name: 'Confirmar convocatoria', exact: true }),
  ).toHaveCount(0);
  expect(state.calls.filter((call) => call.method === 'POST')).toEqual([]);
  expect(state.records.size).toBe(0);
  expect(state.submissions).toEqual([]);
  expect(state.unexpected).toEqual([]);
});

test('navigation preserves an uncertain operation and recovers its original receipt without automatic POST', async ({
  page,
}) => {
  const state = await setupPrecautionaryScheduling(page);
  state.loseResponse = true;
  await openPrecautionaryForm(page);
  await fillPrecautionaryForm(page);
  await preparePrecautionary(page);
  await submitPrecautionary(page);
  const editor = precautionaryEditor(page);
  await expect(
    editor.getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  const sent = clone(state.submissions[0]),
    receipt = clone(state.operations.get(sent.command.operation_id));
  expect(state.preparations).toHaveLength(1);
  expect(state.submissions).toHaveLength(1);
  await leaveAndReturn(page);
  await expect(
    editor.getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  await expect(
    editor.getByRole('button', { name: 'Consultar resultado', exact: true }),
  ).toBeEnabled();
  expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(2);
  expect(state.preparations).toHaveLength(1);
  expect(state.submissions).toEqual([sent]);
  await editor.getByRole('button', { name: 'Consultar resultado', exact: true }).click();
  await expect(editor).toHaveCount(0);
  await expect(precautionaryDetail(page)).toContainText(/Revisi[o\u00f3]n exacta consultada: 1/);
  await expect(precautionaryDetail(page)).toContainText('Sala cautelar declarada');
  expect(
    state.calls.some(
      (call) =>
        call.method === 'GET' &&
        call.path === `${state.base}/operations/${sent.command.operation_id}`,
    ),
  ).toBe(true);
  expect(state.submissions).toEqual([sent]);
  expect(state.operations.get(sent.command.operation_id)).toEqual(receipt);
  expect(state.records.get(sent.command.hearing_id)).toHaveLength(1);
  expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(2);
  expect(state.unexpected).toEqual([]);
});

test('case closure after preparation prevents submitting the reviewed precautionary appointment', async ({
  page,
}) => {
  const state = await setupPrecautionaryScheduling(page);
  await openPrecautionaryForm(page);
  await fillPrecautionaryForm(page);
  await preparePrecautionary(page);
  Object.assign(state.context.administration, {
    revision: 2,
    administrative_status: 'closed',
    changed_at: '2026-10-09T12:00:00Z',
    values_digest: 'c'.repeat(64),
  });
  state.context.context_digest = 'd'.repeat(64);
  state.context.expectation = {
    administration_revision: 2,
    stage_revision: 1,
    context_digest: state.context.context_digest,
  };
  Object.assign(state.hearings.state.admin.administration, {
    revision: 2,
    administrative_status: 'closed',
  });
  await submitPrecautionary(page);
  const editor = precautionaryEditor(page);
  await expect(editor).toContainText('Expediente cerrado administrativamente');
  await expect(
    editor
      .getByRole('button', {
        name: 'Confirmar convocatoria',
        exact: true,
      })
      .and(page.locator(':enabled')),
  ).toHaveCount(0);
  expect(state.preparations).toHaveLength(1);
  expect(state.submissions).toEqual([]);
  expect(state.calls.filter((call) => call.method === 'POST')).toHaveLength(1);
  expect(state.records.size).toBe(0);
  expect(state.unexpected).toEqual([]);
});
