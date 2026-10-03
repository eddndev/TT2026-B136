import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { hearingRecord } from '../fixtures/hearings.mjs';
import { childUploadPath } from './session-subject-document-fixtures.mjs';
import {
  hearingDraftSetup,
  checkHearingDraftRequests,
  holdHearingRead,
  casePath,
  hearingPath,
  hearingsPath,
} from './session-hearing-draft-fixtures.mjs';
import {
  enterHearings,
  beginHearing,
  hearingForm,
  fillHearingDraft,
  expectHearingDraft,
  rawHearing,
  reviewHearing,
  openHearingUpload,
  fillHearingFile,
  prepareHearing,
} from './session-hearing-draft-ui.mjs';

test.afterEach(async ({ page }) => checkHearingDraftRequests(page));

test('a sentencing draft restores raw scheduling and its child File only after fresh case authorization', async ({
  page,
}) => {
  const state = await hearingDraftSetup(page, { stage: 'trial' });
  await login(page);
  await enterHearings(page);
  let form = await beginHearing(page);
  await fillHearingDraft(form, true);
  await form
    .getByRole('combobox', { name: 'Tipo de audiencia', exact: true })
    .selectOption('sentencing');
  await form
    .getByLabel('Declaraci\u00f3n del antecedente', { exact: true })
    .fill(rawHearing.statement);
  let upload = await openHearingUpload(page, form);
  const text = '%PDF-1.4\nhearing-owned draft bytes\n%%EOF';
  await fillHearingFile(upload, 'antecedente.pdf', text);
  await expire(page, state, await upload.elementHandle());
  await login(page);
  await enterHearings(page);
  const before = state.calls.length,
    gate = holdHearingRead(state, 'GET', casePath);
  await page.getByRole('button', { name: 'Programar audiencia', exact: true }).click();
  await expect.poll(() => gate.entered).toBe(true);
  await expect(reviewHearing(hearingForm(page))).toBeDisabled();
  expect(state.preparations).toEqual([]);
  expect(state.uploads).toEqual([]);
  gate.release();
  form = hearingForm(page);
  await expectHearingDraft(form, true);
  await expect(form.getByLabel('Declaraci\u00f3n del antecedente', { exact: true })).toHaveValue(
    rawHearing.statement,
  );
  expect(
    state.calls
      .slice(before)
      .some(
        (call) =>
          call.path === `${hearingsPath}/context` &&
          call.headers.authorization === `Bearer ${state.current.token}`,
      ),
  ).toBe(true);
  upload = await openHearingUpload(page, form);
  await expect(upload).toContainText('antecedente.pdf');
  await expect(upload.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue('  pendiente  ');
  state.nextUpload = {};
  await upload.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(upload).not.toBeVisible();
  expect(state.uploads).toHaveLength(1);
  expect(state.uploads[0]).toMatchObject({
    path: childUploadPath,
    filename: 'antecedente.pdf',
    type: 'application/pdf',
    text,
  });
  expect(state.hearingPosts).toEqual([]);
  expect(state.preparations).toEqual([]);
});

test('replacing a hearing preserves the original base and requires explicit comparison after another revision', async ({
  page,
}) => {
  const original = hearingRecord();
  const state = await hearingDraftSetup(page, { hearings: [original] });
  await login(page);
  await enterHearings(page);
  let form = await beginHearing(page, 'replace');
  await fillHearingDraft(form);
  await form.getByLabel('Motivo del cambio', { exact: true }).fill(rawHearing.reason);
  await expire(page, state, await form.elementHandle());
  state.hearings.get(original.id).push({
    ...structuredClone(original),
    revision: 2,
    values: { ...original.values, venue: 'Sede modificada por otra persona' },
    values_digest: 'f'.repeat(64),
    receipt: {
      ...original.receipt,
      action: 'replace',
      expected_revision: 1,
      operation_id: '90000000-0000-4000-8000-000000000009',
      submission_digest: 'f'.repeat(64),
    },
  });
  state.caseRevision = 2;
  await login(page);
  await enterHearings(page);
  form = await beginHearing(page, 'replace');
  await expectHearingDraft(form);
  await expect(form.getByLabel('Motivo del cambio', { exact: true })).toHaveValue(
    rawHearing.reason,
  );
  await expect(reviewHearing(form)).toBeDisabled();
  expect(state.preparations).toEqual([]);
  await form
    .getByRole('button', { name: 'Consultar audiencia y contexto actuales', exact: true })
    .click();
  await expect(
    form.getByRole('region', { name: 'Registro consultado para comparar', exact: true }),
  ).toContainText('Sede modificada por otra persona');
  await expectHearingDraft(form);
  await form
    .getByRole('button', { name: 'Usar base consultada y revisar borrador', exact: true })
    .click();
  await prepareHearing(state, form);
  expect(state.preparations[0].values.change).toMatchObject({
    action: 'replace',
    expected_revision: 2,
    expected_case_revision: 2,
    reason: rawHearing.reason.trim(),
    values: { venue: rawHearing.venue.trim() },
  });
  expect(state.hearingPosts).toEqual([]);
});

test('an interrupted cancellation is retained and confirmed only from its exact receipt before a pending refresh', async ({
  page,
}) => {
  const state = await hearingDraftSetup(page, { hearings: [hearingRecord()] });
  await login(page);
  await enterHearings(page);
  let form = await beginHearing(page, 'cancel');
  await form.getByLabel('Motivo de cancelaci\u00f3n', { exact: true }).fill(rawHearing.reason);
  await prepareHearing(state, form);
  const sent = holdHearingRead(state, 'POST', `${hearingPath}/cancellation`);
  state.nextHearingWrite = { status: 503 };
  await form.getByRole('button', { name: 'Confirmar registro', exact: true }).click();
  await expect.poll(() => sent.entered).toBe(true);
  const original = structuredClone(state.hearingPosts[0].values);
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterHearings(page);
  await page.getByRole('button', { name: 'Retomar borrador de audiencia', exact: true }).click();
  form = hearingForm(page);
  await expect(form.getByLabel('Motivo de cancelaci\u00f3n', { exact: true })).toHaveValue(
    rawHearing.reason,
  );
  await expect(reviewHearing(form)).toBeDisabled();
  expect(state.hearingPosts).toHaveLength(1);
  const exact = holdHearingRead(state, 'GET', `${hearingPath}/revisions/2`);
  const refresh = holdHearingRead(state, 'GET', hearingsPath);
  await form
    .getByRole('button', { name: 'Consultar resultado del env\u00edo', exact: true })
    .click();
  await expect.poll(() => exact.entered).toBe(true);
  expect(refresh.entered).toBe(false);
  exact.release();
  await expect.poll(() => refresh.entered).toBe(true);
  const element = await form.elementHandle();
  await expire(page, state, element);
  await login(page);
  await enterHearings(page);
  await expect(
    page.getByRole('button', { name: 'Retomar borrador de audiencia', exact: true }),
  ).toHaveCount(0);
  refresh.release();
  sent.release();
  expect(state.hearingPosts.map((row) => row.values)).toEqual([original]);
  expect(state.preparations).toHaveLength(1);
});
