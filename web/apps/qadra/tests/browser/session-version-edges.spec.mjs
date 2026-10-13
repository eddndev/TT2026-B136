import { test, expect } from '@playwright/test';
import { caseRecord, document, login } from './helpers.mjs';
import { administration } from './case-administration-helpers.mjs';
import { requestCompletion } from './request-completion.mjs';
import { expire, casePath } from './session-inactivity-helpers.mjs';
import {
  versionDraftSetup,
  checkVersionDraftRequests,
  openCurrentDocument,
  openAppend,
  prepareAppend,
  appendName,
  fileLabel,
  nameLabel,
  saveName,
  originalFile,
  rawName,
  fileContents,
  documentPath,
  versionsPath,
} from './session-version-drafts-helpers.mjs';

const flights = new WeakMap();

async function holdFirstAppend(page, state) {
  const flight = { calls: [], allowPost: false, release: () => {} };
  const pending = new Promise((resolve) => {
    flight.release = resolve;
  });
  flights.set(page, flight);
  await page.route(
    (url) => url.pathname === versionsPath,
    async (route) => {
      const request = route.request();
      if (request.method() !== 'POST') return route.fallback();
      const call = {
        path: versionsPath,
        method: request.method(),
        search: new URL(request.url()).search,
        headers: request.headers(),
        bytes: [...request.postDataBuffer()],
        expectedBearer: `Bearer ${state.current?.token}`,
      };
      state.calls.push(call);
      if (!flight.allowPost) {
        state.writes.push(call);
        return route.fulfill({ status: 501, json: { error: { code: 'unexpected_append' } } });
      }
      flight.allowPost = false;
      flight.calls.push(call);
      if (
        !state.current ||
        call.headers.authorization !== call.expectedBearer ||
        state.now >= Math.min(state.current.absolute, state.current.deadline)
      )
        return route.fulfill({ status: 401, json: { error: { code: 'invalid_session' } } });
      if (flight.calls.length === 1) {
        await pending;
        return route.abort('internetdisconnected');
      }
      const record = { ...document, version: 2, name: call.headers['x-document-name'] };
      state.records.push(record);
      return route.fulfill({ status: 201, json: record, headers: { 'Cache-Control': 'no-store' } });
    },
  );
  return flight;
}

test.afterEach(async ({ page }) => {
  const flight = flights.get(page);
  flight?.release();
  if (flight) {
    expect(flight.allowPost, 'An armed append must be submitted once').toBe(false);
    for (const call of flight.calls)
      expect(call).toMatchObject({
        method: 'POST',
        path: versionsPath,
        search: '?expected_version=1',
        bytes: [...Buffer.from(fileContents)],
        headers: {
          authorization: call.expectedBearer,
          'x-document-name': rawName.trim(),
          'content-type': 'application/octet-stream',
        },
      });
  }
  await checkVersionDraftRequests(page);
});

test('an in-flight append remains uncertain after MFA even when the document head is unchanged', async ({
  page,
}) => {
  const state = await versionDraftSetup(page);
  const flight = await holdFirstAppend(page, state);
  await login(page);
  await openCurrentDocument(page, state);
  const element = await prepareAppend(page, rawName.trim());
  const initial = page.getByRole('dialog', { name: appendName, exact: true });
  const firstCompleted = requestCompletion(
    page,
    (request) => request.method() === 'POST' && new URL(request.url()).pathname === versionsPath,
  );
  flight.allowPost = true;
  await initial.getByRole('button', { name: saveName, exact: true }).click();
  await expect.poll(() => flight.calls.length).toBe(1);
  const firstBearer = flight.calls[0].headers.authorization;
  await expire(page, state, element);
  await login(page, true);
  await openCurrentDocument(page, state);
  const before = state.calls.length;
  const modal = await openAppend(page);
  await expect(modal.getByText(originalFile, { exact: true })).toBeVisible();
  await expect(modal.getByLabel(nameLabel, { exact: true })).toHaveValue(rawName.trim());
  await expect(modal.getByText('Versi\u00f3n de partida: 1', { exact: true })).toBeVisible();
  await expect(
    modal.getByRole('status').filter({ hasText: 'No se pudo confirmar la versi\u00f3n enviada.' }),
  ).toBeVisible();
  const comparison = modal.getByRole('region', { name: 'Versi\u00f3n actual guardada' });
  await expect(comparison).toContainText('Versi\u00f3n 1');
  await expect(comparison).toContainText(document.name);
  const decision = modal.getByRole('checkbox', { name: /He comparado la versi\u00f3n actual/ });
  const save = modal.getByRole('button', { name: saveName, exact: true });
  await expect(decision).not.toBeChecked();
  await expect(save).toBeDisabled();
  for (const path of [casePath, documentPath])
    expect(
      state.calls
        .slice(before)
        .some(
          (call) =>
            call.path === path &&
            call.method === 'GET' &&
            call.headers.authorization === `Bearer ${state.current.token}`,
        ),
    ).toBe(true);
  expect(state.records.map((record) => record.version)).toEqual([1]);
  expect(flight.calls).toHaveLength(1);
  flight.release();
  expect((await firstCompleted).failed).toBe(true);
  await expect(save).toBeDisabled();
  await expect(decision).not.toBeChecked();
  expect(flight.calls).toHaveLength(1);
  await decision.check();
  await expect(save).toBeEnabled();
  expect(flight.calls).toHaveLength(1);
  flight.allowPost = true;
  await save.click();
  await expect(modal).toBeHidden();
  expect(flight.calls).toHaveLength(2);
  expect(flight.calls[1].headers.authorization).toBe(`Bearer ${state.current.token}`);
  expect(flight.calls[1].headers.authorization).not.toBe(firstBearer);
  expect(state.records.map((record) => record.version)).toEqual([1, 2]);
});

test('fresh case closure restores the version file and name for reading while preventing every append', async ({
  page,
}) => {
  const state = await versionDraftSetup(page);
  await login(page);
  await openCurrentDocument(page, state);
  await expire(page, state, await prepareAppend(page));
  await login(page);
  await openCurrentDocument(page, state);
  const caseReads = [];
  await page.route(
    (url) => url.pathname === casePath,
    (route) => {
      const request = route.request();
      const call = {
        method: request.method(),
        body: request.postData(),
        search: new URL(request.url()).search,
        headers: request.headers(),
      };
      caseReads.push(call);
      if (call.method !== 'GET' || call.body !== null || call.search !== '') {
        state.unexpected.push(call);
        return route.fulfill({ status: 501, json: { error: { code: 'unexpected_case_read' } } });
      }
      if (call.headers.authorization !== `Bearer ${state.current.token}`)
        return route.fulfill({ status: 401, json: { error: { code: 'invalid_session' } } });
      return route.fulfill({
        json: administration(caseRecord, 2, null, 'closed'),
        headers: { 'Cache-Control': 'no-store' },
      });
    },
  );
  const before = state.calls.length;
  const modal = await openAppend(page);
  await expect(modal.getByText(originalFile, { exact: true })).toBeVisible();
  await expect(modal.getByLabel(nameLabel, { exact: true })).toHaveValue(rawName);
  await expect(modal.getByLabel(nameLabel, { exact: true })).toBeDisabled();
  await expect(modal.getByLabel(fileLabel, { exact: true })).toBeDisabled();
  await expect(modal.getByRole('button', { name: saveName, exact: true })).toBeDisabled();
  await expect(
    modal.getByRole('status').filter({ hasText: 'El expediente est\u00e1 cerrado.' }),
  ).toBeVisible();
  await expect(modal.getByText('Versi\u00f3n de partida: 1', { exact: true })).toBeVisible();
  expect(caseReads).toHaveLength(1);
  expect(caseReads[0].headers.authorization).toBe(`Bearer ${state.current.token}`);
  expect(
    state.calls.slice(before).some((call) => call.path === documentPath && call.method === 'GET'),
  ).toBe(true);
  expect(state.submissions).toEqual([]);
});
