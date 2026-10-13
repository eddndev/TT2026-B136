import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { participant } from './participant-helpers.mjs';
import { factRecord, factPrepared, factCommand } from '../fixtures/procedural-facts.mjs';
import { hearingRecord } from '../fixtures/hearings.mjs';
import { resultRecord } from '../fixtures/hearing-results.mjs';
import {
  factDraftSetup,
  checkFactDraftRequests,
  factKey,
  resolutionPath,
} from './session-fact-draft-fixtures.mjs';
import {
  enterFacts,
  openNotifications,
  beginFact,
  fillNotification,
  rawFact,
  mainSource,
  representationSource,
  externalSource,
  declaredRepresentation,
  chooseHistoricalPerson,
  chooseHistoricalResult,
  prepareButton,
  prepareDraft,
  openFactUpload,
  chooseFactFile,
} from './session-fact-draft-ui.mjs';

test.afterEach(async ({ page }) => checkFactDraftRequests(page));

test('a notification retains raw time and declarations plus exact parent, person and result references through a transient source read failure', async ({
  page,
}) => {
  const parent = factRecord(),
    hearing = hearingRecord(),
    result = resultRecord();
  result.values.agreements = [
    { id: '00000000-0000-0000-0000-000000000000', text: 'Acuerdo original' },
  ];
  const state = await factDraftSetup(page, {
    facts: [parent],
    hearings: [hearing],
    results: [result],
  });
  await login(page);
  await enterFacts(page);
  await openNotifications(page);
  let form = await beginFact(page, 'notification');
  await fillNotification(page, rawFact.summary);
  await chooseHistoricalPerson(form, participant);
  await chooseHistoricalResult(form, mainSource, hearing, result);
  await declaredRepresentation(form);
  await form.getByLabel('Registrar tiempo de recepci\u00f3n', { exact: true }).check();
  await form
    .getByRole('combobox', { name: 'Precisi\u00f3n de recepci\u00f3n', exact: true })
    .selectOption('unknown');
  await form.getByLabel('Registrar efecto expresamente declarado', { exact: true }).check();
  await form
    .getByRole('combobox', { name: 'Precisi\u00f3n de efecto declarado', exact: true })
    .selectOption('unknown');
  await form
    .getByLabel('Declaraci\u00f3n del efecto', { exact: true })
    .fill('  Efecto comunicado, sin calcularlo  ');
  await form
    .getByLabel('Localizador del efecto en la procedencia principal', { exact: true })
    .fill('  Apartado pendiente  ');
  await form
    .getByRole('combobox', { name: 'Precisi\u00f3n de pr\u00e1ctica', exact: true })
    .selectOption('minute');
  await form.getByLabel('Fecha de pr\u00e1ctica', { exact: true }).fill('2026-10-01');
  await form.getByLabel('Hora de pr\u00e1ctica', { exact: true }).fill('09:08');
  await form
    .getByRole('combobox', { name: 'Desfase de pr\u00e1ctica', exact: true })
    .selectOption('declared');
  await form.getByLabel('Desfase UTC de pr\u00e1ctica', { exact: true }).fill('-0');
  await expect(form.getByLabel('Desfase UTC de pr\u00e1ctica', { exact: true })).toHaveValue('-0');
  await expire(page, state, await form.elementHandle());
  const retired = factRecord(factPrepared(factCommand('resolution', 'withdraw', 1), parent));
  state.factRecords.get(factKey(parent)).push(retired);
  state.records
    .get(participant.id)
    .push({ ...structuredClone(participant), revision: 2, directory_status: 'archived' });
  const later = structuredClone(result);
  later.revision = 2;
  later.status = 'withdrawn';
  later.reason = 'Retiro posterior';
  later.receipt = { ...later.receipt, action: 'withdraw', expected_revision: 1 };
  state.results.get(result.id).push(later);
  await login(page);
  await enterFacts(page);
  await openNotifications(page);
  state.failures.set(`${resolutionPath}/revisions/1`, 503);
  form = await beginFact(page, 'notification');
  await expect(form.getByRole('alert')).toBeVisible();
  await expect(prepareButton(form)).toBeDisabled();
  state.failures.delete(`${resolutionPath}/revisions/1`);
  const start = state.calls.length;
  await form.getByRole('button', { name: 'Volver a consultar el contexto', exact: true }).click();
  await expect(prepareButton(form)).toBeEnabled();
  await expect(form.getByLabel('Resumen de la notificaci\u00f3n', { exact: true })).toHaveValue(
    rawFact.summary,
  );
  await expect(form.getByLabel('Alcance de la representaci\u00f3n', { exact: true })).toHaveValue(
    rawFact.scope,
  );
  await expect(form.getByLabel('Desfase UTC de pr\u00e1ctica', { exact: true })).toHaveValue('-0');
  await expect(form.getByLabel('Declaraci\u00f3n del efecto', { exact: true })).toHaveValue(
    '  Efecto comunicado, sin calcularlo  ',
  );
  for (const suffix of [
    `${resolutionPath}/revisions/1`,
    `/participants/${participant.id}/revisions/1`,
    `/hearings/${hearing.id}/results/${result.id}/revisions/1`,
  ])
    expect(
      state.calls.slice(start).some((call) => call.method === 'GET' && call.path.endsWith(suffix)),
    ).toBe(true);
  expect(state.factPrepares).toEqual([]);
  expect(state.factPosts).toEqual([]);
  await form.getByLabel('Desfase UTC de pr\u00e1ctica', { exact: true }).fill('-06:00');
  await form.getByLabel('Desfase UTC de pr\u00e1ctica', { exact: true }).press('Tab');
  await prepareDraft(state, form);
  const values = state.factPrepares[0].values.change.values;
  expect(values.resolution).toEqual({ id: parent.id, revision: 1 });
  expect(values.intended_recipient.value).toEqual({
    kind: 'participant',
    id: participant.id,
    revision: 1,
  });
  expect(values.provenance.reference).toEqual({
    hearing_id: hearing.id,
    result_id: result.id,
    revision: 1,
    agreement_id: result.values.agreements[0].id,
  });
  expect(values.received_at).toEqual({ precision: 'unknown' });
  expect(values.stated_effect.at).toEqual({ precision: 'unknown' });
  expect(values.practiced_at).toMatchObject({
    precision: 'minute',
    hour: 9,
    minute: 8,
    offset_seconds: -21600,
  });
});

test('same-name notification Files stay with their source field and redeclaring representation discards only its old child', async ({
  page,
}) => {
  const state = await factDraftSetup(page, { facts: [factRecord()] });
  await login(page);
  await enterFacts(page);
  await openNotifications(page);
  let form = await beginFact(page, 'notification');
  await fillNotification(page, rawFact.summary);
  await externalSource(form, mainSource);
  await declaredRepresentation(form);
  let modal = await openFactUpload(page, form, mainSource);
  const firstBytes = '%PDF-1.4\nnotification principal source\n%%EOF';
  await chooseFactFile(modal, 'mismo-nombre.pdf', firstBytes);
  await expire(page, state, await modal.elementHandle());
  await login(page);
  await enterFacts(page);
  await openNotifications(page);
  form = await beginFact(page, 'notification');
  modal = await openFactUpload(page, form, representationSource);
  await chooseFactFile(modal, 'mismo-nombre.pdf', '%PDF-1.4\nrepresentation old source\n%%EOF');
  await expire(page, state, await modal.elementHandle());
  await login(page);
  await enterFacts(page);
  await openNotifications(page);
  form = await beginFact(page, 'notification');
  await form
    .getByRole('combobox', { name: 'Representaci\u00f3n declarada', exact: true })
    .selectOption('not_recorded');
  await declaredRepresentation(form);
  modal = await openFactUpload(page, form, representationSource);
  await expect(modal.getByLabel('Nombre del documento', { exact: true })).toHaveValue('');
  await expect(modal.getByLabel('Archivo', { exact: true })).toHaveValue('');
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
  modal = await openFactUpload(page, form, mainSource);
  await expect(modal).toContainText('mismo-nombre.pdf');
  expect(state.uploads).toEqual([]);
  state.nextUpload = {};
  await modal.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(modal).not.toBeVisible();
  expect(state.uploads).toHaveLength(1);
  expect(state.uploads[0]).toMatchObject({ filename: 'mismo-nombre.pdf', text: firstBytes });
  expect(state.factPosts).toEqual([]);
  expect(state.factPrepares).toEqual([]);
});
