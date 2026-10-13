import { expect } from '@playwright/test';
import { document, id } from './helpers.mjs';
import {
  sessionSetup,
  checkSessionRequests,
  documentsPath,
} from './session-inactivity-helpers.mjs';

export const documentPath = `${documentsPath}/${id}`;
export const versionsPath = `${documentPath}/versions`;
export const appendName = 'Agregar versi\u00f3n';
export const fileLabel = 'Archivo de la nueva versi\u00f3n';
export const nameLabel = 'Nombre de la nueva versi\u00f3n';
export const saveName = 'Guardar nueva versi\u00f3n';
export const originalFile = 'source evidence.txt';
export const rawName = '  pending-version.txt  ';
export const fileContents = 'Unsent version bytes\nsecond line\n';
const states = new WeakMap();

export async function versionDraftSetup(page) {
  const state = await sessionSetup(page);
  state.records = [{ ...document, sealed: true }];
  state.submissions = [];
  state.allowAppend = false;
  states.set(page, state);
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    const exact = url.pathname.startsWith(`${versionsPath}/`)
      ? url.pathname.slice(versionsPath.length + 1)
      : null;
    if (
      ![documentsPath, documentPath, versionsPath].includes(url.pathname) &&
      !/^[1-9][0-9]*$/.test(exact ?? '')
    )
      return route.fallback();
    const call = {
      path: url.pathname,
      method: request.method(),
      body: request.postData(),
      headers: request.headers(),
      search: url.search,
      at: state.now,
    };
    state.calls.push(call);
    const reply = (json, status = 200) =>
      route.fulfill({
        status,
        json,
        headers: { 'Cache-Control': 'no-store' },
      });
    if (
      !state.current ||
      call.headers.authorization !== `Bearer ${state.current.token}` ||
      state.now >= Math.min(state.current.absolute, state.current.deadline)
    )
      return reply({ error: { code: 'invalid_session' } }, 401);
    if (!state.allowed) return reply({ error: { code: 'case_not_found' } }, 404);
    if (call.method === 'GET' && call.body === null) {
      if (state.gate?.path === call.path) {
        state.gate.entered = true;
        await state.gate.promise;
      }
      if (call.path === documentsPath)
        return reply({ documents: [state.records.at(-1)], has_more: false });
      if (call.path === documentPath) return reply(state.records.at(-1));
      if (call.path === versionsPath) {
        const before = Number(url.searchParams.get('before_version')) || Infinity;
        return reply({
          versions: [...state.records].reverse().filter((record) => record.version < before),
          has_more: false,
          next_before_version: null,
          first_available_version: 1,
        });
      }
      const record = state.records.find((entry) => entry.version === Number(exact));
      return record ? reply(record) : reply({ error: { code: 'document_not_found' } }, 404);
    }
    if (call.path === versionsPath && call.method === 'POST' && state.allowAppend) {
      state.allowAppend = false;
      state.submissions.push({ ...call, bytes: [...request.postDataBuffer()] });
      const expected = Number(url.searchParams.get('expected_version'));
      if (expected !== state.records.at(-1).version)
        return reply({ error: { code: 'document_version_conflict' } }, 409);
      const record = { ...document, version: expected + 1, name: call.headers['x-document-name'] };
      state.records.push(record);
      return reply(record, 201);
    }
    state.writes.push(call);
    return reply({ error: { code: 'unexpected_version_draft_request' } }, 501);
  });
  return state;
}

export async function checkVersionDraftRequests(page) {
  await checkSessionRequests(page);
  const state = states.get(page);
  if (state) expect(state.allowAppend, 'An armed append must be submitted once').toBe(false);
}

export async function openCurrentDocument(page, state) {
  await page.locator('.reference-panel').getByLabel('Identificador del documento').fill(id);
  await page.getByRole('button', { name: 'Abrir documento', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: state.records.at(-1).name, exact: true }),
  ).toBeVisible();
  await expect(page.getByRole('button', { name: appendName, exact: true })).toBeEnabled();
}

export async function openAppend(page) {
  await page.getByRole('button', { name: appendName, exact: true }).click();
  const modal = page.getByRole('dialog', { name: appendName, exact: true });
  await expect(modal).toBeVisible();
  return modal;
}

export async function prepareAppend(page, name = rawName) {
  const modal = await openAppend(page);
  await modal.getByLabel(fileLabel, { exact: true }).setInputFiles({
    name: originalFile,
    mimeType: 'text/plain',
    buffer: Buffer.from(fileContents),
  });
  await modal.getByLabel(nameLabel, { exact: true }).fill(name);
  return modal.elementHandle();
}

export async function expectBlankAppend(modal) {
  await expect(modal.getByLabel(nameLabel, { exact: true })).toHaveValue('');
  await expect(modal.getByText(originalFile, { exact: true })).toHaveCount(0);
}

export function expectSubmission(state, expectedVersion, name = rawName.trim()) {
  expect(state.submissions).toHaveLength(1);
  expect(state.submissions[0]).toMatchObject({
    method: 'POST',
    path: versionsPath,
    search: `?expected_version=${expectedVersion}`,
    bytes: [...Buffer.from(fileContents)],
    headers: {
      authorization: `Bearer ${state.current.token}`,
      'x-document-name': name,
      'content-type': 'application/octet-stream',
    },
  });
}
