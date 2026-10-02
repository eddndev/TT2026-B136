import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import {
  subjectDraftSetup,
  checkSubjectRequests,
  holdSubjectRequest,
} from './session-subject-draft-fixtures.mjs';
import { childUploadPath, documentsPath } from './session-subject-document-fixtures.mjs';
import {
  raw,
  openIdentity,
  fillIdentity,
  prepareIdentity,
  reviewIdentity,
  setCandidate,
  declareDifferent,
  candidateRegion,
  openChildUpload,
  fillChild,
  saveSubject,
} from './session-subject-draft-ui.mjs';

test.afterEach(async ({ page }) => checkSubjectRequests(page));

test('a candidate support upload pending at expiry requires a fresh listing and explicit duplicate-risk decision', async ({
  page,
}) => {
  const state = await subjectDraftSetup(page);
  setCandidate(state);
  await login(page);
  let modal = await openIdentity(page);
  await fillIdentity(modal);
  await prepareIdentity(page, modal, state);
  let comparison = await declareDifferent(modal);
  let upload = await openChildUpload(page, comparison);
  const element = await fillChild(
    upload,
    'candidate-pending.txt',
    'Exact candidate support bytes\n',
  );
  const gate = holdSubjectRequest(state, 'POST', childUploadPath);
  const oldBearer = `Bearer ${state.current.token}`;
  const oldResponse = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === childUploadPath &&
      response.request().headers().authorization === oldBearer,
  );
  state.nextUpload = {};
  await upload.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect.poll(() => gate.entered).toBe(true);
  expect(state.uploads).toHaveLength(1);
  await expire(page, state, element);
  await login(page);
  modal = await openIdentity(page);
  await expect(saveSubject(modal)).toBeDisabled();
  expect(state.reviews).toHaveLength(1);
  await reviewIdentity(page, modal, state);
  comparison = candidateRegion(modal).getByRole('group', {
    name: 'Soporte de comparaci\u00f3n',
    exact: true,
  });
  upload = await openChildUpload(page, comparison);
  await expect(upload.getByText('candidate-pending.txt', { exact: true })).toBeVisible();
  await expect(
    upload.getByRole('button', { name: 'Quitar etiqueta: etiqueta parcial', exact: true }),
  ).toBeVisible();
  const send = upload.getByRole('button', { name: 'Cargar documento', exact: true });
  await expect(send).toBeDisabled();
  gate.release();
  expect((await oldResponse).status()).toBe(201);
  await expect(upload).toBeVisible();
  await expect(send).toBeDisabled();
  expect(state.uploads).toHaveLength(1);
  const before = state.calls.length;
  const listing = holdSubjectRequest(state, 'GET', documentsPath);
  await upload.getByRole('button', { name: 'Consultar documentos actuales', exact: true }).click();
  await expect.poll(() => listing.entered).toBe(true);
  await expect(
    upload.getByRole('button', { name: 'Cargando documento...', exact: true }),
  ).toBeDisabled();
  const decision = upload.getByRole('checkbox', {
    name: /He revisado el listado y decido iniciar otra carga/,
  });
  await expect(decision).toHaveCount(0);
  listing.release();
  await expect(decision).toBeEnabled();
  await expect(decision).not.toBeChecked();
  await expect(
    upload.getByRole('list', { name: 'Documentos actuales', exact: true }),
  ).toContainText('candidate-pending.txt');
  const reads = state.calls.slice(before).filter((call) => call.path === documentsPath);
  expect(reads).toHaveLength(1);
  expect(reads[0]).toMatchObject({
    method: 'GET',
    body: null,
    headers: { authorization: `Bearer ${state.current.token}` },
  });
  await expect(send).toBeDisabled();
  await decision.check();
  expect(state.uploads).toHaveLength(1);
  state.nextUpload = {};
  await send.click();
  await expect(upload).toBeHidden();
  expect(state.uploads).toHaveLength(2);
  for (const call of state.uploads)
    expect(call).toMatchObject({
      filename: 'candidate-pending.txt',
      name: 'candidate-pending.txt',
      type: 'text/plain',
      text: 'Exact candidate support bytes\n',
      keys: ['file', 'metadata'],
      metadata: { classification: 'Soporte en revision', tags: ['etiqueta parcial'] },
    });
  expect(state.uploads[1].headers.authorization).toBe(`Bearer ${state.current.token}`);
  expect(state.uploads[1].headers.authorization).not.toBe(oldBearer);
  expect(state.subjectWrites).toEqual([]);
  await comparison.getByLabel('P\u00e1gina o secci\u00f3n', { exact: true }).fill(raw.locator);
  const created = [...state.documents.values()].at(-1);
  state.nextSubjectWrite = {};
  await saveSubject(modal).click();
  await expect(modal).toBeHidden();
  expect(state.subjectWrites).toHaveLength(1);
  expect(state.subjectWrites[0].values.review.different[0].support).toEqual({
    document_id: created.id,
    version: created.version,
    digest: created.digest,
    locator: raw.locator.trim(),
  });
});
