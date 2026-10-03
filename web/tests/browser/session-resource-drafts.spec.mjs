import { test, expect } from '@playwright/test';
import { login, document } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { browserResource } from './procedural-resources-helpers.mjs';
import { resourceCommandFixture, resourceActId } from '../fixtures/procedural-resource-unit.mjs';
import {
  resourceDraftSetup,
  checkResourceDraftRequests,
  holdResourceRequest,
  casePath,
} from './session-resource-draft-fixtures.mjs';
import {
  enterResources,
  beginResource,
  resourceEditor,
  rawResource,
  resolutionSupport,
  prepareButton,
  confirmButton,
  prepareResource,
} from './session-resource-draft-ui.mjs';
import { openFactUpload, chooseFactFile } from './session-fact-draft-ui.mjs';
import { factsPath } from './session-fact-draft-fixtures.mjs';

test.afterEach(async ({ page }) => checkResourceDraftRequests(page));

test('a partial resource and its support File restore only after fresh authorization and explicit closure discards both', async ({
  page,
}) => {
  const state = await resourceDraftSetup(page);
  await login(page);
  await enterResources(page);
  let form = await beginResource(page);
  await form.getByLabel('Titulo organizativo', { exact: true }).fill(rawResource.title);
  await form.getByLabel('Motivos del recurso', { exact: true }).fill(rawResource.grounds);
  await form.getByLabel('Nombre de recurrente 1', { exact: true }).fill('  Recurrente parcial  ');
  let modal = await openFactUpload(page, form, resolutionSupport);
  const bytes = '%PDF-1.4\nresource owner only\n%%EOF';
  await chooseFactFile(modal, 'recurso-pendiente.pdf', bytes);
  await expire(page, state, await modal.elementHandle());
  await login(page);
  await enterResources(page);
  const fresh = holdResourceRequest(state, 'GET', casePath);
  form = await beginResource(page);
  await expect.poll(() => fresh.entered).toBe(true);
  await expect(prepareButton(form)).toBeDisabled();
  await expect(form.getByLabel('Titulo organizativo', { exact: true })).toHaveValue('');
  fresh.release();
  await expect(form.getByLabel('Titulo organizativo', { exact: true })).toHaveValue(
    rawResource.title,
  );
  await expect(form.getByLabel('Motivos del recurso', { exact: true })).toHaveValue(
    rawResource.grounds,
  );
  await expect(form.getByLabel('Nombre de recurrente 1', { exact: true })).toHaveValue(
    '  Recurrente parcial  ',
  );
  modal = await openFactUpload(page, form, resolutionSupport);
  await expect(modal).toContainText('recurso-pendiente.pdf');
  await expect(modal.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue(
    '  etiqueta pendiente  ',
  );
  expect(state.uploads).toEqual([]);
  state.nextUpload = {};
  await modal.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(modal).not.toBeVisible();
  expect(state.uploads).toHaveLength(1);
  expect(state.uploads[0]).toMatchObject({ filename: 'recurso-pendiente.pdf', text: bytes });
  modal = await openFactUpload(page, form, resolutionSupport);
  await chooseFactFile(modal, 'descartar-con-recurso.pdf', '%PDF-1.4\nremove owner\n%%EOF');
  await expire(page, state, await modal.elementHandle());
  await login(page);
  await enterResources(page);
  form = await beginResource(page);
  await expect(form.getByLabel('Titulo organizativo', { exact: true })).toHaveValue(
    rawResource.title,
  );
  await form.getByRole('button', { name: 'Cerrar formulario', exact: true }).click();
  form = await beginResource(page);
  await expect(form.getByLabel('Titulo organizativo', { exact: true })).toHaveValue('');
  modal = await openFactUpload(page, form, resolutionSupport);
  await expect(modal.getByLabel('Nombre del documento', { exact: true })).toHaveValue('');
  await expect(modal.getByLabel('Archivo', { exact: true })).toHaveValue('');
  expect(state.resourcePrepares).toEqual([]);
  expect(state.resourcePosts).toEqual([]);
});

test('a prepared correction loses approval and keeps exact sources until the operator compares and adopts a concurrent head', async ({
  page,
}) => {
  const original = browserResource(),
    state = await resourceDraftSetup(page, { resources: [original] });
  await login(page);
  await enterResources(page);
  let form = await beginResource(page, 'correct');
  await form.getByLabel('Titulo organizativo', { exact: true }).fill(rawResource.title);
  await form.getByLabel('Motivos del recurso', { exact: true }).fill(rawResource.grounds);
  await form.getByLabel('Motivo', { exact: true }).fill(rawResource.reason);
  await prepareResource(state, form);
  const originalCommand = structuredClone(state.resourcePrepares[0].values);
  await expire(page, state, await form.elementHandle());
  const command = structuredClone(originalCommand);
  command.operation_id = '90000000-0000-4000-8000-000000000011';
  command.change.values.title = 'Cambio ajeno de recurso';
  state.resourceCommit(state.resourcePrepare(command));
  await login(page);
  await enterResources(page);
  const exact = holdResourceRequest(
    state,
    'GET',
    `${factsPath}/${original.values.resolution.id}/revisions/1`,
  );
  form = await beginResource(page, 'correct', 'Cambio ajeno de recurso');
  await expect.poll(() => exact.entered).toBe(true);
  await expect(form.getByLabel('Titulo organizativo', { exact: true })).toHaveValue('');
  exact.release();
  await expect(form.getByLabel('Titulo organizativo', { exact: true })).toHaveValue(
    rawResource.title,
  );
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(rawResource.reason);
  await expect(confirmButton(form)).toHaveCount(0);
  await expect(prepareButton(form)).toHaveCount(0);
  await form.getByRole('button', { name: 'Comparar con registro actual', exact: true }).click();
  await expect(form).toContainText('Cambio ajeno de recurso');
  await expect(prepareButton(form)).toHaveCount(0);
  await form
    .getByRole('button', { name: 'Usar base actual y conservar borrador', exact: true })
    .click();
  await prepareResource(state, form);
  const recovered = state.resourcePrepares[1].values;
  expect(recovered.resource_id).toBe(original.id);
  expect(recovered.operation_id).not.toBe(originalCommand.operation_id);
  expect(recovered.change).toMatchObject({
    expected_revision: 2,
    reason: rawResource.reason.trim(),
    values: {
      title: rawResource.title.trim(),
      grounds: rawResource.grounds.trim(),
      resolution: original.values.resolution,
      resolution_evidence: original.values.resolution_evidence,
      appellants: original.values.appellants,
    },
  });
  expect(state.resourcePosts).toEqual([]);
});

test('correcting a historical act retains its own identity and revision separately from the current resource head', async ({
  page,
}) => {
  const original = browserResource(),
    state = await resourceDraftSetup(page, { resources: [original] });
  await login(page);
  const command = resourceCommandFixture('record_act');
  command.change.expected_revision = 1;
  command.change.values.evidence = [structuredClone(original.values.resolution_evidence)];
  const oldAct = state.resourceCommit(state.resourcePrepare(command));
  command.operation_id = '90000000-0000-4000-8000-000000000022';
  command.change.expected_revision = 2;
  command.change.act_id = '90000000-0000-4000-8000-000000000023';
  command.change.values.statement = 'Otro acto posterior distinto';
  state.resourceCommit(state.resourcePrepare(command));
  await enterResources(page);
  let form = await beginResource(page, 'correct_act', undefined, 2);
  await form.getByLabel('Declaraci\u00f3n del acto', { exact: true }).fill(rawResource.statement);
  await form.getByLabel('Motivo', { exact: true }).fill(rawResource.reason);
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterResources(page);
  form = await beginResource(page, 'correct_act', undefined, 2);
  await expect(form.getByLabel('Declaraci\u00f3n del acto', { exact: true })).toHaveValue(
    rawResource.statement,
  );
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(rawResource.reason);
  await prepareResource(state, form);
  expect(state.resourcePrepares[0].values.change).toMatchObject({
    action: 'correct_act',
    expected_revision: 3,
    act_id: resourceActId,
    expected_act_revision: 1,
    values: { statement: rawResource.statement.trim(), evidence: oldAct.act.values.evidence },
  });
  expect(oldAct.act.id).toBe(resourceActId);
  expect(state.resourcePosts).toEqual([]);
  expect(document.digest).toBe(oldAct.act.values.evidence[0].digest);
  await expect(resourceEditor(page)).toBeVisible();
});
