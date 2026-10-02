import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import { entry } from './stage-helpers.mjs';
import {
  stageDraftSetup,
  checkStageDraftRequests,
  holdStageRequest,
  casePath,
  intermediate,
  stageTime,
  support,
} from './session-stage-draft-fixtures.mjs';
import {
  enterStages,
  beginStage,
  resumeStage,
  stageGroup,
  reviewStage,
  fillIntermediate,
  expectIntermediate,
  rawStage,
  expectFreshStage,
  openChildUpload,
  fillStageChild,
  fillTrial,
  chooseSupport,
} from './session-stage-draft-ui.mjs';

test.afterEach(async ({ page }) => checkStageDraftRequests(page));

test('a stage transition restores raw dates and its child file only after fresh case and stage authorization', async ({
  page,
}) => {
  const state = await stageDraftSetup(page);
  await login(page);
  await enterStages(page);
  let form = await beginStage(page);
  await fillIntermediate(form, true);
  let upload = await openChildUpload(page, stageGroup(form, 'Acusaci\u00f3n'));
  await expire(
    page,
    state,
    await fillStageChild(upload, 'acusacion.pdf', 'Pending accusation bytes\n'),
  );
  await login(page, true);
  await enterStages(page);
  const before = state.calls.length;
  const gate = holdStageRequest(state, 'GET', casePath);
  form = await beginStage(page);
  await expect.poll(() => gate.entered).toBe(true);
  await expect(form.getByLabel('Nota (opcional)', { exact: true })).not.toHaveValue(rawStage.note);
  await expect(reviewStage(form)).toBeDisabled();
  expect(state.stagePosts).toEqual([]);
  gate.release();
  await expectIntermediate(form, true);
  expectFreshStage(state, before);
  upload = await openChildUpload(page, stageGroup(form, 'Acusaci\u00f3n'));
  await expect(upload.getByText('acusacion.pdf', { exact: true })).toBeVisible();
  await expect(upload.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue(
    '  etiqueta de etapa  ',
  );
  await upload.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await reviewStage(form).click();
  await expect(form.getByRole('alert')).toContainText('desfase');
  expect(state.stagePosts).toEqual([]);
  expect(state.uploads).toEqual([]);
});

test('stage adoption preserves its manual reason and precision without fabricating prior transitions', async ({
  page,
}) => {
  const state = await stageDraftSetup(page, null);
  await login(page);
  await enterStages(page);
  let form = await beginStage(page, 'adoption');
  await form.getByLabel('Etapa conocida', { exact: true }).selectOption('trial');
  const date = stageGroup(form, 'Fecha de la etapa conocida');
  await date.getByLabel('Fecha', { exact: true }).fill('2026-09-01');
  await date.getByLabel('Desfase UTC', { exact: true }).fill(rawStage.offset);
  await form.getByLabel('Motivo de adopci\u00f3n', { exact: true }).fill(rawStage.reason);
  await chooseSupport(form, 'Soporte de adopci\u00f3n');
  await expire(page, state, await form.elementHandle());
  await login(page);
  await enterStages(page);
  const before = state.calls.length;
  form = await beginStage(page, 'adoption');
  await expect(form.getByLabel('Motivo de adopci\u00f3n', { exact: true })).toHaveValue(
    rawStage.reason,
  );
  await expect(form.getByLabel('Etapa conocida', { exact: true })).toHaveValue('trial');
  const restored = stageGroup(form, 'Fecha de la etapa conocida');
  await expect(restored.getByLabel('Precisi\u00f3n', { exact: true })).toHaveValue('date');
  await expect(restored.getByLabel('Fecha', { exact: true })).toHaveValue('2026-09-01');
  await expect(restored.getByLabel('Desfase UTC', { exact: true })).toHaveValue(rawStage.offset);
  expectFreshStage(state, before);
  await reviewStage(form).click();
  await expect(form.getByRole('alert')).toContainText('desfase');
  await restored.getByLabel('Desfase UTC', { exact: true }).fill('-06:00');
  await reviewStage(form).click();
  await expect(form.locator('.stage-confirmation')).toContainText('revisi\u00f3n esperada 0');
  expect(state.stageHistory).toEqual([]);
  state.nextStageWrite = {};
  await form.getByRole('button', { name: 'Registrar etapa actual', exact: true }).click();
  await expect(form).toHaveCount(0);
  expect(state.stagePosts).toHaveLength(1);
  expect(state.stagePosts[0].values).toEqual({
    expected_revision: 0,
    stage: 'trial',
    known_at: stageTime,
    reason: rawStage.reason.trim(),
    support,
  });
  expect(state.stageHistory).toHaveLength(1);
  expect(state.stageHistory[0]).toMatchObject({
    kind: 'change',
    stage_revision: 1,
    from_stage: null,
  });
});

test('a stage draft keeps its original transition when a newer head no longer permits that edge', async ({
  page,
}) => {
  const state = await stageDraftSetup(page, intermediate());
  await login(page);
  await enterStages(page);
  let form = await beginStage(page, 'trial');
  await fillTrial(form);
  await chooseSupport(form, 'Auto de apertura');
  await expire(page, state, await form.elementHandle());
  const advanced = entry(
    {
      expected_revision: 2,
      target: 'trial',
      opening_order_issued_at: stageTime,
      opening_order: support,
      received_at: stageTime,
      receiving_court: 'Tribunal de otro registro',
    },
    3,
  );
  state.stageHead = advanced;
  state.stageHistory.unshift(advanced);
  await login(page);
  await enterStages(page);
  form = await resumeStage(page);
  await expect(
    form.getByRole('heading', { name: 'Registrar paso a Juicio', exact: true }),
  ).toBeVisible();
  await expect(form.getByLabel('Tribunal receptor', { exact: true })).toHaveValue(rawStage.court);
  await expect(form.getByLabel('Nota (opcional)', { exact: true })).toHaveValue(rawStage.note);
  await expect(reviewStage(form)).toBeDisabled();
  await form.getByRole('button', { name: 'Consultar etapa e historial', exact: true }).click();
  await expect(form.locator('.stage-reconciliation')).toContainText('Tribunal de otro registro');
  await expect(form.locator('.stage-reconciliation')).toContainText('ya no corresponde');
  await expect(
    form.getByRole('button', { name: 'Usar etapa consultada y revisar borrador', exact: true }),
  ).toHaveCount(0);
  expect(state.stagePosts).toEqual([]);
});
