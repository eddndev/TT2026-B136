import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { typed } from './typed-participant-helpers.mjs';
import { openParticipant, detail } from './participant-helpers.mjs';
import { expire, casePath } from './session-inactivity-helpers.mjs';
import { requestCompletion } from './request-completion.mjs';
import {
  openParticipants,
  createManual,
  editManual,
  fillManual,
  expectManual,
  rawFields,
  fieldLabels,
  participantId,
  participantPath,
  participantsPath,
} from './session-participant-drafts-helpers.mjs';
import {
  edgeSetup,
  checkEdges,
  closeCaseReads,
  holdCommittedCreation,
  populateDirectory,
  expectReadonly,
  expectFreshRecordReads,
  committedId,
  holdEdgeRead,
  caseStatusReads,
  rejectClosedReplacement,
} from './session-participant-edges-helpers.mjs';

const validRaw = [rawFields[0], '  Defensa declarada  ', rawFields[2], rawFields[3]];
const normalized = {
  display_name: validRaw[0].trim(),
  procedural_role: validRaw[1].trim(),
  organization: validRaw[2].trim(),
  legal_status: validRaw[3].trim(),
};
const resume = (page) =>
  detail(page).getByRole('button', {
    name: 'Retomar borrador manual',
    exact: true,
  });
const manualDialog = (page) =>
  page.getByRole('dialog', {
    name: 'Editar participante',
    exact: true,
  });

test.afterEach(async ({ page }) => checkEdges(page));

test('a closed case exposes the pending manual editor separately and restores only readonly fields', async ({
  page,
}) => {
  const state = await edgeSetup(page);
  await login(page);
  await openParticipants(page);
  await expire(page, state, await fillManual(await editManual(page)));
  await closeCaseReads(page, state);
  await login(page);
  await openParticipants(page);
  await openParticipant(page);
  await expect(
    detail(page).getByRole('button', { name: 'Editar participante', exact: true }),
  ).toBeDisabled();
  await expect(resume(page)).toBeEnabled();
  const before = state.calls.length;
  const gate = holdEdgeRead(state, casePath);
  await resume(page).click();
  const modal = manualDialog(page);
  await expect.poll(() => gate.entered).toBe(true);
  await expect(modal.getByLabel(fieldLabels[0], { exact: true })).toHaveValue('');
  await expect(
    modal.getByRole('button', { name: 'Guardar participante', exact: true }),
  ).toBeDisabled();
  gate.release();
  state.gate = null;
  await expectManual(modal);
  await expectReadonly(modal);
  await expect(modal.getByText(/El expediente est\u00e1 cerrado\. Puedes consultar/)).toBeVisible();
  expectFreshRecordReads(state, before, participantPath);
  expect(state.confirmedWrites).toEqual([]);
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await expect(modal).toBeHidden();
  await expect(resume(page)).toHaveCount(0);
  await expect(
    detail(page).getByRole('button', { name: 'Editar participante', exact: true }),
  ).toBeDisabled();
});

test('concurrent typing keeps normal typed editing separate from the readonly manual draft', async ({
  page,
}) => {
  const state = await edgeSetup(page);
  await login(page);
  await openParticipants(page);
  await expire(page, state, await fillManual(await editManual(page)));
  state.records.get(participantId).push({ ...typed, id: participantId, revision: 2 });
  await login(page);
  await openParticipants(page);
  await openParticipant(page, typed.display_name);
  await expect(resume(page)).toBeEnabled();
  await detail(page).getByRole('button', { name: 'Editar participante', exact: true }).click();
  const typedDialog = page.getByRole('dialog', { name: 'Editar ficha tipificada', exact: true });
  await expect(typedDialog).toBeVisible();
  await expect(manualDialog(page)).toBeHidden();
  await typedDialog.getByRole('button', { name: 'Cancelar', exact: true }).click();
  const before = state.calls.length;
  await resume(page).click();
  const modal = manualDialog(page);
  await expectManual(modal);
  await expectReadonly(modal);
  await expect(modal.getByText(/Esta ficha ya tiene un perfil tipificado/)).toBeVisible();
  await expect(
    modal.getByRole('region', { name: 'Valores actuales guardados', exact: true }),
  ).toContainText('Revisi\u00f3n 2');
  expectFreshRecordReads(state, before, participantPath);
  expect(state.calls.filter((call) => call.path.includes('/participants/proposals/'))).toEqual([]);
  expect(state.confirmedWrites).toEqual([]);
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await expect(resume(page)).toHaveCount(0);
  await detail(page).getByRole('button', { name: 'Editar participante', exact: true }).click();
  await expect(typedDialog).toBeVisible();
  await expect(typedDialog.getByText(rawFields[0], { exact: true })).toHaveCount(0);
});

test('an unconfirmed manual creation reads every directory page and requires a new explicit duplicate-risk decision', async ({
  page,
}) => {
  const state = await edgeSetup(page);
  const pending = await holdCommittedCreation(page, state);
  await login(page);
  await openParticipants(page);
  const initial = await createManual(page);
  const element = await fillManual(initial, validRaw);
  const submittedBearer = `Bearer ${state.current.token}`;
  const returned = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      new URL(response.url()).pathname === participantsPath &&
      response.request().headers().authorization === submittedBearer,
  );
  await initial.getByRole('button', { name: 'Guardar participante', exact: true }).click();
  await expect.poll(() => pending.entered).toBe(true);
  expect(state.heldWrites[0].values).toEqual(normalized);
  const originalBearer = state.heldWrites[0].headers.authorization;
  await expire(page, state, element);
  populateDirectory(state);
  await login(page);
  await openParticipants(page);
  const modal = await createManual(page);
  await expectManual(modal, validRaw);
  const save = modal.getByRole('button', { name: 'Guardar participante', exact: true });
  await expect(save).toBeDisabled();
  await expect(
    modal.getByRole('status').filter({ hasText: 'No se pudo confirmar el registro anterior.' }),
  ).toBeVisible();
  pending.release();
  expect((await returned).status()).toBe(201);
  await expectManual(modal, validRaw);
  await expect(save).toBeDisabled();
  expect(state.confirmedWrites).toEqual([]);
  expect(state.heldWrites).toHaveLength(1);
  const before = state.calls.length;
  const first = holdEdgeRead(state, participantsPath);
  await modal.getByRole('button', { name: 'Consultar directorio actual', exact: true }).click();
  await expect.poll(() => first.entered).toBe(true);
  await expect(save).toBeDisabled();
  const second = holdEdgeRead(state, participantsPath);
  first.release();
  await expect.poll(() => second.entered).toBe(true);
  const decision = modal.getByRole('checkbox', { name: /He revisado el directorio completo/ });
  await expect(decision).toHaveCount(0);
  await expect(save).toBeDisabled();
  second.release();
  state.gate = null;
  await expect(decision).toBeEnabled();
  await expect(decision).not.toBeChecked();
  const listing = modal.getByRole('region', { name: 'Directorio actual consultado', exact: true });
  await expect(listing.getByRole('listitem')).toHaveCount(state.records.size);
  await expect(listing).toContainText('Ficha del directorio 51 / Archivado');
  await expect(listing).toContainText(`${normalized.display_name} / Activo`);
  const reads = state.calls.slice(before).filter((call) => call.path === participantsPath);
  expect(reads).toHaveLength(2);
  const authorization = state.calls.slice(before).find((call) => call.path === casePath);
  expect(authorization).toMatchObject({
    method: 'GET',
    body: null,
    search: '',
    headers: { authorization: `Bearer ${state.current.token}` },
  });
  expect(state.calls.indexOf(authorization)).toBeLessThan(state.calls.indexOf(reads[0]));
  for (const [index, call] of reads.entries()) {
    expect(call).toMatchObject({
      method: 'GET',
      body: null,
      headers: { authorization: `Bearer ${state.current.token}` },
    });
    const query = Object.fromEntries(new URLSearchParams(call.search));
    expect(query).toEqual({
      limit: '50',
      status: 'all',
      profile: 'all',
      ...(index ? { after_id: [...state.records.keys()].sort()[49] } : {}),
    });
  }
  await expect(save).toBeDisabled();
  await decision.check();
  await expect(save).toBeEnabled();
  expect(state.confirmedWrites).toEqual([]);
  state.allowWrite = { method: 'POST', path: participantsPath };
  await save.click();
  await expect(modal).toBeHidden();
  expect(state.confirmedWrites).toHaveLength(1);
  expect(state.confirmedWrites[0].values).toEqual(normalized);
  expect(state.confirmedWrites[0].headers.authorization).toBe(`Bearer ${state.current.token}`);
  expect(state.confirmedWrites[0].headers.authorization).not.toBe(originalBearer);
  expect(state.records.has(committedId)).toBe(true);
});

test('confirmed creation removes its draft before a held directory refresh and cannot resurrect after expiry', async ({
  page,
}) => {
  const state = await edgeSetup(page);
  await login(page);
  await openParticipants(page);
  const initial = await createManual(page);
  const element = await fillManual(initial, validRaw);
  const listing = holdEdgeRead(state, participantsPath);
  const originalBearer = `Bearer ${state.current.token}`;
  const returned = page.waitForResponse(
    (response) =>
      response.request().method() === 'GET' &&
      new URL(response.url()).pathname === participantsPath &&
      response.request().headers().authorization === originalBearer,
  );
  state.allowWrite = { method: 'POST', path: participantsPath };
  await initial.getByRole('button', { name: 'Guardar participante', exact: true }).click();
  await expect.poll(() => listing.entered).toBe(true);
  expect(state.confirmedWrites).toHaveLength(1);
  expect(state.confirmedWrites[0].values).toEqual(normalized);
  await expect(initial).toBeHidden();
  await expire(page, state, element);
  state.gate = null;
  await login(page);
  await openParticipants(page);
  const modal = await createManual(page);
  await expectManual(modal, ['', '', '', '']);
  listing.release();
  expect((await returned).status()).toBe(200);
  await expectManual(modal, ['', '', '', '']);
  await expect(modal.getByText(/No se pudo confirmar el registro anterior/)).toHaveCount(0);
  expect(state.confirmedWrites).toHaveLength(1);
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
});

test('a rejected manual save retains raw fields and explicit case reopening permits a new decision without resubmitting', async ({
  page,
}) => {
  const state = await edgeSetup(page);
  const current = { revision: 1, status: 'active' };
  await caseStatusReads(page, state, current);
  await rejectClosedReplacement(page, state, current);
  await login(page);
  await openParticipants(page);
  const modal = await editManual(page);
  await fillManual(modal, validRaw);
  const save = modal.getByRole('button', { name: 'Guardar participante', exact: true });
  const closedRefresh = requestCompletion(
    page,
    (request) => request.method() === 'GET' && new URL(request.url()).pathname === casePath,
  );
  await save.click();
  await expect.poll(() => state.closedWrites.length).toBe(1);
  await closedRefresh;
  expect(state.closedWrites[0].values).toEqual({
    expected_revision: 1,
    ...normalized,
    directory_status: 'active',
  });
  await expect(save).toBeDisabled();
  await expectManual(modal, validRaw);
  const before = state.calls.length;
  current.status = 'active';
  current.revision = 3;
  await modal.getByRole('button', { name: 'Consultar estado del expediente', exact: true }).click();
  await expect(save).toBeEnabled();
  await expectManual(modal, validRaw);
  for (const label of fieldLabels)
    await expect(modal.getByLabel(label, { exact: true })).toBeEditable();
  const reads = state.calls.slice(before).filter((call) => call.path === casePath);
  expect(reads).toHaveLength(1);
  expect(reads[0]).toMatchObject({
    method: 'GET',
    body: null,
    search: '',
    headers: { authorization: `Bearer ${state.current.token}` },
  });
  expect(state.closedWrites).toHaveLength(1);
  expect(state.confirmedWrites).toEqual([]);
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
});
