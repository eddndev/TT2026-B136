import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { openParticipants } from './session-participant-drafts-helpers.mjs';
import {
  typedDraftSetup,
  checkTypedRequests,
  holdTypedRequest,
  casePath,
  typed,
  typedPath,
  subject,
  subjectId,
} from './session-typed-draft-fixtures.mjs';
import { openChildUpload, fillChild } from './session-subject-draft-ui.mjs';
import {
  createTyped,
  beginTyped,
  editTyped,
  fillTypedOwn,
  expectTypedOwn,
  rawTyped,
  roleSupport,
  confirmTyped,
  prepareTypedFresh,
} from './session-typed-draft-ui.mjs';

test.afterEach(async ({ page }) => checkTypedRequests(page));

test('a typed creation authorizes fresh context before restoring raw invalid fields and its role upload', async ({
  page,
}) => {
  const state = await typedDraftSetup(page);
  await login(page);
  let modal = await createTyped(page);
  await fillTypedOwn(modal, true);
  let upload = await openChildUpload(page, roleSupport(modal));
  await expire(page, state, await fillChild(upload, 'typed-role.txt', 'Typed role bytes\n'));
  await login(page, true);
  await openParticipants(page);
  const gate = holdTypedRequest(state, 'GET', casePath);
  modal = await beginTyped(page);
  await expect.poll(() => gate.entered).toBe(true);
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).not.toHaveValue(
    rawTyped.name,
  );
  await expect(modal.getByRole('button', { name: 'Procesando...', exact: true })).toBeDisabled();
  await expect(
    modal.getByRole('button', { name: 'Revisar identidad y coincidencias', exact: true }),
  ).toBeDisabled();
  gate.release();
  await expectTypedOwn(modal, true);
  upload = await openChildUpload(page, roleSupport(modal));
  await expect(upload.getByText('typed-role.txt', { exact: true })).toBeVisible();
  await expect(upload.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue(
    '  etiqueta parcial  ',
  );
  expect(state.uploads).toEqual([]);
  await upload.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await modal
    .getByRole('button', { name: 'Revisar identidad y coincidencias', exact: true })
    .click();
  await expect(modal.getByRole('alert')).toContainText('CURP');
  expect(state.typedReviews).toEqual([]);
  expect(state.typedPreparations).toEqual([]);
  expect(state.typedCommits).toEqual([]);
});

test('typed replacement compares the current participant but keeps its exact bound identity until explicit adoption', async ({
  page,
}) => {
  const state = await typedDraftSetup(page);
  await login(page);
  let modal = await editTyped(page);
  await modal
    .getByLabel('Organizaci\u00f3n (opcional)', { exact: true })
    .fill(rawTyped.organization);
  await expire(page, state, await modal.elementHandle());
  state.records.get(typed.id).push({ ...typed, revision: 2, organization: 'Otra oficina vigente' });
  state.subjects.get(subjectId).push({
    ...subject,
    revision: 2,
    values: {
      ...subject.values,
      name: { state: 'known', value: 'Identidad posterior independiente' },
    },
  });
  await login(page);
  const before = state.calls.length;
  modal = await editTyped(page);
  await expect(modal.getByLabel('Organizaci\u00f3n (opcional)', { exact: true })).toHaveValue(
    rawTyped.organization,
  );
  const comparison = modal.getByRole('region', { name: 'Ficha actual consultada', exact: true });
  await expect(comparison).toContainText('Otra oficina vigente');
  await expect(confirmTyped(modal)).toBeDisabled();
  await expect(
    modal.getByRole('button', { name: 'Elegir identidad existente', exact: true }),
  ).toHaveCount(0);
  expect(
    state.calls
      .slice(before)
      .some(
        (call) =>
          call.path === typedPath && call.headers.authorization === `Bearer ${state.current.token}`,
      ),
  ).toBe(true);
  await comparison
    .getByRole('button', { name: 'Usar esta base y conservar mi formulario', exact: true })
    .click();
  await prepareTypedFresh(page, modal, state);
  expect(state.typedReviews.at(-1).values).toMatchObject({
    participant: { id: typed.id, expected_revision: 2 },
    subject: { operation: 'keep', reference: { id: subjectId, revision: 1 } },
    role: { organization: rawTyped.organization.trim() },
  });
  state.nextTypedCommit = {};
  await confirmTyped(modal).click();
  await expect(modal).toBeHidden();
  expect(state.typedCommits).toHaveLength(1);
  expect(state.records.get(typed.id).at(-1).revision).toBe(3);
  expect(state.records.get(typed.id).at(-1).subject.revision).toBe(1);
});
