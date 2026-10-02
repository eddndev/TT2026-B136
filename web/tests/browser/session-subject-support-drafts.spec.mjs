import { test, expect } from '@playwright/test';
import { login, document } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import {
  subjectDraftSetup,
  checkSubjectRequests,
  subject,
} from './session-subject-draft-fixtures.mjs';
import { exactSupportPath, documentsPath } from './session-subject-document-fixtures.mjs';
import {
  raw,
  openIdentity,
  readIdentity,
  editReadIdentity,
  fillIdentity,
  saveSubject,
  identitySupport,
  prepareIdentity,
  reviewIdentity,
  setCandidate,
  declareDifferent,
  candidateRegion,
  openChildUpload,
  fillChild,
} from './session-subject-draft-ui.mjs';

test.afterEach(async ({ page }) => checkSubjectRequests(page));

test('identity and candidate child uploads have separate owners and cancelling the identity discards pending descendants', async ({
  page,
}) => {
  const state = await subjectDraftSetup(page);
  setCandidate(state);
  await login(page);
  let modal = await openIdentity(page);
  await fillIdentity(modal);
  await prepareIdentity(page, modal, state);
  await declareDifferent(modal);
  let upload = await openChildUpload(page, identitySupport(modal));
  await expire(
    page,
    state,
    await fillChild(upload, 'identity-support.txt', 'Identity-only bytes\n'),
  );
  await login(page);
  modal = await openIdentity(page);
  await reviewIdentity(page, modal, state);
  let comparison = candidateRegion(modal).getByRole('group', {
    name: 'Soporte de comparaci\u00f3n',
    exact: true,
  });
  upload = await openChildUpload(page, comparison);
  await expect(upload.getByLabel('Nombre del documento', { exact: true })).toHaveValue('');
  await expire(
    page,
    state,
    await fillChild(upload, 'candidate-support.txt', 'Candidate-only bytes\n'),
  );
  await login(page);
  modal = await openIdentity(page);
  upload = await openChildUpload(page, identitySupport(modal));
  await expect(upload.getByText('identity-support.txt', { exact: true })).toBeVisible();
  await expect(upload.getByText('candidate-support.txt', { exact: true })).toHaveCount(0);
  await expect(upload.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true })).toHaveValue(
    '  Soporte en revision  ',
  );
  await expect(upload.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue(
    '  etiqueta parcial  ',
  );
  expect(state.uploads).toEqual([]);
  state.nextUpload = {};
  await upload.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(upload).toBeHidden();
  expect(state.uploads).toHaveLength(1);
  expect(state.uploads[0]).toMatchObject({
    filename: 'identity-support.txt',
    name: 'identity-support.txt',
    type: 'text/plain',
    text: 'Identity-only bytes\n',
    keys: ['file', 'metadata'],
    metadata: { classification: 'Soporte en revision', tags: ['etiqueta parcial'] },
    headers: { authorization: `Bearer ${state.current.token}` },
  });
  const element = await modal.elementHandle();
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await expire(page, state, element);
  await login(page);
  modal = await openIdentity(page);
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toHaveValue(
    subject.values.name.value,
  );
  await prepareIdentity(page, modal, state);
  comparison = await declareDifferent(modal);
  upload = await openChildUpload(page, comparison);
  await expect(upload.getByLabel('Nombre del documento', { exact: true })).toHaveValue('');
  await expect(upload.getByText('candidate-support.txt', { exact: true })).toHaveCount(0);
  expect(state.uploads).toHaveLength(1);
  expect(state.subjectWrites).toEqual([]);
});

test('a support picker retains unapplied search while denied exact versions cannot become restored authority', async ({
  page,
}) => {
  const state = await subjectDraftSetup(page);
  await login(page);
  let modal = await openIdentity(page);
  await fillIdentity(modal);
  let group = identitySupport(modal);
  await group.getByRole('button', { name: 'Seleccionar documento', exact: true }).click();
  let picker = group.getByRole('region', { name: 'Seleccionar soporte exacto', exact: true });
  await picker.getByLabel('Nombre del soporte', { exact: true }).fill(raw.search);
  await expire(page, state, await modal.elementHandle());
  await login(page);
  const before = state.calls.length;
  modal = await openIdentity(page);
  group = identitySupport(modal);
  await expect(group.getByLabel('P\u00e1gina o secci\u00f3n', { exact: true })).toHaveValue(
    raw.locator,
  );
  await group.getByRole('button', { name: 'Seleccionar documento', exact: true }).click();
  picker = group.getByRole('region', { name: 'Seleccionar soporte exacto', exact: true });
  await expect(picker.getByLabel('Nombre del soporte', { exact: true })).toHaveValue(raw.search);
  await expect(
    picker.getByRole('button', { name: `${document.name} / versi\u00f3n actual 1`, exact: true }),
  ).toBeVisible();
  const calls = state.calls.slice(before);
  expect(
    calls.some(
      (call) =>
        call.path === exactSupportPath &&
        call.headers.authorization === `Bearer ${state.current.token}`,
    ),
  ).toBe(true);
  for (const call of calls.filter((item) => item.path === documentsPath))
    expect(new URLSearchParams(call.search).has('name')).toBe(false);
  await picker.getByRole('button', { name: 'Cerrar selector', exact: true }).click();
  await expire(page, state, await modal.elementHandle());
  await login(page);
  await readIdentity(page);
  state.deniedVersions.add(exactSupportPath);
  modal = await editReadIdentity(page);
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toHaveValue(raw.name);
  await expect(saveSubject(modal)).toBeDisabled();
  await expect(
    identitySupport(modal).getByLabel('P\u00e1gina o secci\u00f3n', { exact: true }),
  ).toHaveCount(0);
  expect(state.reviews).toEqual([]);
  expect(state.subjectWrites).toEqual([]);
  expect(state.uploads).toEqual([]);
});
