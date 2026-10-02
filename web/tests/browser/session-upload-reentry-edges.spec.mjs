import { test, expect } from '@playwright/test';
import { caseRecord, login } from './helpers.mjs';
import { administration } from './case-administration-helpers.mjs';
import { requestCompletion } from './request-completion.mjs';
import {
  sessionSetup,
  checkSessionRequests,
  expire,
  casePath,
  documentsPath,
} from './session-inactivity-helpers.mjs';

const originalName = 'unconfirmed original.txt';
const editedName = 'unconfirmed-evidence.txt';
const contents = 'Exact draft bytes\nretained across MFA';
const classification = '  classification in progress  ';
const pendingTag = '  upload tag  ';
const uploads = new WeakMap();
const caseReads = new WeakMap();

async function openUpload(page) {
  await page.getByRole('button', { name: 'Subir documento', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Subir documento', exact: true });
  await expect(modal).toBeVisible();
  return modal;
}

async function prepareUpload(page) {
  const modal = await openUpload(page);
  await modal.getByLabel('Archivo', { exact: true }).setInputFiles({
    name: originalName,
    mimeType: 'text/plain',
    buffer: Buffer.from(contents),
  });
  await modal.getByLabel('Nombre del documento', { exact: true }).fill(editedName);
  await modal.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true }).fill(classification);
  await modal.getByLabel('Nueva etiqueta', { exact: true }).fill(pendingTag);
  return modal;
}

async function captureUploads(page, state, holdFirst = false) {
  const record = { calls: [], release: () => {} };
  const pending = new Promise((resolve) => {
    record.release = resolve;
  });
  uploads.set(page, record);
  await page.route(
    (url) => url.pathname === `${documentsPath}/with-metadata`,
    async (route) => {
      const request = route.request();
      const expectedBearer = `Bearer ${state.current?.token}`;
      const form = await new Response(request.postDataBuffer(), {
        headers: { 'Content-Type': request.headers()['content-type'] },
      }).formData();
      const file = form.get('file');
      record.calls.push({
        method: request.method(),
        search: new URL(request.url()).search,
        keys: [...form.keys()].sort(),
        expectedBearer,
        bearer: request.headers().authorization,
        name: file.name,
        documentName: request.headers()['x-document-name'],
        type: file.type,
        text: await file.text(),
        metadata: JSON.parse(await form.get('metadata').text()),
      });
      if (holdFirst && record.calls.length === 1) {
        await pending;
        return route.abort('internetdisconnected');
      }
      return route.fulfill({ status: 503, json: { error: { code: 'unavailable' } } });
    },
  );
  return record;
}

function holdListing(state) {
  const gate = { path: documentsPath, entered: false };
  gate.promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.gate = gate;
  return gate;
}

test.afterEach(async ({ page }) => {
  const record = uploads.get(page);
  record?.release();
  for (const call of record?.calls || []) {
    expect(call.method).toBe('POST');
    expect(call.search).toBe('');
    expect(call.keys).toEqual(['file', 'metadata']);
    expect(call.bearer).toBe(call.expectedBearer);
    expect(call).toMatchObject({
      name: originalName,
      documentName: editedName.trim(),
      type: 'text/plain',
      text: contents,
      metadata: { classification: classification.trim(), tags: [pendingTag.trim()] },
    });
  }
  for (const call of caseReads.get(page) || []) {
    expect(call.method).toBe('GET');
    expect(call.body).toBeNull();
    expect(call.search).toBe('');
    expect(call.bearer).toBe(call.expectedBearer);
  }
  await checkSessionRequests(page);
});

test('an interrupted upload requires a fresh listing and an explicit duplicate-risk decision before another post', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  const record = await captureUploads(page, state, true);
  await login(page);
  const initial = await prepareUpload(page);
  const element = await initial.elementHandle();
  const firstCompleted = requestCompletion(
    page,
    (request) => new URL(request.url()).pathname === `${documentsPath}/with-metadata`,
  );
  await initial.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect.poll(() => record.calls.length).toBe(1);
  const originalBearer = record.calls[0].bearer;
  await expire(page, state, element);
  await login(page);
  const modal = await openUpload(page);
  await expect(modal.getByText(originalName, { exact: true })).toBeVisible();
  await expect(modal.getByLabel('Nombre del documento', { exact: true })).toHaveValue(editedName);
  await expect(modal.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true })).toHaveValue(
    classification,
  );
  await expect(
    modal.getByRole('button', { name: `Quitar etiqueta: ${pendingTag.trim()}` }),
  ).toBeVisible();
  const submit = modal.getByRole('button', { name: 'Cargar documento', exact: true });
  await expect(submit).toBeDisabled();
  await expect(
    modal.getByRole('status').filter({ hasText: 'No se pudo confirmar la carga anterior.' }),
  ).toBeVisible();
  expect(record.calls).toHaveLength(1);
  record.release();
  expect((await firstCompleted).failed).toBe(true);
  await expect(submit).toBeDisabled();
  const before = state.calls.length;
  const listing = holdListing(state);
  await modal.getByRole('button', { name: 'Consultar documentos actuales', exact: true }).click();
  await expect.poll(() => listing.entered).toBe(true);
  await expect(
    modal.getByRole('button', { name: 'Cargando documento...', exact: true }),
  ).toBeDisabled();
  expect(record.calls).toHaveLength(1);
  listing.release();
  state.gate = null;
  const decision = modal.getByRole('checkbox', {
    name: 'He revisado el listado y decido iniciar otra carga; puede duplicar la anterior.',
    exact: true,
  });
  await expect(decision).toBeEnabled();
  await expect(decision).not.toBeChecked();
  await expect(modal.getByText('contrato.pdf', { exact: true })).toBeVisible();
  await expect(submit).toBeDisabled();
  const reads = state.calls.slice(before).filter((call) => call.path === documentsPath);
  expect(reads).toHaveLength(1);
  expect(reads[0]).toMatchObject({
    method: 'GET',
    headers: { authorization: `Bearer ${state.current.token}` },
  });
  expect(record.calls).toHaveLength(1);
  await decision.check();
  await expect(submit).toBeEnabled();
  await submit.click();
  await expect.poll(() => record.calls.length).toBe(2);
  expect(record.calls[1].bearer).toBe(`Bearer ${state.current.token}`);
  expect(record.calls[1].bearer).not.toBe(originalBearer);
});

test('fresh case closure permits reading the recovered upload but never loading its file', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  const record = await captureUploads(page, state);
  await login(page);
  await expire(page, state, await (await prepareUpload(page)).elementHandle());
  await login(page);
  const reads = [];
  caseReads.set(page, reads);
  await page.route(
    (url) => url.pathname === casePath,
    (route) => {
      const request = route.request();
      reads.push({
        method: request.method(),
        body: request.postData(),
        search: new URL(request.url()).search,
        bearer: request.headers().authorization,
        expectedBearer: `Bearer ${state.current.token}`,
      });
      return route.fulfill({ json: administration(caseRecord, 2, null, 'closed') });
    },
  );
  const modal = await openUpload(page);
  await expect(modal.getByText(originalName, { exact: true })).toBeVisible();
  await expect(modal.getByLabel('Nombre del documento', { exact: true })).toHaveValue(editedName);
  await expect(modal.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue(pendingTag);
  await expect(modal.getByLabel('Nombre del documento', { exact: true })).toBeDisabled();
  await expect(modal.getByLabel('Archivo', { exact: true })).toBeDisabled();
  await expect(modal.getByRole('button', { name: 'Cargar documento', exact: true })).toBeDisabled();
  await expect(modal.getByText(/El expediente est\u00e1 cerrado/)).toBeVisible();
  expect(reads).toHaveLength(1);
  expect(record.calls).toEqual([]);
});

test('denied fresh case authorization discards the selected upload before access returns', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  const record = await captureUploads(page, state);
  await login(page);
  await expire(page, state, await (await prepareUpload(page)).elementHandle());
  await login(page);
  const before = state.calls.length;
  state.allowed = false;
  const modal = await openUpload(page);
  await expect(modal.getByRole('alert')).toContainText('ya no tienes acceso');
  await expect(modal.getByRole('button', { name: 'Cargar documento', exact: true })).toBeDisabled();
  await expect(modal.getByText(originalName, { exact: true })).toHaveCount(0);
  expect(state.calls.slice(before).filter((call) => call.path === casePath)).toHaveLength(1);
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
  state.allowed = true;
  const reopened = await openUpload(page);
  await expect(reopened.getByLabel('Nombre del documento', { exact: true })).toHaveValue('');
  await expect(reopened.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue('');
  await expect(reopened.getByText(originalName, { exact: true })).toHaveCount(0);
  expect(record.calls).toEqual([]);
});
