import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { factRecord, factPrepared, factCommand } from '../fixtures/procedural-facts.mjs';
import {
  factDraftSetup,
  checkFactDraftRequests,
  holdFactRequest,
  casePath,
  factKey,
} from './session-fact-draft-fixtures.mjs';
import {
  enterFacts,
  beginFact,
  factEditor,
  fillResolution,
  rawFact,
  resolutionSource,
  externalSource,
  openFactUpload,
  chooseFactFile,
  prepareButton,
  prepareDraft,
} from './session-fact-draft-ui.mjs';

test.afterEach(async ({ page }) => checkFactDraftRequests(page));

test('a resolution restores partial declarations and its child File only after fresh case authorization without preparing or sending automatically', async ({
  page,
}) => {
  const state = await factDraftSetup(page);
  await login(page);
  await enterFacts(page);
  let form = await beginFact(page);
  await fillResolution(page, rawFact.summary);
  await form.getByLabel('Subtipo declarado (opcional)', { exact: true }).fill(rawFact.subtype);
  await form.getByLabel('Motivo: Emisor', { exact: true }).fill('   ');
  await externalSource(form, resolutionSource);
  let modal = await openFactUpload(page, form, resolutionSource);
  const bytes = '%PDF-1.4\nresolution owner original\n%%EOF';
  await chooseFactFile(modal, 'resolucion-pendiente.pdf', bytes);
  await expire(page, state, await modal.elementHandle());
  await login(page);
  await enterFacts(page);
  const fresh = holdFactRequest(state, 'GET', casePath);
  form = await beginFact(page);
  await expect.poll(() => fresh.entered).toBe(true);
  await expect(prepareButton(form)).toBeDisabled();
  expect(state.factPrepares).toEqual([]);
  expect(state.factPosts).toEqual([]);
  fresh.release();
  await expect(form.getByLabel('Resumen de la resoluci\u00f3n', { exact: true })).toHaveValue(
    rawFact.summary,
  );
  await expect(form.getByLabel('Subtipo declarado (opcional)', { exact: true })).toHaveValue(
    rawFact.subtype,
  );
  await expect(form.getByLabel('Motivo: Emisor', { exact: true })).toHaveValue('   ');
  modal = await openFactUpload(page, form, resolutionSource);
  await expect(modal).toContainText('resolucion-pendiente.pdf');
  await expect(modal.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue(
    '  etiqueta pendiente  ',
  );
  expect(state.uploads).toEqual([]);
  state.nextUpload = {};
  await modal.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(modal).not.toBeVisible();
  expect(state.uploads).toHaveLength(1);
  expect(state.uploads[0]).toMatchObject({ filename: 'resolucion-pendiente.pdf', text: bytes });
  await form
    .getByLabel('Localizador documental: ' + resolutionSource, { exact: true })
    .fill('Pagina 1');
  await form.getByLabel('Motivo: Emisor', { exact: true }).fill('El emisor no consta');
  await prepareDraft(state, form);
  expect(state.factPrepares).toHaveLength(1);
  expect(state.factPrepares[0].values.change.values.summary).toBe(rawFact.summary.trim());
  expect(state.factPosts).toEqual([]);
});

test('a corrected resolution keeps its original base and raw fields until the operator reads and explicitly adopts a concurrent revision', async ({
  page,
}) => {
  const original = factRecord();
  const state = await factDraftSetup(page, { facts: [original] });
  await login(page);
  await enterFacts(page);
  let form = await beginFact(page, 'resolution', 'correct');
  await form.getByLabel('Resumen de la resoluci\u00f3n', { exact: true }).fill(rawFact.summary);
  await form.getByLabel('Motivo', { exact: true }).fill(rawFact.reason);
  await expire(page, state, await form.elementHandle());
  const command = factCommand('resolution', 'correct', 1);
  command.change.values.summary = 'Revision ajena declarada';
  const newer = factRecord(factPrepared(command, original));
  state.factRecords.get(factKey(original)).push(newer);
  await login(page);
  await enterFacts(page);
  form = await beginFact(page, 'resolution', 'correct');
  await expect(form.getByLabel('Resumen de la resoluci\u00f3n', { exact: true })).toHaveValue(
    rawFact.summary,
  );
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(rawFact.reason);
  await expect(prepareButton(form)).toBeDisabled();
  expect(state.factPrepares).toEqual([]);
  await form.getByRole('button', { name: 'Consultar base actual', exact: true }).click();
  await expect(form).toContainText('Revision ajena declarada');
  await expect(prepareButton(form)).toBeDisabled();
  await form
    .getByRole('button', { name: 'Usar esta base y conservar borrador', exact: true })
    .click();
  await prepareDraft(state, form);
  expect(state.factPrepares[0].values.change).toMatchObject({
    expected_revision: 2,
    reason: rawFact.reason.trim(),
    values: { summary: rawFact.summary.trim() },
  });
  expect(state.factPosts).toEqual([]);
  await expect(factEditor(page)).toBeVisible();
});
