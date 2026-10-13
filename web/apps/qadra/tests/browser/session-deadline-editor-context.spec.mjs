import { test, expect } from '@playwright/test';
import { login, selectCase } from './helpers.mjs';
import { expire, signInOther } from './session-inactivity-helpers.mjs';
import {
  deadlineEditorSetup,
  checkDeadlineEditorRequests,
} from './session-deadline-editor-fixtures.mjs';
import {
  editor,
  rawDeadline,
  titleField,
  prepareButton,
  enterDeadlines,
  beginDeadline,
  fillOrdinaryDeadline,
} from './session-deadline-editor-ui.mjs';

test.afterEach(async ({ page }) => checkDeadlineEditorRequests(page));

test('a closed case restores its ordinary deadline draft readonly and revalidation unlocks only the reopened case', async ({
  page,
}) => {
  const state = await deadlineEditorSetup(page);
  await login(page);
  await enterDeadlines(page);
  let form = await beginDeadline(page);
  await fillOrdinaryDeadline(page);
  await expire(page, state, await form.elementHandle());
  state.caseStatus = 'closed';
  state.caseRevision++;
  await login(page);
  await enterDeadlines(page);
  form = await beginDeadline(page, 'resume');
  await expect(titleField(form)).toHaveValue(rawDeadline.title);
  await expect(titleField(form)).toBeDisabled();
  await expect(prepareButton(form)).toBeDisabled();
  state.caseStatus = 'active';
  state.caseRevision++;
  await form.getByRole('button', { name: 'Volver a consultar el contexto', exact: true }).click();
  await expect(titleField(form)).toBeEditable();
  await expect(prepareButton(form)).toBeEnabled();
  await expect(titleField(form)).toHaveValue(rawDeadline.title);
  expect(state.deadlinePrepares).toEqual([]);
  expect(state.deadlinePosts).toEqual([]);
});

test('explicit close, another account and fresh case denial discard ordinary deadline drafts without leaking or submitting them', async ({
  page,
}) => {
  const state = await deadlineEditorSetup(page);
  await login(page);
  await enterDeadlines(page);
  let form = await beginDeadline(page);
  await titleField(form).fill(rawDeadline.title);
  await form.getByRole('button', { name: 'Cerrar formulario', exact: true }).click();
  await expect(editor(page)).toHaveCount(0);
  form = await beginDeadline(page);
  await expect(titleField(form)).toHaveValue('');
  await titleField(form).fill(rawDeadline.title);
  await expire(page, state, await form.elementHandle());
  await signInOther(page);
  await selectCase(page);
  await enterDeadlines(page);
  form = await beginDeadline(page);
  await expect(titleField(form)).toHaveValue('');
  await titleField(form).fill('  Borrador privado de la otra cuenta  ');
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterDeadlines(page);
  form = await beginDeadline(page);
  await expect(titleField(form)).toHaveValue('');
  await titleField(form).fill(rawDeadline.title);
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterDeadlines(page);
  state.allowed = false;
  await page.getByRole('button', { name: 'Registrar plazo', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
  ).toBeVisible();
  state.allowed = true;
  await selectCase(page);
  await enterDeadlines(page);
  form = await beginDeadline(page);
  await expect(titleField(form)).toHaveValue('');
  expect(state.deadlinePrepares).toEqual([]);
  expect(state.deadlinePosts).toEqual([]);
});
