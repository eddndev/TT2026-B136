import { test, expect } from '@playwright/test';
import { login, selectCase } from './helpers.mjs';
import { expire, signInOther } from './session-inactivity-helpers.mjs';
import { stageDraftSetup, checkStageDraftRequests } from './session-stage-draft-fixtures.mjs';
import {
  enterStages,
  beginStage,
  resumeStage,
  fillIntermediate,
  expectIntermediate,
  reviewStage,
} from './session-stage-draft-ui.mjs';

test.afterEach(async ({ page }) => checkStageDraftRequests(page));

test('closed stage drafts remain readonly while denied context, another account and explicit closure discard them', async ({
  page,
}) => {
  const state = await stageDraftSetup(page);
  await login(page);
  await enterStages(page);
  let form = await beginStage(page);
  await fillIntermediate(form);
  await expire(page, state, await form.elementHandle());
  state.caseStatus = 'closed';
  state.caseRevision++;
  await login(page);
  await enterStages(page);
  form = await resumeStage(page);
  await expectIntermediate(form);
  await expect(form.getByLabel('Nota (opcional)', { exact: true })).toBeDisabled();
  await expect(reviewStage(form)).toBeDisabled();
  await expire(page, state, await form.elementHandle());
  state.caseStatus = 'active';
  state.caseRevision++;
  await login(page);
  await enterStages(page);
  state.allowed = false;
  await page
    .locator('.stage-current')
    .getByRole('button', { name: 'Registrar paso a Intermedia', exact: true })
    .click();
  await expect(
    page.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
  ).toBeVisible();
  await expect(page.locator('.stage-form')).toHaveCount(0);
  state.allowed = true;
  await selectCase(page);
  await enterStages(page);
  form = await beginStage(page);
  await expect(form.getByLabel('Nota (opcional)', { exact: true })).toHaveValue('');
  await fillIntermediate(form);
  await expire(page, state, await form.elementHandle());
  await signInOther(page);
  await selectCase(page);
  await enterStages(page);
  form = await beginStage(page);
  await expect(form.getByLabel('Nota (opcional)', { exact: true })).toHaveValue('');
  await fillIntermediate(form);
  const element = await form.elementHandle();
  await form.getByRole('button', { name: 'Cerrar formulario de etapa', exact: true }).click();
  await expire(page, state, element);
  await signInOther(page);
  await selectCase(page);
  await enterStages(page);
  form = await beginStage(page);
  await expect(form.getByLabel('Nota (opcional)', { exact: true })).toHaveValue('');
  expect(state.stagePosts).toEqual([]);
  expect(state.uploads).toEqual([]);
});
