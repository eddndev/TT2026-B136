import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import {
  subjectDraftSetup,
  checkSubjectRequests,
  holdSubjectRequest,
  subject,
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
  expectFreshSubject,
  reviewIdentity,
} from './session-subject-draft-ui.mjs';

test.afterEach(async ({ page }) => checkSubjectRequests(page));

test('a failed support read during identity recovery preserves raw fields for an explicit in-place retry', async ({
  page,
}) => {
  const state = await subjectDraftSetup(page);
  await login(page);
  const initial = await openIdentity(page);
  await expire(page, state, await fillIdentity(initial));
  await login(page);
  await readIdentity(page);
  const grants = state.grants.length;
  const bearer = `Bearer ${state.current.token}`;
  const before = state.calls.length;
  const failed = page.waitForEvent(
    'requestfailed',
    (request) => new URL(request.url()).pathname === exactSupportPath,
  );
  await page.route(
    (url) => url.pathname === exactSupportPath,
    async (route) => {
      const request = route.request();
      expect(request.method()).toBe('GET');
      expect(request.postData()).toBeNull();
      expect(new URL(request.url()).search).toBe('');
      expect(request.headers().authorization).toBe(bearer);
      await route.abort('connectionfailed');
    },
    { times: 1 },
  );
  const modal = await editReadIdentity(page);
  await failed;
  await expect(modal.getByRole('alert')).toContainText('No se pudo conectar');
  expectFreshSubject(state, before);
  await expectIdentity(modal);
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toBeDisabled();
  await expect(saveSubject(modal)).toBeDisabled();
  expect(state.reviews).toEqual([]);
  expect(state.subjectWrites).toEqual([]);

  const retry = modal.getByRole('button', {
    name: 'Volver a consultar el contexto',
    exact: true,
  });
  await expect(retry).toBeEnabled();
  const waiting = holdSubjectRequest(state, 'GET', exactSupportPath);
  await retry.click();
  await expect.poll(() => waiting.entered).toBe(true);
  await expectIdentity(modal);
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toBeDisabled();
  await expect(saveSubject(modal)).toBeDisabled();
  waiting.release();
  await expect(modal.getByLabel('Nombre de la persona', { exact: true })).toBeEnabled();
  await expectIdentity(modal);
  await expect(modal.getByRole('alert')).toHaveCount(0);
  await expect(saveSubject(modal)).toBeDisabled();
  expect(state.grants).toHaveLength(grants);
  expect(state.current.token).toBe(bearer.slice('Bearer '.length));
  expect(state.reviews).toEqual([]);
  expect(state.subjectWrites).toEqual([]);
  expect(state.uploads).toEqual([]);

  const reviewed = await reviewIdentity(page, modal, state);
  expect(reviewed.values).toMatchObject({
    expected_revision: subject.revision,
    values: {
      name: { state: 'known', value: raw.name.trim() },
      curp: { state: 'unknown', reason: raw.curpReason.trim() },
      identity_support: { locator: raw.locator.trim() },
    },
  });
  expect(state.subjectWrites).toEqual([]);
});
