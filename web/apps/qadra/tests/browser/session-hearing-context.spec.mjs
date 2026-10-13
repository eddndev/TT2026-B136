import { test, expect } from '@playwright/test';
import { login, selectCase } from './helpers.mjs';
import { expire, signInOther } from './session-inactivity-helpers.mjs';
import { hearingRecord } from '../fixtures/hearings.mjs';
import {
  hearingDraftSetup,
  checkHearingDraftRequests,
  hearingPath,
} from './session-hearing-draft-fixtures.mjs';
import {
  enterHearings,
  beginHearing,
  hearingForm,
  fillHearingDraft,
  rawHearing,
  reviewHearing,
  openResultList,
  beginResult,
  resultForm,
  fillResultDraft,
  rawResult,
  reviewResult,
  openHearingUpload,
  fillHearingFile,
} from './session-hearing-draft-ui.mjs';

test.afterEach(async ({ page }) => checkHearingDraftRequests(page));

test('closed hearing drafts remain readonly while explicit closure, another principal and denied access discard their scopes', async ({
  page,
}) => {
  const state = await hearingDraftSetup(page, { hearings: [hearingRecord()] });
  await login(page);
  await enterHearings(page);
  let form = await beginHearing(page, 'replace');
  await fillHearingDraft(form);
  await expire(page, state, await form.elementHandle());
  state.caseStatus = 'closed';
  state.caseRevision++;
  await login(page);
  await enterHearings(page);
  await page.getByRole('button', { name: 'Retomar borrador de audiencia', exact: true }).click();
  form = hearingForm(page);
  await expect(form.getByLabel('Sede o conexi\u00f3n', { exact: true })).toHaveValue(
    rawHearing.venue,
  );
  await expect(form.getByLabel('Sede o conexi\u00f3n', { exact: true })).toBeDisabled();
  await expect(reviewHearing(form)).toBeDisabled();
  const element = await form.elementHandle();
  await form.getByRole('button', { name: 'Cerrar formulario de audiencia', exact: true }).click();
  await expire(page, state, element);
  state.caseStatus = 'active';
  state.caseRevision++;
  await login(page);
  await enterHearings(page);
  form = await beginHearing(page, 'replace');
  await expect(form.getByLabel('Sede o conexi\u00f3n', { exact: true })).toHaveValue(
    'Sala privada declarada',
  );
  await form.getByRole('button', { name: 'Cerrar formulario de audiencia', exact: true }).click();
  await openResultList(page);
  form = await beginResult(page);
  await fillResultDraft(form);
  await expire(page, state, await form.elementHandle());
  await signInOther(page);
  await selectCase(page);
  await enterHearings(page);
  await openResultList(page);
  form = await beginResult(page);
  await expect(form.getByLabel('Relato del operador', { exact: true })).toHaveValue('');
  await fillResultDraft(form);
  await expire(page, state, await form.elementHandle());
  await signInOther(page);
  await selectCase(page);
  await enterHearings(page);
  await openResultList(page);
  state.allowed = false;
  await page.getByRole('button', { name: 'Registrar sesi\u00f3n o acto', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
  ).toBeVisible();
  state.allowed = true;
  await selectCase(page);
  await enterHearings(page);
  await openResultList(page);
  form = await beginResult(page);
  await expect(form.getByLabel('Relato del operador', { exact: true })).toHaveValue('');
  expect(state.hearingPosts).toEqual([]);
  expect(state.resultPosts).toEqual([]);
});

test('hearing and result child Files have separate owners and a failed historical read can be retried without losing either draft', async ({
  page,
}) => {
  const state = await hearingDraftSetup(page, { hearings: [hearingRecord()], stage: 'trial' });
  await login(page);
  await enterHearings(page);
  let form = await beginHearing(page);
  await fillHearingDraft(form);
  await form
    .getByRole('combobox', { name: 'Tipo de audiencia', exact: true })
    .selectOption('sentencing');
  await form
    .getByLabel('Declaraci\u00f3n del antecedente', { exact: true })
    .fill(rawHearing.statement);
  let upload = await openHearingUpload(page, form);
  await fillHearingFile(upload, 'mismo-nombre.pdf', '%PDF-1.4\nhearing owner\n%%EOF');
  await expire(page, state, await upload.elementHandle());
  await login(page);
  await enterHearings(page);
  await openResultList(page);
  form = await beginResult(page);
  await fillResultDraft(form);
  upload = await openHearingUpload(page, form, true);
  const resultBytes = '%PDF-1.4\nresult owner\n%%EOF';
  await fillHearingFile(upload, 'mismo-nombre.pdf', resultBytes);
  await expire(page, state, await upload.elementHandle());
  await login(page);
  await enterHearings(page);
  form = await beginHearing(page);
  upload = await openHearingUpload(page, form);
  await expect(upload).toContainText('mismo-nombre.pdf');
  await upload.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await form.getByRole('button', { name: 'Cerrar formulario de audiencia', exact: true }).click();
  await openResultList(page);
  state.failures.set(`${hearingPath}/revisions/1`, 503);
  form = await beginResult(page);
  await expect(form.getByRole('alert')).toBeVisible();
  await expect(reviewResult(form)).toBeDisabled();
  state.failures.delete(`${hearingPath}/revisions/1`);
  await form.getByRole('button', { name: 'Volver a consultar el contexto', exact: true }).click();
  await expect(reviewResult(form)).toBeEnabled();
  await expect(form.getByLabel('Relato del operador', { exact: true })).toHaveValue(
    rawResult.summary,
  );
  upload = await openHearingUpload(page, form, true);
  await expect(upload).toContainText('mismo-nombre.pdf');
  state.nextUpload = {};
  await upload.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(upload).not.toBeVisible();
  expect(state.uploads).toHaveLength(1);
  expect(state.uploads[0]).toMatchObject({ filename: 'mismo-nombre.pdf', text: resultBytes });
  expect(state.hearingPosts).toEqual([]);
  expect(state.resultPosts).toEqual([]);
  expect(state.preparations).toEqual([]);
});
