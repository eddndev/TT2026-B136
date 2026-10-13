import { test, expect } from '@playwright/test';
import { document, login, selectCase } from './helpers.mjs';
import { expire, signInOther, casePath } from './session-inactivity-helpers.mjs';
import {
  versionDraftSetup,
  checkVersionDraftRequests,
  openCurrentDocument,
  openAppend,
  prepareAppend,
  expectBlankAppend,
  expectSubmission,
  documentPath,
  appendName,
  nameLabel,
  saveName,
  originalFile,
  rawName,
} from './session-version-drafts-helpers.mjs';

test.afterEach(async ({ page }) => checkVersionDraftRequests(page));

test('same-account MFA restores the selected version file only after fresh case and document reads', async ({
  page,
}) => {
  const state = await versionDraftSetup(page);
  await login(page);
  await openCurrentDocument(page, state);
  await expire(page, state, await prepareAppend(page));
  await login(page, true);
  expect(state.grants.at(-1).factor).toBe('recovery');
  await openCurrentDocument(page, state);
  await expect(page.getByRole('dialog', { name: appendName })).toBeHidden();
  const before = state.calls.length;
  const gate = { path: documentPath, entered: false };
  gate.promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.gate = gate;
  const modal = await openAppend(page);
  await expect.poll(() => gate.entered).toBe(true);
  await expect(modal.getByText(originalFile, { exact: true })).toHaveCount(0);
  await expect(modal.getByRole('button', { name: saveName, exact: true })).toBeDisabled();
  gate.release();
  state.gate = null;
  await expect(modal.getByText(originalFile, { exact: true })).toBeVisible();
  await expect(modal.getByLabel(nameLabel, { exact: true })).toHaveValue(rawName);
  await expect(modal.getByText('Versi\u00f3n de partida: 1', { exact: true })).toBeVisible();
  const freshReads = state.calls.slice(before).filter((call) => call.method === 'GET');
  const caseIndex = freshReads.findIndex((call) => call.path === casePath);
  const documentIndex = freshReads.findIndex((call) => call.path === documentPath);
  expect(caseIndex).toBeGreaterThanOrEqual(0);
  expect(documentIndex).toBeGreaterThan(caseIndex);
  for (const call of [freshReads[caseIndex], freshReads[documentIndex]])
    expect(call.headers.authorization).toBe(`Bearer ${state.current.token}`);
  expect(state.submissions).toEqual([]);
  const storage = await page.evaluate(() =>
    JSON.stringify([Object.entries(localStorage), Object.entries(sessionStorage)]),
  );
  expect(storage).not.toContain(rawName);
  expect(storage).not.toContain(originalFile);
  await modal.getByLabel(nameLabel, { exact: true }).fill(rawName.trim());
  state.allowAppend = true;
  await modal.getByRole('button', { name: saveName, exact: true }).click();
  await expect(modal).toBeHidden();
  expectSubmission(state, 1);
});

test('a changed head keeps the original version base until explicit comparison authorizes append', async ({
  page,
}) => {
  const state = await versionDraftSetup(page);
  await login(page);
  await openCurrentDocument(page, state);
  await expire(page, state, await prepareAppend(page, rawName.trim()));
  state.records.push({ ...document, version: 2, name: 'concurrent-version.txt' });
  await login(page);
  await openCurrentDocument(page, state);
  const before = state.calls.length;
  const modal = await openAppend(page);
  await expect(modal.getByText(originalFile, { exact: true })).toBeVisible();
  await expect(modal.getByLabel(nameLabel, { exact: true })).toHaveValue(rawName.trim());
  await expect(modal.getByText('Versi\u00f3n de partida: 1', { exact: true })).toBeVisible();
  const comparison = modal.getByRole('region', { name: 'Versi\u00f3n actual guardada' });
  await expect(comparison).toContainText('Versi\u00f3n 2');
  await expect(comparison).toContainText('concurrent-version.txt');
  const save = modal.getByRole('button', { name: saveName, exact: true });
  await expect(save).toBeDisabled();
  expect(state.submissions).toEqual([]);
  expect(
    state.calls
      .slice(before)
      .some(
        (call) =>
          call.path === documentPath &&
          call.method === 'GET' &&
          call.headers.authorization === `Bearer ${state.current.token}`,
      ),
  ).toBe(true);
  const decision = modal.getByRole('checkbox', { name: /He comparado la versi\u00f3n actual/ });
  await expect(decision).not.toBeChecked();
  await decision.check();
  expect(state.submissions).toEqual([]);
  state.allowAppend = true;
  await save.click();
  await expect(modal).toBeHidden();
  expectSubmission(state, 2);
  expect(state.records.map((record) => record.version)).toEqual([1, 2, 3]);
});

test('cancelling and changing account discard the recovered version file across later sessions', async ({
  page,
}) => {
  const state = await versionDraftSetup(page);
  await login(page);
  await openCurrentDocument(page, state);
  await expire(page, state, await prepareAppend(page));
  await login(page);
  await openCurrentDocument(page, state);
  let modal = await openAppend(page);
  await expect(modal.getByText(originalFile, { exact: true })).toBeVisible();
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await state.advance(state.current.deadline - state.now + 1);
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  await login(page);
  await openCurrentDocument(page, state);
  modal = await openAppend(page);
  await expectBlankAppend(modal);
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await expire(page, state, await prepareAppend(page));
  await signInOther(page);
  await selectCase(page);
  await openCurrentDocument(page, state);
  modal = await openAppend(page);
  await expectBlankAppend(modal);
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  await login(page);
  await openCurrentDocument(page, state);
  await expectBlankAppend(await openAppend(page));
  expect(state.submissions).toEqual([]);
});
