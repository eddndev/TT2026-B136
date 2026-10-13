import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import {
  subjectDraftSetup,
  checkSubjectRequests,
  holdSubjectRequest,
  subject,
  subjectId,
  subjectPath,
  casePath,
  typed,
} from './session-subject-draft-fixtures.mjs';
import { exactSupportPath } from './session-subject-document-fixtures.mjs';
import {
  raw,
  openIdentity,
  readIdentity,
  editReadIdentity,
  fillIdentity,
  expectIdentity,
  saveSubject,
  prepareIdentity,
  reviewIdentity,
  expectFreshSubject,
  setCandidate,
  declareDifferent,
  pickExactSupport,
  candidateRegion,
} from './session-subject-draft-ui.mjs';

test.afterEach(async ({ page }) => checkSubjectRequests(page));

test('same-account identity recovery authorizes the case and subject before exposing invalid raw fields', async ({
  page,
}) => {
  const state = await subjectDraftSetup(page);
  await login(page);
  const initial = await openIdentity(page);
  await expire(page, state, await fillIdentity(initial, true));
  await login(page, true);
  await readIdentity(page);
  const gate = holdSubjectRequest(state, 'GET', casePath);
  const previousBearer = `Bearer ${state.current.token}`;
  const delayed = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === casePath &&
      response.request().headers().authorization === previousBearer,
  );
  let modal = await editReadIdentity(page);
  await expect.poll(() => gate.entered).toBe(true);
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).not.toHaveValue(raw.name);
  await expect(saveSubject(modal)).toBeDisabled();
  await expire(page, state, await modal.elementHandle());
  await login(page);
  await readIdentity(page);
  const before = state.calls.length;
  modal = await editReadIdentity(page);
  await expectIdentity(modal, true);
  gate.release();
  expect((await delayed).status()).toBe(200);
  await expectIdentity(modal, true);
  expectFreshSubject(state, before);
  expect(
    state.calls
      .slice(before)
      .some(
        (call) =>
          call.path === exactSupportPath &&
          call.headers.authorization === `Bearer ${state.current.token}`,
      ),
  ).toBe(true);
  expect(state.reviews).toEqual([]);
  expect(state.subjectWrites).toEqual([]);
  await modal
    .getByRole('button', { name: 'Revisar coincidencias de identidad', exact: true })
    .click();
  await expect(modal.getByRole('alert')).toContainText('CURP');
  await expectIdentity(modal, true);
  expect(state.reviews).toEqual([]);
  expect(state.subjectWrites).toEqual([]);
});

test('fresh identity candidates preserve written reasons but never reuse approval for another candidate revision', async ({
  page,
}) => {
  const state = await subjectDraftSetup(page);
  setCandidate(state);
  await login(page);
  let modal = await openIdentity(page);
  await fillIdentity(modal);
  await prepareIdentity(page, modal, state);
  await pickExactSupport(await declareDifferent(modal));
  await expire(page, state, await modal.elementHandle());
  await login(page);
  modal = await openIdentity(page);
  await expectIdentity(modal);
  await expect(saveSubject(modal)).toBeDisabled();
  expect(state.reviews).toHaveLength(1);
  await reviewIdentity(page, modal, state);
  await expect(
    modal.getByLabel('Motivo de selecci\u00f3n de identidad', { exact: true }),
  ).toHaveValue(raw.reason);
  await expect(
    candidateRegion(modal).getByLabel('Motivo de candidato distinto', { exact: true }),
  ).toHaveValue(raw.different);
  await expire(page, state, await modal.elementHandle());
  setCandidate(state, 2);
  await login(page);
  modal = await openIdentity(page);
  await expectIdentity(modal);
  await expect(saveSubject(modal)).toBeDisabled();
  expect(state.reviews).toHaveLength(2);
  await reviewIdentity(page, modal, state);
  const current = candidateRegion(modal);
  await expect(current).toContainText('revisi\u00f3n 2');
  await expect(current.getByLabel('Motivo de candidato distinto', { exact: true })).toHaveCount(0);
  await expect
    .poll(() =>
      modal.locator('textarea:visible').evaluateAll((fields) => fields.map((field) => field.value)),
    )
    .toContain(raw.different);
  await current.getByRole('button', { name: 'Declarar persona distinta', exact: true }).click();
  await expect(current.getByLabel('Motivo de candidato distinto', { exact: true })).toHaveValue('');
  await saveSubject(modal).click();
  await expect(modal.getByRole('alert')).toContainText('Motivo de candidato distinto');
  expect(state.subjectWrites).toEqual([]);
});

test('a concurrent subject head requires explicit adoption and a fresh identity review before replacement', async ({
  page,
}) => {
  const state = await subjectDraftSetup(page);
  await login(page);
  let modal = await openIdentity(page);
  await fillIdentity(modal);
  await prepareIdentity(page, modal, state);
  await expire(page, state, await modal.elementHandle());
  state.subjects.get(subjectId).push({
    ...subject,
    revision: 2,
    values: { ...subject.values, name: { state: 'known', value: 'Nombre de otra revision' } },
  });
  await login(page);
  modal = await openIdentity(page);
  await expectIdentity(modal);
  const comparison = modal.getByRole('region', {
    name: 'Revisi\u00f3n de identidad consultada',
    exact: true,
  });
  await expect(comparison).toContainText('Nombre de otra revision');
  await expect(saveSubject(modal)).toBeDisabled();
  expect(state.reviews).toHaveLength(1);
  await comparison
    .getByRole('button', {
      name: 'Usar esta base y conservar el formulario de identidad',
      exact: true,
    })
    .click();
  await expectIdentity(modal);
  await expect(saveSubject(modal)).toBeDisabled();
  const review = await reviewIdentity(page, modal, state);
  expect(review.values.expected_revision).toBe(2);
  expect(review.values.values.name).toEqual({ state: 'known', value: raw.name.trim() });
  await modal.getByLabel('Motivo de selecci\u00f3n de identidad', { exact: true }).fill(raw.reason);
  state.nextSubjectWrite = {};
  await saveSubject(modal).click();
  await expect(modal).toBeHidden();
  expect(state.subjectWrites).toHaveLength(1);
  expect(state.subjectWrites[0]).toMatchObject({
    path: subjectPath,
    method: 'PUT',
    search: '',
    headers: { authorization: `Bearer ${state.current.token}` },
    values: { expected_revision: 2, review: { directory_stamp: state.directoryStamp } },
  });
  expect(state.subjects.get(subjectId).at(-1).revision).toBe(3);
  expect(state.records.get(typed.id)[0].subject.revision).toBe(1);
});
