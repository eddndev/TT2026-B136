import { test, expect } from '@playwright/test';
import { login, selectCase } from './helpers.mjs';
import { expire, signInOther } from './session-inactivity-helpers.mjs';
import { browserResource } from './procedural-resources-helpers.mjs';
import {
  resourceDraftSetup,
  checkResourceDraftRequests,
} from './session-resource-draft-fixtures.mjs';
import {
  enterResources,
  beginResource,
  resourceEditor,
  rawResource,
  resolutionSupport,
  prepareButton,
} from './session-resource-draft-ui.mjs';
import { openFactUpload, chooseFactFile } from './session-fact-draft-ui.mjs';

test.afterEach(async ({ page }) => checkResourceDraftRequests(page));

test('a closed case exposes a resource draft readonly and a later fresh denial discards that context', async ({
  page,
}) => {
  const state = await resourceDraftSetup(page);
  await login(page);
  await enterResources(page);
  let form = await beginResource(page);
  await form.getByLabel('Titulo organizativo', { exact: true }).fill(rawResource.title);
  await form.getByLabel('Motivos del recurso', { exact: true }).fill(rawResource.grounds);
  await expire(page, state, await form.elementHandle());
  state.caseStatus = 'closed';
  state.caseRevision++;
  await login(page);
  await enterResources(page);
  await page.getByRole('button', { name: 'Retomar borrador de recurso', exact: true }).click();
  form = resourceEditor(page);
  await expect(form.getByLabel('Titulo organizativo', { exact: true })).toHaveValue(
    rawResource.title,
  );
  await expect(form.getByLabel('Titulo organizativo', { exact: true })).toBeDisabled();
  await expect(prepareButton(form)).toBeDisabled();
  state.caseStatus = 'active';
  state.caseRevision++;
  await form.getByRole('button', { name: 'Volver a consultar el contexto', exact: true }).click();
  await expect(form.getByLabel('Titulo organizativo', { exact: true })).toBeEditable();
  await expect(form.getByLabel('Motivos del recurso', { exact: true })).toHaveValue(
    rawResource.grounds,
  );
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterResources(page);
  state.allowed = false;
  await page.getByRole('button', { name: 'Registrar recurso', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
  ).toBeVisible();
  state.allowed = true;
  await selectCase(page);
  await enterResources(page);
  form = await beginResource(page);
  await expect(form.getByLabel('Titulo organizativo', { exact: true })).toHaveValue('');
  expect(state.resourcePosts).toEqual([]);
  expect(state.resourcePrepares).toEqual([]);
});

test('a different Owner never receives resource declarations or child files and returning to the first account cannot revive them', async ({
  page,
}) => {
  const state = await resourceDraftSetup(page);
  await login(page);
  await enterResources(page);
  let form = await beginResource(page);
  await form.getByLabel('Titulo organizativo', { exact: true }).fill(rawResource.title);
  let modal = await openFactUpload(page, form, resolutionSupport);
  await chooseFactFile(modal, 'privado-del-primer-owner.pdf', '%PDF-1.4\nfirst principal\n%%EOF');
  await expire(page, state, await modal.elementHandle());
  await signInOther(page);
  await selectCase(page);
  await enterResources(page);
  form = await beginResource(page);
  await expect(form.getByLabel('Titulo organizativo', { exact: true })).toHaveValue('');
  modal = await openFactUpload(page, form, resolutionSupport);
  await expect(modal.getByLabel('Nombre del documento', { exact: true })).toHaveValue('');
  await expect(modal.getByLabel('Archivo', { exact: true })).toHaveValue('');
  await expire(page, state, await modal.elementHandle());
  await login(page);
  await enterResources(page);
  form = await beginResource(page);
  await expect(form.getByLabel('Titulo organizativo', { exact: true })).toHaveValue('');
  modal = await openFactUpload(page, form, resolutionSupport);
  await expect(modal.getByLabel('Nombre del documento', { exact: true })).toHaveValue('');
  expect(state.uploads).toEqual([]);
  expect(state.resourcePosts).toEqual([]);
  expect(state.resourcePrepares).toEqual([]);
});

test('an act support keeps its File with the original row and removing that row cannot transfer the File to its replacement', async ({
  page,
}) => {
  const state = await resourceDraftSetup(page, { resources: [browserResource()] });
  await login(page);
  await enterResources(page);
  let form = await beginResource(page, 'record_act');
  await form.getByLabel('Declaraci\u00f3n del acto', { exact: true }).fill(rawResource.statement);
  await form.getByRole('button', { name: 'Agregar soporte del acto', exact: true }).click();
  let modal = await openFactUpload(page, form, 'acto 2');
  await chooseFactFile(modal, 'segundo-soporte-propio.pdf', '%PDF-1.4\nsecond row owner\n%%EOF');
  await expire(page, state, await modal.elementHandle());
  await login(page);
  await enterResources(page);
  form = await beginResource(page, 'record_act');
  await expect(form.getByLabel('Declaraci\u00f3n del acto', { exact: true })).toHaveValue(
    rawResource.statement,
  );
  modal = await openFactUpload(page, form, 'acto 2');
  await expect(modal).toContainText('segundo-soporte-propio.pdf');
  await expire(page, state, await modal.elementHandle());
  await login(page);
  await enterResources(page);
  form = await beginResource(page, 'record_act');
  await expect(form.getByLabel('Declaraci\u00f3n del acto', { exact: true })).toHaveValue(
    rawResource.statement,
  );
  await form.getByRole('button', { name: 'Quitar soporte adicional 2', exact: true }).click();
  await form.getByRole('button', { name: 'Agregar soporte del acto', exact: true }).click();
  modal = await openFactUpload(page, form, 'acto 2');
  await expect(modal.getByLabel('Nombre del documento', { exact: true })).toHaveValue('');
  await expect(modal.getByLabel('Archivo', { exact: true })).toHaveValue('');
  expect(state.uploads).toEqual([]);
  expect(state.resourcePosts).toEqual([]);
  expect(state.resourcePrepares).toEqual([]);
});
