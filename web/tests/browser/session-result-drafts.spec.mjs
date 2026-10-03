import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { hearingRecord } from '../fixtures/hearings.mjs';
import { resultRecord, resultId } from '../fixtures/hearing-results.mjs';
import {
  hearingDraftSetup,
  checkHearingDraftRequests,
  holdHearingRead,
  casePath,
  hearingPath,
  participant,
} from './session-hearing-draft-fixtures.mjs';
import {
  enterHearings,
  openResultList,
  beginResult,
  resultForm,
  fillResultDraft,
  rawResult,
  reviewResult,
  prepareResult,
  openHearingUpload,
  fillHearingFile,
} from './session-hearing-draft-ui.mjs';

test.afterEach(async ({ page }) => checkHearingDraftRequests(page));

test('a result draft preserves exact attendee rows, reordered agreement IDs and its own source File across MFA', async ({
  page,
}) => {
  const state = await hearingDraftSetup(page, { hearings: [hearingRecord()] });
  await login(page);
  await enterHearings(page);
  await openResultList(page);
  let form = await beginResult(page);
  await fillResultDraft(form, true);
  await form.getByText('Comparecencias informadas (0/32)', { exact: true }).click();
  await form.getByRole('button', { name: 'Agregar comparecencia', exact: true }).click();
  const picker = form.getByRole('region', { name: 'Elegir ficha hist\u00f3rica', exact: true });
  await picker
    .getByRole('button', { name: `Consultar historia de ficha ${participant.id}`, exact: true })
    .click();
  await picker
    .getByRole('button', { name: 'Consultar ficha revisi\u00f3n 1', exact: true })
    .click();
  await picker
    .getByRole('button', { name: 'Informar comparecencia de esta revisi\u00f3n', exact: true })
    .click();
  await form.getByLabel('Calidad en esta sesi\u00f3n', { exact: true }).fill(rawResult.capacity);
  await form
    .getByLabel('Observaci\u00f3n de comparecencia (opcional)', { exact: true })
    .fill(rawResult.observation);
  await form.getByText('Acuerdos declarados (0/16)', { exact: true }).click();
  await form.getByRole('button', { name: 'Agregar acuerdo declarado', exact: true }).click();
  await form.getByLabel('Texto del acuerdo 1', { exact: true }).fill(rawResult.first);
  await form.getByRole('button', { name: 'Agregar acuerdo declarado', exact: true }).click();
  await form.getByLabel('Texto del acuerdo 2', { exact: true }).fill(rawResult.second);
  const ids = await form
    .getByText('Identificador del acuerdo', { exact: true })
    .locator('..')
    .locator('code')
    .allTextContents();
  expect(ids).toHaveLength(2);
  expect(new Set(ids).size).toBe(2);
  await form.getByRole('button', { name: 'Subir acuerdo 2', exact: true }).click();
  let upload = await openHearingUpload(page, form, true);
  const bytes = '%PDF-1.4\nresult source bytes\n%%EOF';
  await fillHearingFile(upload, 'resultado.pdf', bytes);
  await expire(page, state, await upload.elementHandle());
  state.records.get(participant.id).push({
    ...participant,
    revision: 2,
    display_name: 'Nombre posterior',
    directory_status: 'archived',
  });
  await login(page);
  await enterHearings(page);
  await openResultList(page);
  const gate = holdHearingRead(state, 'GET', casePath),
    before = state.calls.length;
  await page.getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true }).click();
  await expect.poll(() => gate.entered).toBe(true);
  await expect(reviewResult(resultForm(page))).toBeDisabled();
  gate.release();
  form = resultForm(page);
  await expect(form.getByLabel('Relato del operador', { exact: true })).toHaveValue(
    rawResult.summary,
  );
  await expect(form.getByLabel('Desfase UTC', { exact: true })).toHaveValue('-0');
  await form.getByText('Comparecencias informadas (1/32)', { exact: true }).click();
  await expect(form.getByLabel('Calidad en esta sesi\u00f3n', { exact: true })).toHaveValue(
    rawResult.capacity,
  );
  await expect(
    form.getByLabel('Observaci\u00f3n de comparecencia (opcional)', { exact: true }),
  ).toHaveValue(rawResult.observation);
  await form.getByText('Acuerdos declarados (2/16)', { exact: true }).click();
  await expect(form.getByLabel('Texto del acuerdo 1', { exact: true })).toHaveValue(
    rawResult.second,
  );
  await expect(form.getByLabel('Texto del acuerdo 2', { exact: true })).toHaveValue(
    rawResult.first,
  );
  expect(
    await form
      .getByText('Identificador del acuerdo', { exact: true })
      .locator('..')
      .locator('code')
      .allTextContents(),
  ).toEqual([...ids].reverse());
  expect(
    state.calls
      .slice(before)
      .some((row) => row.path.endsWith(`/participants/${participant.id}/revisions/1`)),
  ).toBe(true);
  upload = await openHearingUpload(page, form, true);
  await expect(upload).toContainText('resultado.pdf');
  await expect(upload.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue('  pendiente  ');
  state.nextUpload = {};
  await upload.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(upload).not.toBeVisible();
  expect(state.uploads[0]).toMatchObject({ filename: 'resultado.pdf', text: bytes });
  expect(state.resultPosts).toEqual([]);
  expect(state.preparations).toEqual([]);
});

test('a continuation keeps its historical scheduling and previous result even when both heads change', async ({
  page,
}) => {
  const originalHearing = hearingRecord(),
    originalResult = resultRecord();
  const state = await hearingDraftSetup(page, {
    hearings: [originalHearing],
    results: [originalResult],
  });
  await login(page);
  await enterHearings(page);
  await openResultList(page);
  let form = await beginResult(page, 'continue');
  await fillResultDraft(form);
  await expire(page, state, await form.elementHandle());
  state.hearings.get(originalHearing.id).push({
    ...structuredClone(originalHearing),
    revision: 2,
    status: 'cancelled',
    reason: 'Cancelacion posterior',
    receipt: {
      ...originalHearing.receipt,
      action: 'cancel',
      expected_revision: 1,
      operation_id: '90000000-0000-4000-8000-000000000019',
      expected_context: null,
      submission_digest: 'f'.repeat(64),
    },
  });
  state.results.get(resultId).push({
    ...structuredClone(originalResult),
    revision: 2,
    status: 'withdrawn',
    reason: 'Retiro posterior',
    receipt: {
      ...originalResult.receipt,
      action: 'withdraw',
      expected_revision: 1,
      operation_id: '90000000-0000-4000-8000-000000000029',
      submission_digest: 'f'.repeat(64),
    },
  });
  await login(page);
  await enterHearings(page);
  await openResultList(page);
  const before = state.calls.length;
  await page.getByRole('button', { name: 'Retomar borrador de resultado', exact: true }).click();
  form = resultForm(page);
  await expect(form.getByLabel('Relato del operador', { exact: true })).toHaveValue(
    rawResult.summary,
  );
  await expect(form.locator('.hearing-result-sources')).toContainText('Revisi\u00f3n 1 exacta');
  expect(state.calls.slice(before).some((row) => row.path === `${hearingPath}/revisions/1`)).toBe(
    true,
  );
  expect(
    state.calls
      .slice(before)
      .some((row) => row.path === `${hearingPath}/results/${resultId}/revisions/1`),
  ).toBe(true);
  expect(state.preparations).toEqual([]);
  await prepareResult(state, form);
  expect(state.preparations[0].values.change).toMatchObject({
    action: 'record',
    expected_revision: 0,
    anchor_revision: 1,
    continuation: { result_id: resultId, revision: 1 },
  });
  expect(state.resultPosts).toEqual([]);
});

test('a correction retains its original fields until the fresh result revision is explicitly adopted', async ({
  page,
}) => {
  const original = resultRecord();
  const state = await hearingDraftSetup(page, { hearings: [hearingRecord()], results: [original] });
  await login(page);
  await enterHearings(page);
  await openResultList(page);
  let form = await beginResult(page, 'correct');
  await fillResultDraft(form);
  await form.getByLabel('Motivo', { exact: true }).fill('  Precision propia parcial  ');
  await expire(page, state, await form.elementHandle());
  state.results.get(resultId).push({
    ...structuredClone(original),
    revision: 2,
    values: { ...original.values, summary: 'Otra version del relato' },
    values_digest: 'f'.repeat(64),
    receipt: {
      ...original.receipt,
      action: 'correct',
      expected_revision: 1,
      operation_id: '90000000-0000-4000-8000-000000000039',
      submission_digest: 'f'.repeat(64),
    },
  });
  await login(page);
  await enterHearings(page);
  await openResultList(page);
  form = await beginResult(page, 'correct');
  await expect(form.getByLabel('Relato del operador', { exact: true })).toHaveValue(
    rawResult.summary,
  );
  await expect(reviewResult(form)).toBeDisabled();
  await form
    .getByRole('button', { name: 'Consultar base actual del resultado', exact: true })
    .click();
  await expect(form).toContainText('Otra version del relato');
  await expect(form.getByLabel('Relato del operador', { exact: true })).toHaveValue(
    rawResult.summary,
  );
  expect(state.preparations).toEqual([]);
  await form
    .getByRole('button', { name: 'Usar esta base y conservar borrador', exact: true })
    .click();
  await prepareResult(state, form);
  expect(state.preparations[0].values.change).toMatchObject({
    action: 'correct',
    expected_revision: 2,
    reason: 'Precision propia parcial',
    values: { summary: rawResult.summary.trim() },
  });
  expect(state.resultPosts).toEqual([]);
});
