import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import {
  stageDraftSetup,
  checkStageDraftRequests,
  holdStageRequest,
  intermediate,
  stagePath,
  historyPath,
  transitionPath,
} from './session-stage-draft-fixtures.mjs';
import {
  enterStages,
  beginStage,
  resumeStage,
  reviewStage,
  fillTrial,
  chooseSupport,
  prepareIntermediate,
  expectIntermediate,
  rawStage,
} from './session-stage-draft-ui.mjs';

test.afterEach(async ({ page }) => checkStageDraftRequests(page));

test('an interrupted stage command retains its exact payload and reads every history page without attributing a matching entry', async ({
  page,
}) => {
  const state = await stageDraftSetup(page, intermediate());
  state.historyPageSize = 1;
  await login(page);
  await enterStages(page);
  let form = await beginStage(page, 'trial');
  await fillTrial(form);
  await chooseSupport(form, 'Auto de apertura');
  await reviewStage(form).click();
  const sent = holdStageRequest(state, 'POST', transitionPath);
  const bearer = `Bearer ${state.current.token}`;
  const late = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === transitionPath &&
      response.request().headers().authorization === bearer,
  );
  state.nextStageWrite = { status: 503 };
  await form.getByRole('button', { name: 'Registrar transici\u00f3n', exact: true }).click();
  await expect.poll(() => sent.entered).toBe(true);
  const original = structuredClone(state.stagePosts[0].values);
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterStages(page);
  form = await resumeStage(page);
  await expect(form.getByLabel('Tribunal receptor', { exact: true })).toHaveValue(rawStage.court);
  await expect(reviewStage(form)).toBeDisabled();
  await form.getByText('Consultar el \u00faltimo env\u00edo', { exact: true }).click();
  await expect(form.locator('.stage-last-request')).toContainText('Revisi\u00f3n esperada: 2');
  await expect(form.locator('.stage-last-request')).toContainText(rawStage.receipt.trim());
  sent.release();
  expect((await late).status()).toBe(503);
  const before = state.calls.length;
  const history = holdStageRequest(state, 'GET', historyPath, '3');
  await form.getByRole('button', { name: 'Consultar etapa e historial', exact: true }).click();
  await expect.poll(() => history.entered).toBe(true);
  await expect(reviewStage(form)).toBeDisabled();
  expect(state.stagePosts).toHaveLength(1);
  history.release();
  await expect(form).toHaveAttribute('aria-busy', 'false');
  const cursors = state.calls
    .slice(before)
    .filter((call) => call.path === historyPath)
    .map((call) => new URLSearchParams(call.search).get('before_revision'));
  expect(cursors).toEqual([null, '3', '2']);
  await expect(form.locator('.stage-reconciliation')).toContainText('Una coincidencia no confirma');
  await form.getByText('Historial consultado para comparar', { exact: true }).click();
  await expect(form.locator('.stage-reconciliation')).toContainText('original@example.com');
  await expect(form.locator('.stage-reconciliation .stage-entry')).toHaveCount(4);
  await expect(
    form.getByRole('button', { name: 'Usar etapa consultada y revisar borrador', exact: true }),
  ).toHaveCount(0);
  expect(state.stagePosts.map((call) => call.values)).toEqual([original]);
  expect(
    state.calls.some((call) => /\/stage\/(operations|receipts|revisions)/.test(call.path)),
  ).toBe(false);
});

test('an absent uncertain stage command requires fresh head and complete history before explicit review and another submission', async ({
  page,
}) => {
  const state = await stageDraftSetup(page);
  await login(page);
  await enterStages(page);
  let form = await beginStage(page);
  await prepareIntermediate(page, form);
  const sent = holdStageRequest(state, 'POST', transitionPath);
  const bearer = `Bearer ${state.current.token}`;
  const late = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === transitionPath &&
      response.request().headers().authorization === bearer,
  );
  state.nextStageWrite = { commit: false, status: 503 };
  await form.getByRole('button', { name: 'Registrar transici\u00f3n', exact: true }).click();
  await expect.poll(() => sent.entered).toBe(true);
  const original = structuredClone(state.stagePosts[0].values);
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterStages(page);
  form = await beginStage(page);
  await expectIntermediate(form);
  await expect(reviewStage(form)).toBeDisabled();
  sent.release();
  expect((await late).status()).toBe(503);
  const before = state.calls.length;
  const head = holdStageRequest(state, 'GET', stagePath);
  const history = holdStageRequest(state, 'GET', historyPath);
  await form.getByRole('button', { name: 'Consultar etapa e historial', exact: true }).click();
  await expect.poll(() => head.entered).toBe(true);
  expect(history.entered).toBe(false);
  await expect(reviewStage(form)).toBeDisabled();
  head.release();
  await expect.poll(() => history.entered).toBe(true);
  const accept = form.getByRole('button', {
    name: 'Usar etapa consultada y revisar borrador',
    exact: true,
  });
  await expect(accept).toBeDisabled();
  history.release();
  await expect(accept).toBeEnabled();
  const reads = state.calls
    .slice(before)
    .filter((call) => [stagePath, historyPath].includes(call.path));
  expect(reads.map((call) => call.path)).toEqual([stagePath, historyPath]);
  for (const call of reads)
    expect(call.headers.authorization).toBe(`Bearer ${state.current.token}`);
  expect(state.stagePosts).toHaveLength(1);
  await accept.click();
  await expectIntermediate(form);
  await reviewStage(form).click();
  await expect(form.locator('.stage-confirmation')).toContainText('revisi\u00f3n esperada 1');
  expect(state.stagePosts).toHaveLength(1);
  state.nextStageWrite = {};
  await form.getByRole('button', { name: 'Registrar transici\u00f3n', exact: true }).click();
  await expect(form).toHaveCount(0);
  expect(state.stagePosts.map((call) => call.values)).toEqual([original, original]);
});

test('a confirmed stage command cannot revive while its subsequent history refresh is still pending', async ({
  page,
}) => {
  const state = await stageDraftSetup(page);
  await login(page);
  await enterStages(page);
  await page.getByRole('button', { name: 'Ver historial de etapas', exact: true }).click();
  await expect(page.locator('.stage-history')).toHaveAttribute('aria-busy', 'false');
  let form = await beginStage(page);
  await prepareIntermediate(page, form);
  const element = await form.elementHandle();
  const history = holdStageRequest(state, 'GET', historyPath);
  const bearer = `Bearer ${state.current.token}`;
  const late = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === historyPath &&
      response.request().headers().authorization === bearer,
  );
  state.nextStageWrite = {};
  await form.getByRole('button', { name: 'Registrar transici\u00f3n', exact: true }).click();
  await expect.poll(() => history.entered).toBe(true);
  expect(state.stagePosts).toHaveLength(1);
  await expire(page, state, element);
  await login(page);
  await enterStages(page);
  await expect(
    page.getByRole('button', { name: 'Retomar borrador de etapa', exact: true }),
  ).toHaveCount(0);
  form = await beginStage(page, 'trial');
  await expect(form.getByLabel('Nota (opcional)', { exact: true })).toHaveValue('');
  await expect(
    form.getByRole('button', { name: 'Consultar etapa e historial', exact: true }),
  ).toHaveCount(0);
  history.release();
  expect((await late).status()).toBe(200);
  await expect(form.getByLabel('Nota (opcional)', { exact: true })).toHaveValue('');
  expect(state.stagePosts).toHaveLength(1);
});
