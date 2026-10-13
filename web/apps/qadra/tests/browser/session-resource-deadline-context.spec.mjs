import { test, expect } from '@playwright/test';
import { login, selectCase } from './helpers.mjs';
import { expire, signInOther } from './session-inactivity-helpers.mjs';
import { selectResource } from './session-resource-draft-ui.mjs';
import {
  resourceDeadlineDraftSetup,
  checkResourceDeadlineRequests,
  holdDeadlineRequest,
  activitiesPath,
  compositePath,
} from './session-resource-deadline-fixtures.mjs';
import {
  editor,
  startResourceDeadline,
  beginResourceDeadline,
  reopenResourceDeadline,
  openActivityScope,
  activityPanel,
  titleField,
  rawDeadline,
  prepareDeadlineButton,
  confirmDeadlineButton,
  prepareResourceDeadline,
} from './session-resource-deadline-ui.mjs';

test.afterEach(async ({ page }) => checkResourceDeadlineRequests(page));

test('a closed case restores a resource deadline readonly and explicit context refresh reopens it while fresh denial discards it', async ({
  page,
}) => {
  const state = await resourceDeadlineDraftSetup(page);
  let form = await startResourceDeadline(page, state);
  await expire(page, state, await form.elementHandle());
  state.caseStatus = 'closed';
  state.caseRevision++;
  form = await reopenResourceDeadline(page, state, true);
  await expect(titleField(form)).toHaveValue(rawDeadline.title);
  await expect(titleField(form)).toBeDisabled();
  await expect(prepareDeadlineButton(form)).toBeDisabled();
  state.caseStatus = 'active';
  state.caseRevision++;
  await form.getByRole('button', { name: 'Volver a consultar el contexto', exact: true }).click();
  await expect(titleField(form)).toBeEditable();
  await expect(prepareDeadlineButton(form)).toBeEnabled();
  await expire(page, state, await form.elementHandle());
  await login(page);
  await openActivityScope(page, state);
  state.allowed = false;
  await activityPanel(page).getByRole('button', { name: 'Crear plazo', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
  ).toBeVisible();
  state.allowed = true;
  await selectCase(page);
  await openActivityScope(page, state);
  form = await beginResourceDeadline(page);
  await expect(titleField(form)).toHaveValue('');
  expect(state.compositePrepares).toEqual([]);
  expect(state.compositePosts).toEqual([]);
});

test('confirmed composite creation clears its draft before a held parent refresh and never resurrects after another expiry', async ({
  page,
}) => {
  const state = await resourceDeadlineDraftSetup(page);
  let form = await startResourceDeadline(page, state);
  await prepareResourceDeadline(state, form);
  const refresh = holdDeadlineRequest(state, 'GET', activitiesPath(state.original.id));
  const node = await form.elementHandle();
  state.nextCompositeWrite = {};
  await confirmDeadlineButton(form).click();
  await expect.poll(() => refresh.entered).toBe(true);
  await expect(editor(page)).toHaveCount(0);
  expect(state.compositePosts).toHaveLength(1);
  expect(state.compositePosts[0].path).toBe(`${compositePath(state.original.id)}/submit`);
  expect(state.jointOperations.size).toBe(1);
  await expire(page, state, node);
  form = await reopenResourceDeadline(page, state);
  await expect(titleField(form)).toHaveValue('');
  await expect(form.getByRole('button', { name: 'Consultar resultado', exact: true })).toHaveCount(
    0,
  );
  await expect(confirmDeadlineButton(form)).toHaveCount(0);
  await titleField(form).fill('Una declaracion nueva');
  refresh.release();
  await expect(titleField(form)).toHaveValue('Una declaracion nueva');
  expect(state.compositePrepares).toHaveLength(1);
  expect(state.compositePosts).toHaveLength(1);
});

test('resource deadline drafts are isolated by resource and account and explicit close permanently discards only its own draft', async ({
  page,
}) => {
  const state = await resourceDeadlineDraftSetup(page);
  let form = await startResourceDeadline(page, state);
  const other = structuredClone(state.original);
  other.id = 'e0000000-0000-4000-8000-000000000096';
  other.values.title = 'Otro recurso para plazo';
  state.resources.set(other.id, [other]);
  await expire(page, state, await form.elementHandle());
  await login(page);
  await openActivityScope(page, state, undefined, other.values.title);
  form = await beginResourceDeadline(page);
  await expect(titleField(form)).toHaveValue('');
  await titleField(form).fill('Texto del otro recurso');
  await form.getByRole('button', { name: 'Cerrar formulario', exact: true }).click();
  await expect(editor(page)).toHaveCount(0);
  await selectResource(page, state.resource.values.title);
  await expect(
    activityPanel(page).getByRole('button', { name: 'Actualizar actividades', exact: true }),
  ).toBeEnabled();
  form = await beginResourceDeadline(page);
  await expect(titleField(form)).toHaveValue(rawDeadline.title);
  await form.getByRole('button', { name: 'Cerrar formulario', exact: true }).click();
  form = await beginResourceDeadline(page);
  await expect(titleField(form)).toHaveValue('');
  await titleField(form).fill('Borrador exclusivo de la primera cuenta');
  await expire(page, state, await form.elementHandle());
  await signInOther(page);
  await selectCase(page);
  await openActivityScope(page, state);
  form = await beginResourceDeadline(page);
  await expect(titleField(form)).toHaveValue('');
  await expire(page, state, await form.elementHandle());
  await login(page);
  await openActivityScope(page, state);
  form = await beginResourceDeadline(page);
  await expect(titleField(form)).toHaveValue('');
  expect(state.compositePrepares).toEqual([]);
  expect(state.compositePosts).toEqual([]);
});
