import { test, expect } from '@playwright/test';
import { login, selectCase } from './helpers.mjs';
import { expire, signInOther, casePath } from './session-inactivity-helpers.mjs';
import {
  participantDraftSetup,
  checkParticipantDraftRequests,
  openParticipants,
  createManual,
  editManual,
  fillManual,
  expectManual,
  holdParticipantRead,
  fieldLabels,
  rawFields,
  participant,
  participantId,
  participantsPath,
  participantPath,
} from './session-participant-drafts-helpers.mjs';

test.afterEach(async ({ page }) => checkParticipantDraftRequests(page));

test('manual creation restores four raw fields after same-account MFA and fresh case authorization without posting', async ({
  page,
}) => {
  const state = await participantDraftSetup(page);
  await login(page);
  await openParticipants(page);
  await expire(page, state, await fillManual(await createManual(page)));
  await login(page, true);
  expect(state.grants.at(-1)).toMatchObject({ factor: 'recovery', user: state.grants[0].user });
  await openParticipants(page);
  await expect(
    page.getByRole('dialog', { name: 'Agregar participante', exact: true }),
  ).toBeHidden();
  const before = state.calls.length,
    gate = holdParticipantRead(state, casePath);
  const modal = await createManual(page);
  await expect.poll(() => gate.entered).toBe(true);
  await expect(modal.getByLabel(fieldLabels[0], { exact: true })).toHaveValue('');
  await expect(
    modal.getByRole('button', { name: 'Guardar participante', exact: true }),
  ).toBeDisabled();
  expect(state.confirmedWrites).toEqual([]);
  gate.release();
  state.gate = null;
  await expectManual(modal);
  const reads = state.calls.slice(before).filter((call) => call.path === casePath);
  expect(reads).toHaveLength(1);
  expect(reads[0]).toMatchObject({
    method: 'GET',
    body: null,
    search: '',
    headers: { authorization: `Bearer ${state.current.token}` },
  });
  const storage = await page.evaluate(() =>
    JSON.stringify([Object.entries(localStorage), Object.entries(sessionStorage)]),
  );
  expect(storage).not.toContain(rawFields[0]);
  expect(state.confirmedWrites).toEqual([]);
  await modal.getByRole('button', { name: 'Guardar participante', exact: true }).click();
  await expect(modal.getByLabel(fieldLabels[1], { exact: true })).toHaveAttribute(
    'aria-invalid',
    'true',
  );
  await expectManual(modal);
  expect(state.confirmedWrites).toEqual([]);
  await modal.getByLabel(fieldLabels[1], { exact: true }).fill('  Defensa declarada  ');
  state.allowWrite = { method: 'POST', path: participantsPath };
  await modal.getByRole('button', { name: 'Guardar participante', exact: true }).click();
  await expect(modal).toBeHidden();
  expect(state.confirmedWrites).toHaveLength(1);
  expect(state.confirmedWrites[0]).toMatchObject({
    method: 'POST',
    path: participantsPath,
    search: '',
    headers: { authorization: `Bearer ${state.current.token}` },
  });
  expect(state.confirmedWrites[0].values).toEqual({
    display_name: rawFields[0].trim(),
    procedural_role: 'Defensa declarada',
    organization: rawFields[2].trim(),
    legal_status: rawFields[3].trim(),
  });
});

test('manual editing compares a concurrent revision and status before an explicit replacement decision', async ({
  page,
}) => {
  const state = await participantDraftSetup(page);
  const values = [rawFields[0], '  Rol pendiente  ', rawFields[2], rawFields[3]];
  await login(page);
  await openParticipants(page);
  await expire(page, state, await fillManual(await editManual(page), values));
  state.records.get(participantId).push({
    ...participant,
    revision: 2,
    display_name: 'Cambio concurrente',
    organization: 'Organizacion actual',
    directory_status: 'archived',
  });
  await login(page);
  await openParticipants(page);
  await page.getByLabel('Estado del directorio', { exact: true }).selectOption('all');
  await page.getByRole('button', { name: 'Aplicar filtros', exact: true }).click();
  const before = state.calls.length;
  const modal = await editManual(page, 'Cambio concurrente');
  await expectManual(modal, values);
  const comparison = modal.getByRole('region', { name: 'Valores actuales guardados', exact: true });
  await expect(comparison).toContainText('Revisi\u00f3n 2');
  await expect(comparison).toContainText('Cambio concurrente');
  await expect(comparison).toContainText('Archivado');
  await expect(comparison).toContainText('incluido el estado');
  const save = modal.getByRole('button', { name: 'Guardar mis cambios', exact: true });
  await expect(save).toBeDisabled();
  const reads = state.calls.slice(before).filter((call) => call.method === 'GET');
  const caseIndex = reads.findIndex((call) => call.path === casePath);
  const resourceIndex = reads.findIndex(
    (call, index) => index > caseIndex && call.path === participantPath,
  );
  expect(caseIndex).toBeGreaterThanOrEqual(0);
  expect(resourceIndex).toBeGreaterThan(caseIndex);
  for (const call of [reads[caseIndex], reads[resourceIndex]])
    expect(call.headers.authorization).toBe(`Bearer ${state.current.token}`);
  expect(state.confirmedWrites).toEqual([]);
  await modal.getByRole('checkbox', { name: /He comparado los valores actuales/ }).check();
  expect(state.confirmedWrites).toEqual([]);
  state.allowWrite = { method: 'PUT', path: participantPath };
  await save.click();
  await expect(modal).toBeHidden();
  expect(state.confirmedWrites).toHaveLength(1);
  expect(state.confirmedWrites[0].values).toEqual({
    expected_revision: 2,
    display_name: values[0].trim(),
    procedural_role: values[1].trim(),
    organization: values[2].trim(),
    legal_status: values[3].trim(),
    directory_status: 'active',
  });
  expect(state.confirmedWrites[0].headers.authorization).toBe(`Bearer ${state.current.token}`);
  expect(state.records.get(participantId).at(-1).revision).toBe(3);
});

test('cancelling and changing account discard suspended manual fields without reviving them later', async ({
  page,
}) => {
  const state = await participantDraftSetup(page);
  const blank = ['', '', '', ''];
  await login(page);
  await openParticipants(page);
  await expire(page, state, await fillManual(await createManual(page)));
  await login(page);
  await openParticipants(page);
  let modal = await createManual(page);
  await expectManual(modal);
  const closedElement = await modal.elementHandle();
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await expire(page, state, closedElement);
  await login(page);
  await openParticipants(page);
  modal = await createManual(page);
  await expectManual(modal, blank);
  await expire(page, state, await fillManual(modal));
  await signInOther(page);
  await selectCase(page);
  await openParticipants(page);
  modal = await createManual(page);
  await expectManual(modal, blank);
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  await login(page);
  await openParticipants(page);
  await expectManual(await createManual(page), blank);
  expect(state.confirmedWrites).toEqual([]);
});
