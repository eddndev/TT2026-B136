import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { browserResource } from './procedural-resources-helpers.mjs';
import {
  resourceDraftSetup,
  checkResourceDraftRequests,
  holdResourceRequest,
  resourcesPath,
} from './session-resource-draft-fixtures.mjs';
import {
  enterResources,
  beginResource,
  rawResource,
  prepareButton,
  confirmButton,
  prepareResource,
} from './session-resource-draft-ui.mjs';

test.afterEach(async ({ page }) => checkResourceDraftRequests(page));

test('an interrupted archive remains uncertain after an absent receipt and a foreign receipt cannot confirm or repeat it', async ({
  page,
}) => {
  const original = browserResource(),
    state = await resourceDraftSetup(page, { resources: [original] });
  await login(page);
  await enterResources(page);
  let form = await beginResource(page, 'archive');
  await form.getByLabel('Motivo', { exact: true }).fill(rawResource.reason);
  await prepareResource(state, form);
  const sent = holdResourceRequest(state, 'POST', `${resourcesPath}/${original.id}/archive`);
  state.nextResourceWrite = { commit: false, status: 503 };
  await confirmButton(form).click();
  await expect.poll(() => sent.entered).toBe(true);
  const submitted = structuredClone(state.resourcePosts[0].values);
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterResources(page);
  form = await beginResource(page, 'archive');
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(rawResource.reason);
  await expect(prepareButton(form)).toHaveCount(0);
  const before = state.calls.length;
  await form.getByRole('button', { name: 'Consultar envio exacto', exact: true }).click();
  await expect(form.getByRole('alert')).toContainText('incierto');
  expect(
    state.calls
      .slice(before)
      .filter(
        (call) =>
          call.method === 'GET' && call.path === `${resourcesPath}/${original.id}/revisions/2`,
      ),
  ).toHaveLength(1);
  const foreign = state.resourcePrepare(submitted.command);
  foreign.command.operation_id = '90000000-0000-4000-8000-000000000031';
  state.resourceCommit(foreign);
  await form.getByRole('button', { name: 'Consultar envio exacto', exact: true }).click();
  await expect(form.getByRole('alert')).toContainText('otro');
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(rawResource.reason);
  await expect(prepareButton(form)).toHaveCount(0);
  expect(state.resourcePosts.map((row) => row.values)).toEqual([submitted]);
  expect(state.resourcePrepares).toHaveLength(1);
  sent.release();
});

test('an exact correction receipt clears the draft before its parent refresh and another expiry without a late callback reviving it', async ({
  page,
}) => {
  const original = browserResource(),
    state = await resourceDraftSetup(page, { resources: [original] });
  await login(page);
  await enterResources(page);
  let form = await beginResource(page, 'correct');
  await form.getByLabel('Motivos del recurso', { exact: true }).fill(rawResource.grounds);
  await form.getByLabel('Motivo', { exact: true }).fill(rawResource.reason);
  await prepareResource(state, form);
  const sent = holdResourceRequest(state, 'PUT', `${resourcesPath}/${original.id}`);
  state.nextResourceWrite = { status: 503 };
  await confirmButton(form).click();
  await expect.poll(() => sent.entered).toBe(true);
  const submitted = structuredClone(state.resourcePosts[0].values);
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterResources(page);
  form = await beginResource(page, 'correct');
  await expect(form.getByLabel('Motivos del recurso', { exact: true })).toHaveValue(
    rawResource.grounds,
  );
  await expect(prepareButton(form)).toHaveCount(0);
  const refresh = holdResourceRequest(state, 'GET', resourcesPath),
    before = state.calls.length;
  await form.getByRole('button', { name: 'Consultar envio exacto', exact: true }).click();
  await expect.poll(() => refresh.entered).toBe(true);
  expect(
    state.calls
      .slice(before)
      .filter(
        (call) =>
          call.method === 'GET' && call.path === `${resourcesPath}/${original.id}/revisions/2`,
      ),
  ).toHaveLength(1);
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterResources(page);
  form = await beginResource(page, 'correct');
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue('');
  await expect(form.getByLabel('Motivos del recurso', { exact: true })).toHaveValue(
    rawResource.grounds.trim(),
  );
  await expect(
    form.getByRole('button', { name: 'Consultar envio exacto', exact: true }),
  ).toHaveCount(0);
  await form.getByLabel('Motivo', { exact: true }).fill('Captura posterior independiente');
  refresh.release();
  sent.release();
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(
    'Captura posterior independiente',
  );
  expect(state.resourcePosts.map((row) => row.values)).toEqual([submitted]);
  expect(state.resourcePrepares).toHaveLength(1);
});
