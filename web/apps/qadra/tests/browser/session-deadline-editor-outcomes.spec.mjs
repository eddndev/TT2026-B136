import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import {
  deadlineEditorSetup,
  checkDeadlineEditorRequests,
  holdDeadlineRead,
  deadlinesPath,
  initialDeadline,
} from './session-deadline-editor-fixtures.mjs';
import {
  editor,
  rawDeadline,
  titleField,
  prepareButton,
  confirmButton,
  enterDeadlines,
  beginDeadline,
  reopenDeadline,
  interruptDeadline,
} from './session-deadline-editor-ui.mjs';

test.afterEach(async ({ page }) => checkDeadlineEditorRequests(page));

test('an interrupted ordinary deadline stays uncertain after an absent exact revision and another expiry never replays its write', async ({
  page,
}) => {
  const state = await deadlineEditorSetup(page);
  const { sent, submitted } = await interruptDeadline(page, state);
  let form = await reopenDeadline(page);
  await expect(titleField(form)).toHaveValue(rawDeadline.title);
  await expect(prepareButton(form)).toBeDisabled();
  await expect(confirmButton(form)).toHaveCount(0);
  const path = `${deadlinesPath}/${submitted.command.deadline_id}/revisions/1`;
  const exact = holdDeadlineRead(state, 'GET', path);
  await form.getByRole('button', { name: 'Consultar envio exacto', exact: true }).click();
  await expect.poll(() => exact.entered).toBe(true);
  expect(state.deadlinePosts.map((row) => row.values)).toEqual([submitted]);
  exact.release();
  await expect(form.getByRole('alert')).toContainText('sigue incierto');
  await expect(prepareButton(form)).toBeDisabled();
  await expire(page, state, await form.elementHandle());
  form = await reopenDeadline(page);
  const start = state.calls.length;
  await expect(titleField(form)).toHaveValue(rawDeadline.title);
  await expect(prepareButton(form)).toBeDisabled();
  await form.getByRole('button', { name: 'Consultar envio exacto', exact: true }).click();
  await expect(form.getByRole('alert')).toContainText('sigue incierto');
  expect(
    state.calls.slice(start).filter((row) => row.method === 'GET' && row.path === path),
  ).toHaveLength(1);
  expect(state.deadlinePosts.map((row) => row.values)).toEqual([submitted]);
  expect(state.deadlinePrepares).toHaveLength(1);
  sent.release();
});

test('a foreign retirement receipt cannot confirm an interrupted deadline retirement even when its reason is identical', async ({
  page,
}) => {
  const original = initialDeadline(),
    state = await deadlineEditorSetup(page, { deadlines: [original] });
  const { sent, submitted } = await interruptDeadline(page, state, { record: original });
  const other = structuredClone(submitted.command);
  other.operation_id = 'f0000000-0000-4000-8000-000000000078';
  state.deadlineCommit(state.deadlinePrepare(other));
  const form = await reopenDeadline(page);
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(rawDeadline.reason);
  await expect(prepareButton(form)).toBeDisabled();
  const start = state.calls.length;
  await form.getByRole('button', { name: 'Consultar envio exacto', exact: true }).click();
  await expect(form.getByRole('alert')).toContainText('otro envio');
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(rawDeadline.reason);
  await expect(prepareButton(form)).toBeDisabled();
  await expect(confirmButton(form)).toHaveCount(0);
  expect(
    state.calls
      .slice(start)
      .filter(
        (row) => row.method === 'GET' && row.path === `${deadlinesPath}/${original.id}/revisions/2`,
      ),
  ).toHaveLength(1);
  expect(state.deadlinePosts.map((row) => row.values)).toEqual([submitted]);
  expect(state.deadlinePrepares).toHaveLength(1);
  sent.release();
});

test('an exact ordinary deadline receipt clears its draft before parent refresh and late responses cannot restore it', async ({
  page,
}) => {
  const state = await deadlineEditorSetup(page);
  const { sent, submitted } = await interruptDeadline(page, state, { commit: true });
  let form = await reopenDeadline(page);
  const node = await form.elementHandle();
  const exact = holdDeadlineRead(
    state,
    'GET',
    `${deadlinesPath}/${submitted.command.deadline_id}/revisions/1`,
  );
  const refresh = holdDeadlineRead(state, 'GET', deadlinesPath);
  await form.getByRole('button', { name: 'Consultar envio exacto', exact: true }).click();
  await expect.poll(() => exact.entered).toBe(true);
  expect(refresh.entered).toBe(false);
  exact.release();
  await expect.poll(() => refresh.entered).toBe(true);
  await expire(page, state, node);
  await login(page);
  await enterDeadlines(page);
  await expect(
    page.getByRole('button', { name: 'Retomar borrador de plazo', exact: true }),
  ).toHaveCount(0);
  form = await beginDeadline(page);
  await expect(titleField(form)).toHaveValue('');
  await expect(
    form.getByRole('button', { name: 'Consultar envio exacto', exact: true }),
  ).toHaveCount(0);
  await expect(confirmButton(form)).toHaveCount(0);
  await titleField(form).fill('  Nuevo borrador independiente  ');
  refresh.release();
  sent.release();
  await expect(titleField(form)).toHaveValue('  Nuevo borrador independiente  ');
  expect(state.deadlinePosts.map((row) => row.values)).toEqual([submitted]);
  expect(state.deadlinePrepares).toHaveLength(1);
});
