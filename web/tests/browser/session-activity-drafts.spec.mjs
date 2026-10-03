import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { resourceCommandFixture } from '../fixtures/procedural-resource-unit.mjs';
import {
  activityDraftSetup,
  checkActivityDraftRequests,
  holdActivityRequest,
  casePath,
} from './session-activity-draft-fixtures.mjs';
import { resourceRef, targetRef } from './session-activity-command-fixtures.mjs';
import {
  openActivityScope,
  beginActivity,
  selectTarget,
  chooseOldAct,
  prepareActivity,
  prepareActivityButton,
  confirmActivityButton,
} from './session-activity-draft-ui.mjs';

test.afterEach(async ({ page }) => checkActivityDraftRequests(page));

test('a link restores its historical resource act and hearing only after fresh case authority without preparing or mutating any owner', async ({
  page,
}) => {
  const state = await activityDraftSetup(page);
  await login(page);
  state.seedResourceActs();
  await openActivityScope(page, state, 1);
  let form = await beginActivity(page);
  await selectTarget(page, state, 'hearing');
  await chooseOldAct(form);
  await expire(page, state, await form.elementHandle());
  await login(page);
  await openActivityScope(page, state, 1);
  const fresh = holdActivityRequest(state, 'GET', casePath);
  form = await beginActivity(page);
  await expect.poll(() => fresh.entered).toBe(true);
  await expect(form.getByRole('combobox', { name: 'Tipo de actividad', exact: true })).toHaveValue(
    '',
  );
  await expect(prepareActivityButton(form)).toBeDisabled();
  fresh.release();
  await expect(form.getByRole('combobox', { name: 'Tipo de actividad', exact: true })).toHaveValue(
    'hearing',
  );
  await expect(form).toContainText('Actividad elegida: revisi\u00f3n 1.');
  await expect(
    form.getByRole('region', { name: 'Acto opcional del recurso', exact: true }),
  ).toContainText('Recurso revisi\u00f3n 2');
  expect(state.activityPrepares).toEqual([]);
  expect(state.activityPosts).toEqual([]);
  await prepareActivity(state, form);
  expect(state.activityPrepares[0].values).toMatchObject({
    expected_resource_revision: 3,
    change: {
      resource: resourceRef(state.original),
      target: targetRef('hearing', state.hearing[0]),
      act: {
        id: state.act.act.id,
        revision: 1,
        resource_revision: 2,
        capture_digest: state.act.receipt.capture_digest,
      },
    },
  });
});

test('a prepared deadline link loses approval and requires explicit adoption of a new resource head while keeping the old target capture', async ({
  page,
}) => {
  const state = await activityDraftSetup(page);
  await login(page);
  state.seedResourceActs();
  await openActivityScope(page, state);
  let form = await beginActivity(page);
  await selectTarget(page, state, 'deadline');
  await prepareActivity(state, form);
  const original = structuredClone(state.activityPrepares[0].values);
  await expire(page, state, await form.elementHandle());
  const command = resourceCommandFixture('correct');
  command.change.expected_revision = 3;
  command.change.values = structuredClone(state.resource.values);
  command.change.values.title = 'Cabeza ajena posterior';
  state.resourceCommit(state.resourcePrepare(command));
  await login(page);
  await openActivityScope(page, state, undefined, 'Cabeza ajena posterior');
  form = await beginActivity(page);
  await expect(confirmActivityButton(form)).toHaveCount(0);
  await expect(prepareActivityButton(form)).toBeDisabled();
  await expect(form.getByRole('combobox', { name: 'Tipo de actividad', exact: true })).toHaveValue(
    'deadline',
  );
  await form.getByRole('button', { name: 'Comparar con registro actual', exact: true }).click();
  await expect(form).toContainText('Cabeza ajena posterior');
  await expect(prepareActivityButton(form)).toBeDisabled();
  await form
    .getByRole('button', { name: 'Usar base actual y conservar borrador', exact: true })
    .click();
  await prepareActivity(state, form);
  const next = state.activityPrepares[1].values;
  expect(next.association_id).toBe(original.association_id);
  expect(next.operation_id).not.toBe(original.operation_id);
  expect(next.expected_resource_revision).toBe(4);
  expect(next.change).toEqual(original.change);
  expect(next.change.target).toEqual(targetRef('deadline', state.deadline[0]));
  expect(next.change.target.capture_digest).not.toBe(state.deadline[1].receipt.capture_digest);
  expect(state.activityPosts).toEqual([]);
});

test('an unfinished target selector restores its inputs and rereads the preview without turning it into an approved selection', async ({
  page,
}) => {
  const state = await activityDraftSetup(page);
  await login(page);
  state.seedResourceActs();
  await openActivityScope(page, state);
  let form = await beginActivity(page);
  await form
    .getByRole('combobox', { name: 'Tipo de actividad', exact: true })
    .selectOption('hearing');
  await form
    .getByRole('combobox', { name: 'Actividad existente', exact: true })
    .selectOption(state.hearing[0].id);
  await form
    .getByRole('combobox', { name: 'Revisi\u00f3n de la actividad', exact: true })
    .selectOption('1');
  await expect(
    form.getByRole('button', { name: 'Usar esta revisi\u00f3n', exact: true }),
  ).toBeEnabled();
  await expect(prepareActivityButton(form)).toBeDisabled();
  await expire(page, state, await form.elementHandle());
  await login(page);
  await openActivityScope(page, state);
  const path = `/api/v1/cases/${state.original.case_id}/hearings/${state.hearing[0].id}/revisions/1`;
  const exact = holdActivityRequest(state, 'GET', path);
  form = await beginActivity(page);
  await expect.poll(() => exact.entered).toBe(true);
  await expect(prepareActivityButton(form)).toBeDisabled();
  exact.release();
  await expect(
    form.getByRole('combobox', { name: 'Actividad existente', exact: true }),
  ).toHaveValue(state.hearing[0].id);
  await expect(
    form.getByRole('combobox', { name: 'Revisi\u00f3n de la actividad', exact: true }),
  ).toHaveValue('1');
  await expect(form).toContainText('Sala historica uno');
  await expect(prepareActivityButton(form)).toBeDisabled();
  expect(state.activityPrepares).toEqual([]);
  await form.getByRole('button', { name: 'Usar esta revisi\u00f3n', exact: true }).click();
  await prepareActivity(state, form);
  expect(state.activityPrepares[0].values.change.target).toEqual(
    targetRef('hearing', state.hearing[0]),
  );
  expect(state.activityPosts).toEqual([]);
});
