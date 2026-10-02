import { test, expect } from '@playwright/test';
import { login, selectCase } from './helpers.mjs';
import { expire, signInOther } from './session-inactivity-helpers.mjs';
import { factRecord } from '../fixtures/procedural-facts.mjs';
import { factDraftSetup, checkFactDraftRequests } from './session-fact-draft-fixtures.mjs';
import {
  enterFacts,
  openNotifications,
  beginFact,
  factEditor,
  fillResolution,
  fillNotification,
  rawFact,
  mainSource,
  externalSource,
  prepareButton,
  openFactUpload,
  chooseFactFile,
} from './session-fact-draft-ui.mjs';

test.afterEach(async ({ page }) => checkFactDraftRequests(page));

test('a closed case keeps a fact draft readonly and fresh denial discards that context without a write', async ({
  page,
}) => {
  const state = await factDraftSetup(page);
  await login(page);
  await enterFacts(page);
  let form = await beginFact(page);
  await fillResolution(page, rawFact.summary);
  await expire(page, state, await form.elementHandle());
  state.caseStatus = 'closed';
  state.caseRevision++;
  await login(page);
  await enterFacts(page);
  await page
    .getByRole('button', { name: 'Retomar borrador de resoluci\u00f3n', exact: true })
    .click();
  form = factEditor(page);
  await expect(form.getByLabel('Resumen de la resoluci\u00f3n', { exact: true })).toHaveValue(
    rawFact.summary,
  );
  await expect(form.getByLabel('Resumen de la resoluci\u00f3n', { exact: true })).toBeDisabled();
  await expect(prepareButton(form)).toBeDisabled();
  state.caseStatus = 'active';
  state.caseRevision++;
  await form.getByRole('button', { name: 'Volver a consultar el contexto', exact: true }).click();
  await expect(prepareButton(form)).toBeEnabled();
  await expect(form.getByLabel('Resumen de la resoluci\u00f3n', { exact: true })).toHaveValue(
    rawFact.summary,
  );
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterFacts(page);
  state.allowed = false;
  await page.getByRole('button', { name: 'Registrar resoluci\u00f3n', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
  ).toBeVisible();
  state.allowed = true;
  await selectCase(page);
  await enterFacts(page);
  form = await beginFact(page);
  await expect(form.getByLabel('Resumen de la resoluci\u00f3n', { exact: true })).toHaveValue('');
  expect(state.factPosts).toEqual([]);
  expect(state.factPrepares).toEqual([]);
});

test('a notification draft belongs to one parent and account while explicit closure removes its complete child tree', async ({
  page,
}) => {
  const parent = factRecord(),
    other = structuredClone(parent);
  other.id = '90000000-0000-4000-8000-000000000009';
  other.values.summary = 'Otra resolucion propietaria';
  const state = await factDraftSetup(page, { facts: [parent, other] });
  await login(page);
  await enterFacts(page);
  await openNotifications(page, parent.id);
  let form = await beginFact(page, 'notification');
  await fillNotification(page, rawFact.summary);
  await externalSource(form, mainSource);
  let modal = await openFactUpload(page, form, mainSource);
  await chooseFactFile(modal, 'archivo-del-primer-padre.pdf', '%PDF-1.4\nfirst parent only\n%%EOF');
  await expire(page, state, await modal.elementHandle());
  await login(page);
  await enterFacts(page);
  await openNotifications(page, other.id);
  form = await beginFact(page, 'notification');
  await expect(form.getByLabel('Resumen de la notificaci\u00f3n', { exact: true })).toHaveValue('');
  await form.getByRole('button', { name: 'Cerrar formulario', exact: true }).click();
  await openNotifications(page, parent.id);
  form = await beginFact(page, 'notification');
  await expect(form.getByLabel('Resumen de la notificaci\u00f3n', { exact: true })).toHaveValue(
    rawFact.summary,
  );
  modal = await openFactUpload(page, form, mainSource);
  await expect(modal).toContainText('archivo-del-primer-padre.pdf');
  await expire(page, state, await modal.elementHandle());
  await signInOther(page);
  await selectCase(page);
  await enterFacts(page);
  await openNotifications(page, parent.id);
  form = await beginFact(page, 'notification');
  await expect(form.getByLabel('Resumen de la notificaci\u00f3n', { exact: true })).toHaveValue('');
  await fillNotification(page, '  Borrador de la otra cuenta  ');
  await externalSource(form, mainSource);
  modal = await openFactUpload(page, form, mainSource);
  await expect(modal.getByLabel('Nombre del documento', { exact: true })).toHaveValue('');
  await chooseFactFile(modal, 'descartar-con-su-editor.pdf', '%PDF-1.4\nexplicit closure\n%%EOF');
  await expire(page, state, await modal.elementHandle());
  await signInOther(page);
  await selectCase(page);
  await enterFacts(page);
  await openNotifications(page, parent.id);
  form = await beginFact(page, 'notification');
  await expect(form.getByLabel('Resumen de la notificaci\u00f3n', { exact: true })).toHaveValue(
    '  Borrador de la otra cuenta  ',
  );
  await form.getByRole('button', { name: 'Cerrar formulario', exact: true }).click();
  form = await beginFact(page, 'notification');
  await expect(form.getByLabel('Resumen de la notificaci\u00f3n', { exact: true })).toHaveValue('');
  await externalSource(form, mainSource);
  modal = await openFactUpload(page, form, mainSource);
  await expect(modal.getByLabel('Nombre del documento', { exact: true })).toHaveValue('');
  await expect(modal.getByLabel('Archivo', { exact: true })).toHaveValue('');
  expect(state.uploads).toEqual([]);
  expect(state.factPosts).toEqual([]);
  expect(state.factPrepares).toEqual([]);
});
