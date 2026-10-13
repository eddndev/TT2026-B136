import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { resourceCommandFixture } from '../fixtures/procedural-resource-unit.mjs';
import { resourceRef } from './session-activity-command-fixtures.mjs';
import {
  resourceDeadlineDraftSetup,
  checkResourceDeadlineRequests,
  holdDeadlineRequest,
  casePath,
} from './session-resource-deadline-fixtures.mjs';
import {
  startResourceDeadline,
  beginResourceDeadline,
  openActivityScope,
  chooseOldAct,
  prepareDeadlineButton,
  confirmDeadlineButton,
  prepareResourceDeadline,
  titleField,
  rawDeadline,
} from './session-resource-deadline-ui.mjs';

test.afterEach(async ({ page }) => checkResourceDeadlineRequests(page));

test('a resource deadline restores raw declarations and a partial offset only after fresh case authority without preparing or writing', async ({
  page,
}) => {
  const state = await resourceDeadlineDraftSetup(page);
  let form = await startResourceDeadline(page, state);
  await chooseOldAct(form);
  await form
    .getByLabel('Motivo de fuente no identificada', { exact: true })
    .fill(rawDeadline.source);
  await form
    .getByLabel('Declaraci\u00f3n de aplicabilidad', { exact: true })
    .fill(rawDeadline.statement);
  await form.getByLabel('Localizador de aplicabilidad', { exact: true }).fill(rawDeadline.locator);
  await form.getByLabel('Declarar un inicio calificado', { exact: true }).check();
  await form
    .getByRole('combobox', { name: 'Finalidad del inicio', exact: true })
    .selectOption('ordered_period_start');
  await form
    .getByRole('combobox', { name: 'Precisi\u00f3n de inicio calificado', exact: true })
    .selectOption('minute');
  await form.getByLabel('Fecha de inicio calificado', { exact: true }).fill('2026-10-02');
  await form.getByLabel('Hora de inicio calificado', { exact: true }).fill('11:23');
  await form
    .getByRole('combobox', { name: 'Desfase de inicio calificado', exact: true })
    .selectOption('declared');
  await form.getByLabel('Desfase UTC de inicio calificado', { exact: true }).fill('-0');
  await expire(page, state, await form.elementHandle());
  await login(page);
  await openActivityScope(page, state, 1);
  const fresh = holdDeadlineRequest(state, 'GET', casePath);
  form = await beginResourceDeadline(page);
  await expect.poll(() => fresh.entered).toBe(true);
  await expect(titleField(form)).toHaveValue('');
  await expect(prepareDeadlineButton(form)).toBeDisabled();
  fresh.release();
  await expect(titleField(form)).toHaveValue(rawDeadline.title);
  await expect(form.getByLabel('Motivo de fuente no identificada', { exact: true })).toHaveValue(
    rawDeadline.source,
  );
  await expect(form.getByLabel('Declaraci\u00f3n de aplicabilidad', { exact: true })).toHaveValue(
    rawDeadline.statement,
  );
  await expect(form.getByLabel('Localizador de aplicabilidad', { exact: true })).toHaveValue(
    rawDeadline.locator,
  );
  await expect(form.getByLabel('Fecha de inicio calificado', { exact: true })).toHaveValue(
    '2026-10-02',
  );
  await expect(form.getByLabel('Hora de inicio calificado', { exact: true })).toHaveValue('11:23');
  await expect(form.getByLabel('Desfase UTC de inicio calificado', { exact: true })).toHaveValue(
    '-0',
  );
  await expect(
    form.getByRole('combobox', { name: 'Cuando cambie el perfil', exact: true }),
  ).toHaveValue('fixed');
  await expect(
    form.getByRole('region', { name: 'Acto opcional del recurso', exact: true }),
  ).toContainText('Recurso revisi\u00f3n 2');
  await expect(
    form.getByRole('region', { name: 'Responsable seleccionado', exact: true }),
  ).toContainText('staff@example.test');
  expect(state.compositePrepares).toEqual([]);
  expect(state.compositePosts).toEqual([]);
});

test('a prepared resource deadline loses approval and explicitly adopts the new resource head while preserving both draft identities and historical captures', async ({
  page,
}) => {
  const state = await resourceDeadlineDraftSetup(page);
  let form = await startResourceDeadline(page, state);
  await chooseOldAct(form);
  await prepareResourceDeadline(state, form);
  const original = structuredClone(state.compositePrepares[0].values);
  await expire(page, state, await form.elementHandle());
  const command = resourceCommandFixture('correct');
  command.change.expected_revision = 3;
  command.change.values = structuredClone(state.resource.values);
  command.change.values.title = 'Cabeza del recurso posterior';
  state.resourceCommit(state.resourcePrepare(command));
  await login(page);
  await openActivityScope(page, state, undefined, 'Cabeza del recurso posterior');
  form = await beginResourceDeadline(page);
  await expect(titleField(form)).toHaveValue(rawDeadline.title);
  await expect(confirmDeadlineButton(form)).toHaveCount(0);
  await expect(prepareDeadlineButton(form)).toBeDisabled();
  await form.getByRole('button', { name: 'Comparar con registro actual', exact: true }).click();
  await expect(form).toContainText('Cabeza del recurso posterior');
  await expect(prepareDeadlineButton(form)).toBeDisabled();
  await form
    .getByRole('button', { name: 'Usar base actual y conservar borrador', exact: true })
    .click();
  await prepareResourceDeadline(state, form, false);
  const next = state.compositePrepares[1].values;
  expect(next.association_id).toBe(original.association_id);
  expect(next.deadline.deadline_id).toBe(original.deadline.deadline_id);
  expect(next.deadline.operation_id).not.toBe(original.deadline.operation_id);
  expect(next.expected_resource_revision).toBe(4);
  expect(next.resource).toEqual(resourceRef(state.original));
  expect(next.act).toEqual(original.act);
  expect(next.deadline.change).toEqual(original.deadline.change);
  expect(state.compositePosts).toEqual([]);
});
