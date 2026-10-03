import { test, expect } from '@playwright/test';
import { login, selectCase } from './helpers.mjs';
import { detail } from './participant-helpers.mjs';
import { expire, signInOther } from './session-inactivity-helpers.mjs';
import {
  subjectDraftSetup,
  checkSubjectRequests,
  holdSubjectRequest,
  subject,
  subjectId,
  subjectPath,
} from './session-subject-draft-fixtures.mjs';
import {
  raw,
  openIdentity,
  readIdentity,
  editReadIdentity,
  subjectModal,
  currentIdentity,
  fillIdentity,
  expectIdentity,
  saveSubject,
  prepareIdentity,
  reviewIdentity,
} from './session-subject-draft-ui.mjs';

test.afterEach(async ({ page }) => checkSubjectRequests(page));

test('an interrupted identity replacement retains its exact expectation and confirmed work cannot revive during a later read', async ({
  page,
}) => {
  const state = await subjectDraftSetup(page);
  await login(page);
  let modal = await openIdentity(page);
  await fillIdentity(modal);
  await prepareIdentity(page, modal, state);
  let element = await modal.elementHandle();
  const gate = holdSubjectRequest(state, 'PUT', subjectPath);
  const originalBearer = `Bearer ${state.current.token}`;
  const oldResponse = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === subjectPath &&
      response.request().method() === 'PUT' &&
      response.request().headers().authorization === originalBearer,
  );
  state.nextSubjectWrite = { commit: false, status: 503 };
  await saveSubject(modal).click();
  await expect.poll(() => gate.entered).toBe(true);
  const submitted = structuredClone(state.subjectWrites[0].values);
  expect(submitted.expected_revision).toBe(1);
  await expire(page, state, element);
  await login(page);
  modal = await openIdentity(page);
  await expectIdentity(modal);
  await expect(saveSubject(modal)).toBeDisabled();
  expect(state.reviews).toHaveLength(1);
  const exactPath = `${subjectPath}/revisions/2`;
  const absent = page.waitForResponse((response) => new URL(response.url()).pathname === exactPath);
  await modal
    .getByRole('button', { name: 'Consultar revisi\u00f3n enviada de identidad', exact: true })
    .click();
  expect((await absent).status()).toBe(404);
  await expect(modal.getByRole('alert')).toContainText('no encontr');
  await expectIdentity(modal);
  state.subjects.get(subjectId).push({
    ...subject,
    revision: 2,
    values: { ...subject.values, name: { state: 'known', value: 'Revision de otro envio' } },
  });
  await modal
    .getByRole('button', { name: 'Consultar revisi\u00f3n enviada de identidad', exact: true })
    .click();
  const comparison = modal.getByRole('region', {
    name: 'Revisi\u00f3n de identidad consultada',
    exact: true,
  });
  await expect(comparison).toContainText('Revision de otro envio');
  await expect(comparison).toContainText('no atribuye');
  gate.release();
  expect((await oldResponse).status()).toBe(503);
  await expectIdentity(modal);
  await expect(saveSubject(modal)).toBeDisabled();
  expect(state.subjectWrites).toHaveLength(1);
  expect(state.subjectWrites[0].values).toEqual(submitted);
  await comparison
    .getByRole('button', {
      name: 'Usar esta base y conservar el formulario de identidad',
      exact: true,
    })
    .click();
  await reviewIdentity(page, modal, state);
  await modal.getByLabel('Motivo de selecci\u00f3n de identidad', { exact: true }).fill(raw.reason);
  state.nextSubjectWrite = {};
  element = await modal.elementHandle();
  await saveSubject(modal).click();
  await expect(modal).toBeHidden();
  expect(state.subjectWrites).toHaveLength(2);
  expect(state.subjectWrites[1].values.expected_revision).toBe(2);
  const read = holdSubjectRequest(state, 'GET', subjectPath);
  const previousBearer = `Bearer ${state.current.token}`;
  const lateRead = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === subjectPath &&
      response.request().method() === 'GET' &&
      response.request().headers().authorization === previousBearer,
  );
  await detail(page)
    .getByRole('button', { name: 'Consultar identidad actual', exact: true })
    .click();
  await expect.poll(() => read.entered).toBe(true);
  await expire(page, state, element);
  await login(page);
  modal = await openIdentity(page);
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toHaveValue(
    raw.name.trim(),
  );
  read.release();
  expect((await lateRead).status()).toBe(200);
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toHaveValue(
    raw.name.trim(),
  );
  await expect(
    modal.getByRole('button', {
      name: 'Consultar revisi\u00f3n enviada de identidad',
      exact: true,
    }),
  ).toHaveCount(0);
  await expect(saveSubject(modal)).toBeDisabled();
  expect(state.subjectWrites).toHaveLength(2);
});

test('closed identity drafts remain readonly while fresh denial and a different account discard them', async ({
  page,
}) => {
  const state = await subjectDraftSetup(page);
  await login(page);
  let modal = await openIdentity(page);
  await expire(page, state, await fillIdentity(modal));
  state.caseStatus = 'closed';
  state.caseRevision = 2;
  await login(page);
  await readIdentity(page);
  await expect(
    currentIdentity(page).getByRole('button', { name: 'Editar identidad', exact: true }),
  ).toBeDisabled();
  await detail(page)
    .getByRole('button', { name: 'Retomar borrador de identidad', exact: true })
    .click();
  modal = subjectModal(page);
  await expectIdentity(modal);
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toBeDisabled();
  await expect(saveSubject(modal)).toBeDisabled();
  await expire(page, state, await modal.elementHandle());
  state.caseStatus = 'active';
  state.caseRevision = 3;
  await login(page);
  await readIdentity(page);
  state.deniedSubjects.add(subjectId);
  await currentIdentity(page)
    .getByRole('button', { name: 'Editar identidad', exact: true })
    .click();
  await expect(page.getByRole('alert')).toBeVisible();
  await expect
    .poll(() =>
      page.locator('input, textarea').evaluateAll((fields) => fields.map((field) => field.value)),
    )
    .not.toContain(raw.name);
  state.deniedSubjects.clear();
  await page
    .getByRole('region', { name: 'Directorio del expediente', exact: true })
    .getByRole('button', { name: 'Actualizar', exact: true })
    .click();
  await readIdentity(page);
  modal = await editReadIdentity(page);
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toHaveValue(
    subject.values.name.value,
  );
  await expire(page, state, await fillIdentity(modal));
  await signInOther(page);
  await selectCase(page);
  modal = await openIdentity(page);
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toHaveValue(
    subject.values.name.value,
  );
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  await login(page);
  modal = await openIdentity(page);
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toHaveValue(
    subject.values.name.value,
  );
  expect(state.subjectWrites).toEqual([]);
  expect(state.uploads).toEqual([]);
});
