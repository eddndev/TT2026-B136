import { test, expect } from '@playwright/test';
import { expire } from './session-inactivity-helpers.mjs';
import {
  resourceDeadlineDraftSetup,
  checkResourceDeadlineRequests,
} from './session-resource-deadline-fixtures.mjs';
import {
  editor,
  interruptResourceDeadline,
  reopenResourceDeadline,
  checkBothReceipts,
  prepareDeadlineButton,
  retryDeadlineButton,
  originDeadlineButton,
  titleField,
  rawDeadline,
} from './session-resource-deadline-ui.mjs';

test.afterEach(async ({ page }) => checkResourceDeadlineRequests(page));

test('an interrupted composite creation rereads both absent revisions after every expiry before an explicit exact resend', async ({
  page,
}) => {
  const state = await resourceDeadlineDraftSetup(page);
  const { sent, submitted } = await interruptResourceDeadline(page, state);
  let form = await reopenResourceDeadline(page, state);
  await expect(
    form.getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  await expect(titleField(form)).toHaveValue(rawDeadline.title);
  await expect(prepareDeadlineButton(form)).toBeDisabled();
  await expect(retryDeadlineButton(form)).toHaveCount(0);
  await checkBothReceipts(state, form, submitted);
  await expect(retryDeadlineButton(form)).toBeEnabled();
  expect(state.compositePosts.map((row) => row.values)).toEqual([submitted]);
  await expire(page, state, await form.elementHandle());
  form = await reopenResourceDeadline(page, state);
  await expect(retryDeadlineButton(form)).toHaveCount(0);
  await expect(prepareDeadlineButton(form)).toBeDisabled();
  await checkBothReceipts(state, form, submitted);
  await expect(retryDeadlineButton(form)).toBeEnabled();
  state.nextCompositeWrite = {};
  await retryDeadlineButton(form).click();
  await expect(editor(page)).toHaveCount(0);
  expect(state.compositePosts.map((row) => row.values)).toEqual([submitted, submitted]);
  expect(state.compositePrepares).toHaveLength(1);
  expect(state.jointOperations.size).toBe(1);
  sent.release();
});

test('paired exact receipts preserve uncertainty until explicit server confirmation of their joint origin even after case closure', async ({
  page,
}) => {
  const state = await resourceDeadlineDraftSetup(page);
  const { sent, submitted } = await interruptResourceDeadline(page, state, { commit: true });
  state.caseStatus = 'closed';
  state.caseRevision++;
  const form = await reopenResourceDeadline(page, state, true);
  await expect(titleField(form)).toHaveValue(rawDeadline.title);
  await expect(titleField(form)).toBeDisabled();
  await expect(prepareDeadlineButton(form)).toBeDisabled();
  await checkBothReceipts(state, form, submitted);
  await expect(originDeadlineButton(form)).toBeEnabled();
  await expect(
    form.getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  expect(state.compositePosts.map((row) => row.values)).toEqual([submitted]);
  state.nextCompositeWrite = {};
  await originDeadlineButton(form).click();
  await expect(editor(page)).toHaveCount(0);
  expect(state.compositePosts.map((row) => row.values)).toEqual([submitted, submitted]);
  expect(state.compositePrepares).toHaveLength(1);
  sent.release();
});

test('partial foreign and separately recorded paired receipts never establish a confirmed composite origin', async ({
  page,
}) => {
  const state = await resourceDeadlineDraftSetup(page);
  const { sent, submitted, draft } = await interruptResourceDeadline(page, state);
  const form = await reopenResourceDeadline(page, state);
  state.publishCompositeReceipts(draft, { association: false });
  await checkBothReceipts(state, form, submitted);
  await expect(form.getByRole('alert')).toContainText('no corresponden');
  await expect(retryDeadlineButton(form)).toHaveCount(0);
  await expect(originDeadlineButton(form)).toHaveCount(0);
  state.publishCompositeReceipts(draft);
  const foreign = state.associations.get(submitted.command.association_id)[0];
  foreign.receipt.operation_id = 'e0000000-0000-4000-8000-000000000097';
  await checkBothReceipts(state, form, submitted);
  await expect(form.getByRole('alert')).toContainText('no corresponden');
  await expect(retryDeadlineButton(form)).toHaveCount(0);
  await expect(originDeadlineButton(form)).toHaveCount(0);
  expect(state.compositePosts.map((row) => row.values)).toEqual([submitted]);
  state.publishCompositeReceipts(draft);
  await checkBothReceipts(state, form, submitted);
  await expect(originDeadlineButton(form)).toBeEnabled();
  state.nextCompositeWrite = {};
  await originDeadlineButton(form).click();
  await expect(form.getByRole('alert')).toContainText('no confirma el origen conjunto');
  await expect(
    form.getByRole('heading', { name: 'Resultado incierto', exact: true }),
  ).toBeVisible();
  await expect(titleField(form)).toHaveValue(rawDeadline.title);
  await expect(prepareDeadlineButton(form)).toBeDisabled();
  await expect(originDeadlineButton(form)).toHaveCount(0);
  await expect(retryDeadlineButton(form)).toHaveCount(0);
  expect(state.jointOperations.size).toBe(0);
  expect(state.compositePosts.map((row) => row.values)).toEqual([submitted, submitted]);
  expect(state.compositePrepares).toHaveLength(1);
  sent.release();
});
