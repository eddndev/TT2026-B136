import { test, expect } from '@playwright/test';
import { login, selectCase } from './helpers.mjs';
import { expire, signInOther } from './session-inactivity-helpers.mjs';
import {
  activityDraftSetup,
  checkActivityDraftRequests,
} from './session-activity-draft-fixtures.mjs';
import { selectResource } from './session-resource-draft-ui.mjs';
import {
  openActivityScope,
  beginActivity,
  selectTarget,
  prepareActivityButton,
  activityEditor,
  activityPanel,
  activityDetail,
  rawActivityReason,
} from './session-activity-draft-ui.mjs';

test.afterEach(async ({ page }) => checkActivityDraftRequests(page));

test('a closed case exposes an unlink draft readonly and fresh case denial discards it without any association mutation', async ({
  page,
}) => {
  const state = await activityDraftSetup(page);
  await login(page);
  state.seedResourceActs();
  const linked = state.seedAssociation();
  await openActivityScope(page, state);
  let form = await beginActivity(page, linked);
  await form.getByLabel('Motivo', { exact: true }).fill(rawActivityReason);
  await expire(page, state, await form.elementHandle());
  state.caseStatus = 'closed';
  state.caseRevision++;
  await login(page);
  await openActivityScope(page, state);
  await activityPanel(page)
    .getByRole('button', { name: 'Retomar borrador de actividad', exact: true })
    .click();
  form = activityEditor(page);
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue(rawActivityReason);
  await expect(form.getByLabel('Motivo', { exact: true })).toBeDisabled();
  await expect(prepareActivityButton(form, 'unlink')).toBeDisabled();
  state.caseStatus = 'active';
  state.caseRevision++;
  await form.getByRole('button', { name: 'Volver a consultar el contexto', exact: true }).click();
  await expect(form.getByLabel('Motivo', { exact: true })).toBeEditable();
  await expect(prepareActivityButton(form, 'unlink')).toBeEnabled();
  await expire(page, state, await form.elementHandle());
  await login(page);
  await openActivityScope(page, state);
  await activityPanel(page)
    .getByRole('button', { name: `Consultar v\u00ednculo ${linked.id}`, exact: true })
    .click();
  await expect(activityDetail(page)).toBeVisible();
  state.allowed = false;
  await activityDetail(page)
    .getByRole('button', { name: 'Desvincular actividad', exact: true })
    .click();
  await expect(
    page.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
  ).toBeVisible();
  state.allowed = true;
  await selectCase(page);
  await openActivityScope(page, state);
  form = await beginActivity(page, linked);
  await expect(form.getByLabel('Motivo', { exact: true })).toHaveValue('');
  expect(state.activityPosts).toEqual([]);
  expect(state.activityPrepares).toEqual([]);
});

test('an association draft belongs to its resource and account and explicit close does not discard another resource draft', async ({
  page,
}) => {
  const state = await activityDraftSetup(page);
  await login(page);
  state.seedResourceActs();
  const other = structuredClone(state.original);
  other.id = 'e0000000-0000-4000-8000-000000000099';
  other.values.title = 'Recurso independiente';
  state.resources.set(other.id, [other]);
  await openActivityScope(page, state);
  let form = await beginActivity(page);
  await selectTarget(page, state, 'deadline');
  await expire(page, state, await form.elementHandle());
  await login(page);
  await openActivityScope(page, state, undefined, other.values.title);
  form = await beginActivity(page);
  await expect(form.getByRole('combobox', { name: 'Tipo de actividad', exact: true })).toHaveValue(
    '',
  );
  await form.getByRole('button', { name: 'Cerrar formulario', exact: true }).click();
  await expect(activityEditor(page)).toHaveCount(0);
  await selectResource(page, state.resource.values.title);
  await expect(
    activityPanel(page).getByRole('button', { name: 'Actualizar actividades', exact: true }),
  ).toBeEnabled();
  form = await beginActivity(page);
  await expect(form.getByRole('combobox', { name: 'Tipo de actividad', exact: true })).toHaveValue(
    'deadline',
  );
  await expect(form).toContainText('Actividad elegida: revisi\u00f3n 1.');
  await expire(page, state, await form.elementHandle());
  await signInOther(page);
  await selectCase(page);
  await openActivityScope(page, state);
  form = await beginActivity(page);
  await expect(form.getByRole('combobox', { name: 'Tipo de actividad', exact: true })).toHaveValue(
    '',
  );
  await expect(form.getByRole('button', { name: 'Consultar resultado', exact: true })).toHaveCount(
    0,
  );
  await expire(page, state, await form.elementHandle());
  await login(page);
  await openActivityScope(page, state);
  form = await beginActivity(page);
  await expect(form.getByRole('combobox', { name: 'Tipo de actividad', exact: true })).toHaveValue(
    '',
  );
  expect(state.activityPosts).toEqual([]);
  expect(state.activityPrepares).toEqual([]);
});
