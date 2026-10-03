import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import { expire, documentsPath } from './session-inactivity-helpers.mjs';
import {
  stageDraftSetup,
  checkStageDraftRequests,
  intermediate,
} from './session-stage-draft-fixtures.mjs';
import {
  enterStages,
  beginStage,
  stageGroup,
  fillTrial,
  chooseSupport,
  openChildUpload,
  fillStageChild,
} from './session-stage-draft-ui.mjs';

test.afterEach(async ({ page }) => checkStageDraftRequests(page));

test('trial support owners preserve independent files and raw selector input while optional removal discards only its descendants', async ({
  page,
}) => {
  const state = await stageDraftSetup(page, intermediate());
  await login(page);
  await enterStages(page);
  let form = await beginStage(page, 'trial');
  await fillTrial(form);
  await chooseSupport(form, 'Auto de apertura');
  await chooseSupport(form, 'Constancia de recepci\u00f3n');
  let opening = stageGroup(form, 'Auto de apertura');
  await opening.getByRole('button', { name: 'Elegir documento', exact: true }).click();
  let picker = opening.getByRole('region', { name: 'Seleccionar soporte exacto', exact: true });
  await picker
    .getByLabel('Nombre del soporte', { exact: true })
    .fill('  consulta aun sin enviar  ');
  await picker.getByRole('button', { name: 'Cerrar selector', exact: true }).click();
  let upload = await openChildUpload(page, opening);
  await expire(
    page,
    state,
    await fillStageChild(upload, 'auto-apertura.pdf', 'Opening order bytes\n'),
  );
  await login(page);
  await enterStages(page);
  form = await beginStage(page, 'trial');
  opening = stageGroup(form, 'Auto de apertura');
  const before = state.calls.length;
  await opening.getByRole('button', { name: 'Elegir documento', exact: true }).click();
  picker = opening.getByRole('region', { name: 'Seleccionar soporte exacto', exact: true });
  await expect(picker.getByLabel('Nombre del soporte', { exact: true })).toHaveValue(
    '  consulta aun sin enviar  ',
  );
  await expect(
    picker.getByRole('button', { name: 'contrato.pdf / versi\u00f3n actual 1', exact: true }),
  ).toBeVisible();
  const queries = state.calls.slice(before).filter((call) => call.path === documentsPath);
  expect(queries.length).toBeGreaterThan(0);
  for (const call of queries) expect(new URLSearchParams(call.search).has('name')).toBe(false);
  await picker.getByRole('button', { name: 'Cerrar selector', exact: true }).click();
  upload = await openChildUpload(page, stageGroup(form, 'Constancia de recepci\u00f3n'));
  await expire(
    page,
    state,
    await fillStageChild(upload, 'constancia.pdf', 'Receipt support bytes\n'),
  );
  await login(page);
  await enterStages(page);
  form = await beginStage(page, 'trial');
  upload = await openChildUpload(page, stageGroup(form, 'Auto de apertura'));
  await expect(upload.getByText('auto-apertura.pdf', { exact: true })).toBeVisible();
  await expect(upload.getByText('constancia.pdf', { exact: true })).toHaveCount(0);
  await expect(upload.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue(
    '  etiqueta de etapa  ',
  );
  state.nextUpload = {};
  await upload.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(upload).toBeHidden();
  await expect(stageGroup(form, 'Auto de apertura')).toContainText('auto-apertura.pdf');
  upload = await openChildUpload(page, stageGroup(form, 'Constancia de recepci\u00f3n'));
  await expect(upload.getByText('constancia.pdf', { exact: true })).toBeVisible();
  await expect(upload.getByText('auto-apertura.pdf', { exact: true })).toHaveCount(0);
  await expire(page, state, await upload.elementHandle());
  await login(page);
  await enterStages(page);
  form = await beginStage(page, 'trial');
  await stageGroup(form, 'Constancia de recepci\u00f3n')
    .getByRole('button', { name: 'Quitar soporte opcional', exact: true })
    .click();
  upload = await openChildUpload(page, stageGroup(form, 'Constancia de recepci\u00f3n'));
  await expect(upload.getByLabel('Nombre del documento', { exact: true })).toHaveValue('');
  await expect(upload.getByText('constancia.pdf', { exact: true })).toHaveCount(0);
  await expect(stageGroup(form, 'Auto de apertura')).toContainText('auto-apertura.pdf');
  expect(state.uploads).toHaveLength(1);
  expect(state.uploads[0]).toMatchObject({
    filename: 'auto-apertura.pdf',
    text: 'Opening order bytes\n',
    type: 'application/pdf',
    metadata: { classification: 'Acto declarado', tags: ['etiqueta de etapa'] },
  });
  expect(state.stagePosts).toEqual([]);
});
