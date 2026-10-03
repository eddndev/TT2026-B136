import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import {
  activityDraftSetup,
  checkActivityDraftRequests,
  holdActivityRequest,
  activitiesPath,
} from './session-activity-draft-fixtures.mjs';
import {
  openActivityScope,
  beginActivity,
  selectTarget,
  prepareActivity,
  prepareActivityButton,
  confirmActivityButton,
  activityEditor,
  activityDetail,
  rawActivityReason,
} from './session-activity-draft-ui.mjs';

test.afterEach(async ({ page }) => checkActivityDraftRequests(page));

test('an uncertain link requires a fresh absence check after every expiry before explicitly resending its exact command', async ({
  page,
}) => {
  const state = await activityDraftSetup(page);
  await login(page);
  state.seedResourceActs();
  await openActivityScope(page, state);
  let form = await beginActivity(page);
  await selectTarget(page, state, 'hearing');
  await prepareActivity(state, form);
  const sent = holdActivityRequest(state, 'POST', activitiesPath(state.original.id));
  state.nextActivityWrite = { commit: false, status: 503 };
  await confirmActivityButton(form).click();
  await expect.poll(() => sent.entered).toBe(true);
  const submitted = structuredClone(state.activityPosts[0].values);
  await expire(page, state, await form.elementHandle());
  await login(page);
  await openActivityScope(page, state);
  form = await beginActivity(page);
  await expect(prepareActivityButton(form)).toBeDisabled();
  await expect(
    form.getByRole('button', { name: 'Reintentar envio exacto', exact: true }),
  ).toHaveCount(0);
  await form.getByRole('button', { name: 'Consultar resultado', exact: true }).click();
  await expect(
    form.getByRole('button', { name: 'Reintentar envio exacto', exact: true }),
  ).toBeEnabled();
  await expire(page, state, await form.elementHandle());
  await login(page);
  await openActivityScope(page, state);
  form = await beginActivity(page);
  await expect(
    form.getByRole('button', { name: 'Reintentar envio exacto', exact: true }),
  ).toHaveCount(0);
  await expect(prepareActivityButton(form)).toBeDisabled();
  const before = state.calls.length;
  await form.getByRole('button', { name: 'Consultar resultado', exact: true }).click();
  await expect(
    form.getByRole('button', { name: 'Reintentar envio exacto', exact: true }),
  ).toBeEnabled();
  expect(
    state.calls
      .slice(before)
      .filter(
        (call) =>
          call.method === 'GET' &&
          call.path ===
            `${activitiesPath(state.original.id)}/${submitted.command.association_id}/revisions/1`,
      ),
  ).toHaveLength(1);
  state.nextActivityWrite = {};
  await form.getByRole('button', { name: 'Reintentar envio exacto', exact: true }).click();
  await expect(activityEditor(page)).toHaveCount(0);
  await expect(activityDetail(page)).toBeVisible();
  expect(state.activityPosts.map((row) => row.values)).toEqual([submitted, submitted]);
  expect(state.activityPrepares).toHaveLength(1);
  sent.release();
});

test('a foreign unlink receipt cannot confirm the interrupted operation or authorize replay and preserves its raw reason', async ({
  page,
}) => {
  const state = await activityDraftSetup(page);
  await login(page);
  state.seedResourceActs();
  const linked = state.seedAssociation();
  await openActivityScope(page, state);
  let form = await beginActivity(page, linked);
  await form.getByLabel('Motivo', { exact: true }).fill(rawActivityReason);
  await prepareActivity(state, form, 'unlink');
  const sent = holdActivityRequest(
    state,
    'POST',
    `${activitiesPath(state.original.id)}/${linked.id}/unlink`,
  );
  state.nextActivityWrite = { commit: false, status: 503 };
  await confirmActivityButton(form, 'unlink').click();
  await expect.poll(() => sent.entered).toBe(true);
  const submitted = structuredClone(state.activityPosts[0].values);
  await expire(page, state, await form.elementHandle());
  await login(page);
  await openActivityScope(page, state);
  form = await beginActivity(page, linked);
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(rawActivityReason);
  await expect(prepareActivityButton(form, 'unlink')).toBeDisabled();
  const foreign = state.activityPrepare(submitted.command);
  foreign.command.operation_id = 'e0000000-0000-4000-8000-000000000097';
  state.activityCommit(foreign);
  await form.getByRole('button', { name: 'Consultar resultado', exact: true }).click();
  await expect(form.getByRole('alert')).toContainText('otro');
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(rawActivityReason);
  await expect(prepareActivityButton(form, 'unlink')).toBeDisabled();
  await expect(
    form.getByRole('button', { name: 'Reintentar envio exacto', exact: true }),
  ).toHaveCount(0);
  expect(state.activityPosts.map((row) => row.values)).toEqual([submitted]);
  expect(state.activityPrepares).toHaveLength(1);
  sent.release();
});

test('an exact association receipt clears the draft before parent refresh and late responses cannot revive it after another expiry', async ({
  page,
}) => {
  const state = await activityDraftSetup(page);
  await login(page);
  state.seedResourceActs();
  await openActivityScope(page, state);
  let form = await beginActivity(page);
  await selectTarget(page, state, 'deadline');
  await prepareActivity(state, form);
  const sent = holdActivityRequest(state, 'POST', activitiesPath(state.original.id));
  state.nextActivityWrite = { status: 503 };
  await confirmActivityButton(form).click();
  await expect.poll(() => sent.entered).toBe(true);
  const submitted = structuredClone(state.activityPosts[0].values);
  await expire(page, state, await form.elementHandle());
  await login(page);
  await openActivityScope(page, state);
  form = await beginActivity(page);
  const node = await form.elementHandle(),
    before = state.calls.length,
    refresh = holdActivityRequest(state, 'GET', activitiesPath(state.original.id));
  await form.getByRole('button', { name: 'Consultar resultado', exact: true }).click();
  await expect.poll(() => refresh.entered).toBe(true);
  await expect(activityEditor(page)).toHaveCount(0);
  expect(
    state.calls
      .slice(before)
      .filter(
        (call) =>
          call.method === 'GET' &&
          call.path ===
            `${activitiesPath(state.original.id)}/${submitted.command.association_id}/revisions/1`,
      ),
  ).toHaveLength(1);
  await expire(page, state, node);
  await login(page);
  await openActivityScope(page, state);
  form = await beginActivity(page);
  await expect(form.getByRole('combobox', { name: 'Tipo de actividad', exact: true })).toHaveValue(
    '',
  );
  await expect(form.getByRole('button', { name: 'Consultar resultado', exact: true })).toHaveCount(
    0,
  );
  await expect(confirmActivityButton(form)).toHaveCount(0);
  await form
    .getByRole('combobox', { name: 'Tipo de actividad', exact: true })
    .selectOption('hearing');
  refresh.release();
  sent.release();
  await expect(form.getByRole('combobox', { name: 'Tipo de actividad', exact: true })).toHaveValue(
    'hearing',
  );
  await expect(prepareActivityButton(form)).toBeDisabled();
  expect(state.activityPosts.map((row) => row.values)).toEqual([submitted]);
  expect(state.activityPrepares).toHaveLength(1);
});
