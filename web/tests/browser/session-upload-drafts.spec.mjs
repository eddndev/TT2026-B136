import { test, expect } from '@playwright/test';
import { login, selectCase } from './helpers.mjs';
import {
  sessionSetup,
  checkSessionRequests,
  expire,
  signInOther,
} from './session-inactivity-helpers.mjs';

const originalName = 'original evidence.txt';
const editedName = '  edited-evidence.txt  ';
const contents = 'Unfinished evidence bytes\nsecond line';
const classification = '  still editing  ';
const pendingTag = '  pending upload tag  ';
async function openUpload(page) {
  await page.getByRole('button', { name: 'Subir documento', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Subir documento', exact: true });
  await expect(modal).toBeVisible();
  return modal;
}
async function prepare(page) {
  const modal = await openUpload(page);
  await modal.getByLabel('Archivo', { exact: true }).setInputFiles({
    name: originalName,
    mimeType: 'text/plain',
    buffer: Buffer.from(contents),
  });
  await modal.getByLabel('Nombre del documento', { exact: true }).fill(editedName);
  await modal.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true }).fill(classification);
  await modal.getByLabel('Nueva etiqueta', { exact: true }).fill(pendingTag);
  return modal.elementHandle();
}
async function blank(modal) {
  await expect(modal.getByLabel('Nombre del documento', { exact: true })).toHaveValue('');
  await expect(modal.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue('');
  await expect(modal.getByText(originalName, { exact: true })).toHaveCount(0);
}

test.afterEach(async ({ page }) => checkSessionRequests(page));

test('same account restores an unsent root upload and sends exact file only after confirmation', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  const submissions = [];
  await page.route('**/documents/with-metadata', async (route) => {
    const request = route.request();
    const form = await new Response(request.postDataBuffer(), {
      headers: { 'Content-Type': request.headers()['content-type'] },
    }).formData();
    const file = form.get('file');
    submissions.push({
      name: file.name,
      documentName: request.headers()['x-document-name'],
      type: file.type,
      text: await file.text(),
      metadata: JSON.parse(await form.get('metadata').text()),
      bearer: request.headers().authorization,
    });
    await route.fulfill({ status: 503, json: { error: { code: 'unavailable' } } });
  });
  await login(page);
  await expire(page, state, await prepare(page));
  await login(page);
  expect(submissions).toEqual([]);
  const modal = await openUpload(page);
  await expect(modal.getByText(originalName, { exact: true })).toBeVisible();
  await expect(modal.getByLabel('Nombre del documento', { exact: true })).toHaveValue(editedName);
  await expect(modal.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true })).toHaveValue(
    classification,
  );
  await expect(modal.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue(pendingTag);
  expect(submissions).toEqual([]);
  await modal.getByLabel('Nombre del documento', { exact: true }).fill(editedName.trim());
  await modal.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect.poll(() => submissions.length).toBe(1);
  expect(submissions[0]).toMatchObject({
    name: originalName,
    documentName: editedName.trim(),
    type: 'text/plain',
    text: contents,
    metadata: { classification: classification.trim(), tags: [pendingTag.trim()] },
    bearer: `Bearer ${state.current.token}`,
  });
});

test('another account cannot recover a selected upload or revive it in the original account', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  await login(page);
  await expire(page, state, await prepare(page));
  await signInOther(page);
  await selectCase(page);
  const modal = await openUpload(page);
  await blank(modal);
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  await login(page);
  await blank(await openUpload(page));
});

test('explicitly cancelling a recovered upload discards its file before another expiry', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  await login(page);
  await expire(page, state, await prepare(page));
  await login(page);
  const modal = await openUpload(page);
  await expect(modal.getByText(originalName, { exact: true })).toBeVisible();
  await modal.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await state.advance(state.current.deadline - state.now + 1);
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  await login(page);
  await blank(await openUpload(page));
});
