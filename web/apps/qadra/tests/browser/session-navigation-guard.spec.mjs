import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import {
  sessionSetup,
  draft,
  editorName,
  classification,
  classificationLabel,
  pendingTag,
} from './session-inactivity-helpers.mjs';
import {
  visibility,
  holdControl,
  finishVisibilityRequests,
} from './session-visibility-helpers.mjs';

test.afterEach(async ({ page }) => finishVisibilityRequests(page));

test('a hash change cannot discard an open draft while the returning session is unconfirmed', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  await login(page);
  const element = await draft(page);
  const editor = page.getByRole('dialog', { name: editorName, exact: true });
  const gate = await holdControl(page, '/api/v1/auth/session', state.current.token);
  const before = state.calls.length;
  await visibility(page, 'hidden');
  await expect(editor).toBeHidden();
  await visibility(page, 'visible');
  await expect.poll(() => gate.calls.length).toBe(1);
  await expect(page.locator('.app-layout')).toHaveAttribute('inert', '');
  await page.evaluate(
    () =>
      new Promise((resolve) => {
        window.addEventListener('hashchange', () => resolve(), { once: true });
        location.hash = '#overview';
      }),
  );
  expect(await element.evaluate((node) => node.isConnected)).toBe(true);
  await expect(page.locator('.app-layout')).toHaveAttribute('inert', '');
  await expect(
    page.getByRole('heading', { name: 'Tu mesa de trabajo', includeHidden: true }),
  ).toHaveCount(0);
  expect(
    state.calls.slice(before).filter((call) => !call.path.startsWith('/api/v1/auth/')),
  ).toEqual([]);
  expect(state.writes).toEqual([]);
  gate.release();
  await expect(editor).toBeVisible();
  expect(await element.evaluate((node) => node.isConnected)).toBe(true);
  await expect(editor.getByLabel(classificationLabel, { exact: true })).toHaveValue(classification);
  await expect(editor.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue(pendingTag);
  expect(state.writes).toEqual([]);
});
