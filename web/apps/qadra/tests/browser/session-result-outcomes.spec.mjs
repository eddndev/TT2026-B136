import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { hearingRecord } from '../fixtures/hearings.mjs';
import { resultRecord, resultId } from '../fixtures/hearing-results.mjs';
import {
  hearingDraftSetup,
  checkHearingDraftRequests,
  holdHearingRead,
  hearingPath,
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
} from './session-hearing-draft-ui.mjs';

test.afterEach(async ({ page }) => checkHearingDraftRequests(page));
const resultPath = `${hearingPath}/results/${resultId}`;

test('an interrupted withdrawal remains uncertain after absence and a different exact receipt never confirms it', async ({
  page,
}) => {
  const initial = resultRecord();
  const state = await hearingDraftSetup(page, { hearings: [hearingRecord()], results: [initial] });
  await login(page);
  await enterHearings(page);
  await openResultList(page);
  let form = await beginResult(page, 'withdraw');
  await form.getByLabel('Motivo', { exact: true }).fill('  Retiro declarado pendiente  ');
  await prepareResult(state, form);
  const sent = holdHearingRead(state, 'POST', `${resultPath}/withdrawal`);
  state.nextResultWrite = { commit: false, status: 503 };
  await form.getByRole('button', { name: 'Confirmar resultado', exact: true }).click();
  await expect.poll(() => sent.entered).toBe(true);
  const submitted = structuredClone(state.resultPosts[0].values);
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterHearings(page);
  await openResultList(page);
  form = await beginResult(page, 'withdraw');
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(
    '  Retiro declarado pendiente  ',
  );
  await expect(reviewResult(form)).toBeDisabled();
  const before = state.calls.length;
  await form.getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true }).click();
  await expect(form.getByRole('alert')).toContainText('incierto');
  expect(
    state.calls.slice(before).filter((call) => call.path === `${resultPath}/revisions/2`),
  ).toHaveLength(1);
  await expect(reviewResult(form)).toBeDisabled();
  const foreign = state.resultPrepare(submitted.command);
  foreign.command.operation_id = '90000000-0000-4000-8000-000000000099';
  state.resultCommit(foreign);
  await form.getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true }).click();
  await expect(form.getByRole('alert')).toContainText('otro env\u00edo');
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(
    '  Retiro declarado pendiente  ',
  );
  await expect(form).toBeVisible();
  expect(state.resultPosts.map((row) => row.values)).toEqual([submitted]);
  expect(state.preparations).toHaveLength(1);
  sent.release();
});

test('a confirmed result is not recaptured while its list refresh is unresolved', async ({
  page,
}) => {
  const state = await hearingDraftSetup(page, { hearings: [hearingRecord()] });
  await login(page);
  await enterHearings(page);
  await openResultList(page);
  let form = await beginResult(page);
  await fillResultDraft(form);
  await prepareResult(state, form);
  const element = await form.elementHandle();
  const refresh = holdHearingRead(state, 'GET', `${hearingPath}/results`);
  const bearer = `Bearer ${state.current.token}`;
  const late = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === `${hearingPath}/results` &&
      response.request().method() === 'GET' &&
      response.request().headers().authorization === bearer,
  );
  state.nextResultWrite = {};
  await form.getByRole('button', { name: 'Confirmar resultado', exact: true }).click();
  await expect.poll(() => refresh.entered).toBe(true);
  expect(state.resultPosts).toHaveLength(1);
  await expire(page, state, element);
  await login(page);
  await enterHearings(page);
  await openResultList(page);
  await expect(
    page.getByRole('button', { name: 'Retomar borrador de resultado', exact: true }),
  ).toHaveCount(0);
  form = await beginResult(page);
  await expect(form.getByLabel('Relato del operador', { exact: true })).toHaveValue('');
  await form.getByLabel('Relato del operador', { exact: true }).fill(rawResult.summary);
  const before = state.calls.length;
  refresh.release();
  expect((await late).status()).toBe(200);
  await expect(form.getByLabel('Relato del operador', { exact: true })).toHaveValue(
    rawResult.summary,
  );
  expect(
    state.calls
      .slice(before)
      .filter((call) => call.path.includes('/results') && call.method !== 'GET'),
  ).toEqual([]);
  expect(state.resultPosts).toHaveLength(1);
});
