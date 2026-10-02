import { test, expect } from '@playwright/test';
import { login, openDocument } from './helpers.mjs';
import { requestCompletion } from './request-completion.mjs';
import {
  sessionSetup,
  checkSessionRequests,
  draft,
  expire,
  casePath,
  metadataPath,
  editorName,
  classification,
  classificationLabel,
  pendingTag,
} from './session-inactivity-helpers.mjs';
import { visibility } from './session-visibility-helpers.mjs';

function holdContext(state, path) {
  const gate = { path, entered: false };
  gate.promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.gate = gate;
  return gate;
}

test.afterEach(async ({ page }) => checkSessionRequests(page));

test('visibility interrupted draft recovery stays blocked until an explicit fresh context retry', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  await login(page);
  await expire(page, state, await draft(page));
  await login(page);
  await openDocument(page);
  const before = state.calls.length;
  const caseGate = holdContext(state, casePath);
  await page.getByRole('button', { name: editorName, exact: true }).click();
  const editor = page.getByRole('dialog', { name: editorName, exact: true });
  await expect(editor).toBeVisible();
  await expect.poll(() => caseGate.entered).toBe(true);
  await expect(editor.getByRole('button', { name: /^Guard/ })).toBeDisabled();
  await visibility(page, 'hidden');
  await expect(editor).toBeHidden();
  const caseCompleted = requestCompletion(
    page,
    (request) => new URL(request.url()).pathname === casePath,
  );
  caseGate.release();
  state.gate = null;
  expect((await caseCompleted).failed).toBe(false);
  await expect(
    page.locator('.metadata-dialog .dialog-actions button[type="button"]'),
  ).toBeEnabled();
  expect(state.calls.slice(before).filter((call) => call.path === metadataPath)).toEqual([]);
  const sessionCompleted = requestCompletion(
    page,
    (request) => new URL(request.url()).pathname === '/api/v1/auth/session',
  );
  await visibility(page, 'visible');
  const confirmed = await sessionCompleted;
  expect(confirmed.failed).toBe(false);
  expect((await confirmed.request.response()).status()).toBe(200);
  await expect(page.locator('.app-layout')).not.toHaveAttribute('inert', '');
  await expect(editor).toBeVisible();
  await expect(editor.getByRole('button', { name: /^Guardar/ })).toBeDisabled();
  await expect(editor.getByLabel(classificationLabel, { exact: true })).toBeDisabled();
  const retry = editor.getByRole('button', {
    name: 'Volver a consultar el contexto',
    exact: true,
  });
  await expect(retry).toBeEnabled();
  const retryStart = state.calls.length;
  const metadataGate = holdContext(state, metadataPath);
  await retry.click();
  await expect.poll(() => metadataGate.entered).toBe(true);
  const freshReads = state.calls
    .slice(retryStart)
    .filter((call) => [casePath, metadataPath].includes(call.path));
  expect(freshReads.map((call) => call.path)).toEqual([casePath, metadataPath]);
  for (const call of freshReads) {
    expect(call.method).toBe('GET');
    expect(call.headers.authorization).toBe(`Bearer ${state.current.token}`);
  }
  await expect(editor.getByRole('button', { name: /^Guard/ })).toBeDisabled();
  await expect(editor.getByLabel(classificationLabel, { exact: true })).toBeDisabled();
  metadataGate.release();
  state.gate = null;
  await expect(editor.getByLabel(classificationLabel, { exact: true })).toHaveValue(classification);
  await expect(editor.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue(pendingTag);
  await expect(editor.getByLabel(classificationLabel, { exact: true })).toBeEditable();
  await expect(
    editor.getByRole('button', { name: 'Guardar clasificaci\u00f3n', exact: true }),
  ).toBeEnabled();
  await expect(retry).toHaveCount(0);
});
