import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { factRecord } from '../fixtures/procedural-facts.mjs';
import {
  factDraftSetup,
  checkFactDraftRequests,
  holdFactRequest,
  notificationRecord,
  factsPath,
  notificationPath,
} from './session-fact-draft-fixtures.mjs';
import {
  enterFacts,
  openNotifications,
  beginFact,
  fillResolution,
  rawFact,
  prepareButton,
  prepareDraft,
  confirmButton,
} from './session-fact-draft-ui.mjs';

test.afterEach(async ({ page }) => checkFactDraftRequests(page));

test('an interrupted notification withdrawal remains uncertain after absence and a foreign exact receipt never confirms or repeats it', async ({
  page,
}) => {
  const parent = factRecord();
  const state = await factDraftSetup(page, { facts: [parent, notificationRecord(parent)] });
  await login(page);
  await enterFacts(page);
  await openNotifications(page);
  let form = await beginFact(page, 'notification', 'withdraw');
  await form.getByLabel('Motivo', { exact: true }).fill(rawFact.reason);
  await prepareDraft(state, form);
  const sent = holdFactRequest(state, 'POST', `${notificationPath}/withdrawal`);
  state.nextFactWrite = { commit: false, status: 503 };
  await confirmButton(form).click();
  await expect.poll(() => sent.entered).toBe(true);
  const submitted = structuredClone(state.factPosts[0].values);
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterFacts(page);
  await openNotifications(page);
  form = await beginFact(page, 'notification', 'withdraw');
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(rawFact.reason);
  await expect(prepareButton(form)).toBeDisabled();
  const before = state.calls.length;
  await form.getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true }).click();
  await expect(form.getByRole('alert')).toContainText('incierto');
  expect(
    state.calls
      .slice(before)
      .filter((call) => call.method === 'GET' && call.path === `${notificationPath}/revisions/2`),
  ).toHaveLength(1);
  await expect(prepareButton(form)).toBeDisabled();
  const foreign = state.factPrepare(submitted.command);
  foreign.command.operation_id = '90000000-0000-4000-8000-000000000099';
  state.factCommit(foreign);
  await form.getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true }).click();
  await expect(form.getByRole('alert')).toContainText('otro env\u00edo');
  await expect(form).toBeVisible();
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(rawFact.reason);
  expect(state.factPosts.map((row) => row.values)).toEqual([submitted]);
  expect(state.factPrepares).toHaveLength(1);
  sent.release();
});

test('an exact receipt confirms the original resolution submission and removes its draft before a held refresh and another expiry', async ({
  page,
}) => {
  const state = await factDraftSetup(page);
  await login(page);
  await enterFacts(page);
  let form = await beginFact(page);
  await fillResolution(page, rawFact.summary);
  await prepareDraft(state, form);
  const sent = holdFactRequest(state, 'POST', factsPath);
  state.nextFactWrite = { status: 503 };
  await confirmButton(form).click();
  await expect.poll(() => sent.entered).toBe(true);
  const submitted = structuredClone(state.factPosts[0].values);
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterFacts(page);
  form = await beginFact(page);
  await expect(prepareButton(form)).toBeDisabled();
  await expect(form.getByLabel('Resumen de la resoluci\u00f3n', { exact: true })).toHaveValue(
    rawFact.summary,
  );
  const refresh = holdFactRequest(state, 'GET', factsPath);
  const before = state.calls.length;
  await form.getByRole('button', { name: 'Consultar env\u00edo exacto', exact: true }).click();
  await expect.poll(() => refresh.entered).toBe(true);
  expect(
    state.calls
      .slice(before)
      .some(
        (call) =>
          call.method === 'GET' && call.path === `${factsPath}/${submitted.command.id}/revisions/1`,
      ),
  ).toBe(true);
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterFacts(page);
  form = await beginFact(page);
  await expect(form.getByLabel('Resumen de la resoluci\u00f3n', { exact: true })).toHaveValue('');
  await form
    .getByLabel('Resumen de la resoluci\u00f3n', { exact: true })
    .fill('Captura posterior independiente');
  refresh.release();
  sent.release();
  await expect(form.getByLabel('Resumen de la resoluci\u00f3n', { exact: true })).toHaveValue(
    'Captura posterior independiente',
  );
  expect(state.factPosts.map((row) => row.values)).toEqual([submitted]);
  expect(state.factPrepares).toHaveLength(1);
});
