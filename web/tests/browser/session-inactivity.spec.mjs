import { test, expect } from '@playwright/test';
import { document, login, navigate, selectCase } from './helpers.mjs';
import { requestCompletion } from './request-completion.mjs';
import {
  sessionSetup,
  checkSessionRequests,
  signInOther,
  openEditor,
  draft,
  expire,
  blankEditor,
  classification,
  pendingTag,
  metadataPath,
  casePath,
  documentsPath,
  editorName,
  classificationLabel,
} from './session-inactivity-helpers.mjs';

test.afterEach(async ({ page }) => checkSessionRequests(page));

test('MFA idle deadlines drive bounded trusted activity while background reads do not renew', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  await login(page);
  await expect(page.getByText(document.name, { exact: true })).toBeVisible();
  await state.advance(10001);
  const before = state.activity().length;
  const completed = requestCompletion(
    page,
    (request) => new URL(request.url()).pathname === documentsPath,
  );
  await page.getByRole('button', { name: 'Actualizar', exact: true }).dispatchEvent('click');
  const background = await completed;
  expect(background.failed).toBe(false);
  expect((await background.request.response()).status()).toBe(200);
  expect(state.activity()).toHaveLength(before);
  const originalDeadline = state.current.deadline;
  await page.getByLabel('Buscar por nombre').pressSequentially('contrato');
  await expect.poll(() => state.activity().length).toBe(before + 1);
  await page.getByLabel('Buscar por nombre').pressSequentially(' pendiente');
  await state.advance(9999);
  expect(state.activity()).toHaveLength(before + 1);
  expect(state.current.deadline).toBeGreaterThan(originalDeadline);
  expect(state.current.absolute).toBe(state.grants[0].absolute_expires_at_unix_ms);
  const activity = state.activity().at(-1);
  expect(activity).toMatchObject({ method: 'POST', body: null, search: '' });
  expect(activity.headers.authorization).toBe(`Bearer ${state.grants[0].access_token}`);
  expect(activity.headers['content-type']).toBeUndefined();
  await state.advance(state.current.deadline - state.now + 1);
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  expect(state.activity()).toHaveLength(before + 1);
});

test('expiry unmounts an open editor and same-account recovery waits for fresh context authorization', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  await login(page);
  const element = await draft(page);
  await expire(page, state, element);
  await login(page, true, false);
  expect(state.grants.at(-1).factor).toBe('recovery');
  await expect(page.getByRole('dialog', { name: editorName })).toHaveCount(0);
  expect(
    await page.locator('input').evaluateAll((nodes) => nodes.map((node) => node.value)),
  ).not.toContain(classification);
  const gate = { path: casePath, entered: false };
  gate.promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.gate = gate;
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await expect.poll(() => gate.entered).toBe(true);
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toHaveCount(0);
  await expect(page.getByRole('dialog', { name: editorName })).toHaveCount(0);
  gate.release();
  state.gate = null;
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
  await page.getByRole('link', { name: 'Documentos', exact: true }).click();
  const editor = await openEditor(page);
  await expect(editor.getByLabel(classificationLabel, { exact: true })).toHaveValue(classification);
  await expect(editor.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue(pendingTag);
  await expect(editor.getByRole('button', { name: `Quitar etiqueta: ${pendingTag}` })).toHaveCount(
    0,
  );
  const currentReads = state.calls.filter(
    (call) => call.headers.authorization === `Bearer ${state.current.token}`,
  );
  expect(currentReads.some((call) => call.path === casePath)).toBe(true);
  expect(currentReads.some((call) => call.path === metadataPath)).toBe(true);
  const stored = await page.evaluate(() =>
    JSON.stringify([Object.entries(localStorage), Object.entries(sessionStorage)]),
  );
  expect(stored).not.toContain(classification);
  expect(stored).not.toContain(pendingTag);
});

test('a different account cannot recover a draft and returning to the owner does not revive it', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  await login(page);
  await expire(page, state, await draft(page));
  await signInOther(page);
  await selectCase(page);
  await blankEditor(page);
  await page
    .getByRole('dialog', { name: editorName })
    .getByRole('button', { name: 'Cancelar', exact: true })
    .click();
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n' }).click();
  await login(page);
  await blankEditor(page);
});

test('explicit logout discards a suspended draft even before its original editor is reopened', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  await login(page);
  await expire(page, state, await draft(page));
  await login(page, false, false);
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n' }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  await login(page);
  await blankEditor(page);
});

test('denied fresh case access prevents restoration and discards that context draft', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  await login(page);
  await expire(page, state, await draft(page));
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  state.allowed = false;
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await expect(page.getByRole('alert')).toContainText('ya no tienes acceso');
  await expect(page.getByRole('dialog', { name: editorName })).toHaveCount(0);
  const token = `Bearer ${state.current.token}`;
  expect(
    state.calls.some((call) => call.headers.authorization === token && call.path === metadataPath),
  ).toBe(false);
  state.allowed = true;
  await page.getByRole('button', { name: 'Actualizar', exact: true }).click();
  await selectCase(page);
  await blankEditor(page);
});
