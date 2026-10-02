import { test, expect } from '@playwright/test';
import { document, login } from './helpers.mjs';
import { requestCompletion } from './request-completion.mjs';
import {
  sessionSetup,
  draft,
  expire,
  openEditor,
  blankEditor,
  classification,
  pendingTag,
  editorName,
  classificationLabel,
  documentsPath,
} from './session-inactivity-helpers.mjs';
import {
  visibility,
  controlRoute,
  holdControl,
  finishVisibilityRequests,
} from './session-visibility-helpers.mjs';

const sessionPath = '/api/v1/auth/session';
const logoutPath = '/api/v1/auth/logout';
const sessionReads = (state) => state.calls.filter((call) => call.path === sessionPath);

test.afterEach(async ({ page }) => finishVisibilityRequests(page));

test('returning from controlled hidden visibility confirms the session before business access', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  await login(page);
  await expect(page.getByText(document.name, { exact: true })).toBeVisible();
  const gate = await holdControl(page, sessionPath, state.current.token);
  const before = state.calls.length,
    activity = state.activity().length;
  await visibility(page, 'hidden');
  await expect(page.locator('.app-layout')).toHaveAttribute('inert', '');
  expect(gate.calls).toHaveLength(0);
  await visibility(page, 'visible');
  await expect.poll(() => gate.calls.length).toBe(1);
  await expect(page.getByRole('status')).toContainText('Comprobando tu sesi\u00f3n');
  await expect(page.locator('.app-layout')).toHaveAttribute('inert', '');
  await page
    .locator('.app-layout button')
    .filter({ hasText: /^Actualizar$/ })
    .dispatchEvent('click');
  await state.advance(1);
  expect(
    state.calls.slice(before).filter((call) => !call.path.startsWith('/api/v1/auth/')),
  ).toEqual([]);
  expect(state.activity()).toHaveLength(activity);
  gate.release();
  await expect(page.locator('.app-layout')).not.toHaveAttribute('inert', '');
  expect(sessionReads(state)).toHaveLength(1);
  const completed = requestCompletion(
    page,
    (request) => new URL(request.url()).pathname === documentsPath,
  );
  await page.getByRole('button', { name: 'Actualizar', exact: true }).click();
  const result = await completed;
  expect(result.failed).toBe(false);
  expect((await result.request.response()).status()).toBe(200);
  const admitted = state.calls.slice(before);
  expect(admitted[0].path).toBe(sessionPath);
  expect(admitted.findIndex((call) => call.path === documentsPath)).toBeGreaterThan(0);
});

test('a failed visibility read keeps business inert until an explicit successful retry', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  await login(page);
  const control = await controlRoute(page, sessionPath, state.current.token, (route, count) =>
    count === 1 ? route.abort('internetdisconnected') : route.fallback(),
  );
  const activity = state.activity().length,
    deadline = state.current.deadline;
  await visibility(page, 'hidden');
  const completed = requestCompletion(
    page,
    (request) => new URL(request.url()).pathname === sessionPath,
  );
  await visibility(page, 'visible');
  expect((await completed).failed).toBe(true);
  await expect(page.locator('.app-layout')).toHaveAttribute('inert', '');
  await expect(page.getByRole('button', { name: 'Comprobar de nuevo', exact: true })).toBeVisible();
  await state.advance(10001);
  expect(control.calls).toHaveLength(1);
  expect(state.activity()).toHaveLength(activity);
  expect(state.current.deadline).toBe(deadline);
  await page.getByRole('button', { name: 'Comprobar de nuevo', exact: true }).click();
  await expect(page.locator('.app-layout')).not.toHaveAttribute('inert', '');
  expect(control.calls).toHaveLength(2);
  expect(state.activity()).toHaveLength(activity);
  expect(state.current.deadline).toBe(deadline);
});

test('an open editor stops blocking session controls while visibility confirmation is pending', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  await login(page);
  const element = await draft(page);
  const editor = page.getByRole('dialog', { name: editorName, exact: true });
  const gate = await holdControl(page, sessionPath, state.current.token);
  const activity = state.activity().length;
  await visibility(page, 'hidden');
  await expect(editor).toBeHidden();
  expect(await element.evaluate((node) => node.contains(document.activeElement))).toBe(false);
  await visibility(page, 'visible');
  await expect.poll(() => gate.calls.length).toBe(1);
  await expect(editor).toBeHidden();
  const retry = page.getByRole('button', { name: 'Comprobar de nuevo', exact: true });
  const logout = page
    .getByRole('status')
    .getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true });
  await retry.focus();
  await expect(retry).toBeFocused();
  await logout.focus();
  await expect(logout).toBeFocused();
  await retry.click();
  expect(gate.calls).toHaveLength(1);
  expect(state.activity()).toHaveLength(activity);
  gate.release();
  await expect(editor).toBeVisible();
  await expect(editor.getByLabel(classificationLabel, { exact: true })).toHaveValue(classification);
  await expect(editor.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue(pendingTag);
});

test('focus and synthetic input events never renew the idle deadline', async ({ page }) => {
  const state = await sessionSetup(page);
  await login(page);
  await state.advance(10001);
  const activity = state.activity().length,
    reads = sessionReads(state).length,
    deadline = state.current.deadline;
  await page.getByLabel('Buscar por nombre').focus();
  await expect(page.getByLabel('Buscar por nombre')).toBeFocused();
  await page.getByLabel('Buscar por nombre').dispatchEvent('input');
  await page.getByLabel('Buscar por nombre').dispatchEvent('keydown', { key: 'Shift' });
  await page.getByLabel('Buscar por nombre').dispatchEvent('pointerdown');
  await page.evaluate(() => window.dispatchEvent(new FocusEvent('focus')));
  await state.advance(1000);
  expect(state.activity()).toHaveLength(activity);
  expect(sessionReads(state)).toHaveLength(reads);
  expect(state.current.deadline).toBe(deadline);
});

test('the deadline expires while hidden, removes private editors and preserves an authorized draft', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  await login(page);
  const element = await draft(page),
    activity = state.activity().length,
    reads = sessionReads(state).length;
  await visibility(page, 'hidden');
  await expire(page, state, element);
  await visibility(page, 'visible');
  expect(state.activity()).toHaveLength(activity);
  expect(sessionReads(state)).toHaveLength(reads);
  await expect(page.getByLabel('Correo electr\u00f3nico')).toBeFocused();
  await login(page);
  const editor = await openEditor(page);
  await expect(editor.getByLabel(classificationLabel, { exact: true })).toHaveValue(classification);
  await expect(editor.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue(pendingTag);
});

test('offline logout clears local access immediately and a late failure cannot revive its draft', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  await login(page);
  await expire(page, state, await draft(page));
  await login(page, false, false);
  const gate = await holdControl(page, logoutPath, state.current.token, true);
  const completed = requestCompletion(
    page,
    (request) => new URL(request.url()).pathname === logoutPath,
  );
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  await expect.poll(() => gate.calls.length).toBe(1);
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  await expect(page.getByLabel('Correo electr\u00f3nico')).toBeFocused();
  await expect(page.locator('.app-layout')).toHaveCount(0);
  await expect(page.locator('dialog')).toHaveCount(0);
  await login(page);
  await expect(page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true })).toBeEnabled();
  await blankEditor(page);
  const token = state.current.token;
  gate.release();
  expect((await completed).failed).toBe(true);
  await state.advance(1);
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toHaveCount(0);
  await expect(page.getByText('Saliste de esta pantalla.', { exact: false })).toHaveCount(0);
  const editor = page.getByRole('dialog', { name: editorName, exact: true });
  await expect(editor.getByLabel(classificationLabel, { exact: true })).toHaveValue('Civil');
  await expect(editor.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue('');
  expect(state.current.token).toBe(token);
});
