import { test, expect } from '@playwright/test';
import { setup, login, navigate } from './helpers.mjs';

test('classified upload preserves Unicode tags as individual values and waits for one multipart commit', async ({
  page,
}) => {
  const calls = await setup(page);
  await login(page);
  await navigate(page, 'Documentos');
  await page.getByRole('button', { name: 'Subir documento', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Subir documento' });
  await expect(modal.getByLabel('Tipo de documento (opcional)', { exact: true })).toBeVisible();
  await modal.getByLabel('Archivo', { exact: true }).setInputFiles({
    name: 'evidence.txt',
    mimeType: 'text/plain',
    buffer: Buffer.from('content'),
  });
  await modal.getByLabel('Tipo de documento (opcional)', { exact: true }).fill('Escrito');
  await modal.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true }).fill('Civil');
  await modal.getByLabel('Nueva etiqueta', { exact: true }).fill('acci\u00f3n, prueba');
  await modal.getByLabel('Nueva etiqueta', { exact: true }).press('Enter');
  await expect(
    modal.getByRole('button', { name: 'Quitar etiqueta: acci\u00f3n, prueba' }),
  ).toBeVisible();
  await expect(modal).toBeVisible();
  let release;
  let submitted;
  await page.route('**/documents/with-metadata', async (route) => {
    const form = await new Response(route.request().postDataBuffer(), {
      headers: { 'Content-Type': route.request().headers()['content-type'] },
    }).formData();
    submitted = JSON.parse(await form.get('metadata').text());
    await new Promise((resolve) => {
      release = resolve;
    });
    await route.fallback();
  });
  await modal.getByLabel('Nueva etiqueta', { exact: true }).fill('pending, tag');
  await modal.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect.poll(() => typeof release).toBe('function');
  await expect(modal).toBeVisible();
  expect(submitted).toEqual({
    document_type: 'Escrito',
    classification: 'Civil',
    tags: ['acci\u00f3n, prueba', 'pending, tag'],
  });
  release();
  await expect(page.getByRole('heading', { name: 'evidence.txt', exact: true })).toBeVisible();
  const card = page.getByRole('region', { name: 'Clasificaci\u00f3n actual del documento' });
  await expect(card.getByText('acci\u00f3n, prueba', { exact: true })).toBeVisible();
  expect(
    calls.filter((call) => call.method === 'POST' && call.path.endsWith('/with-metadata')),
  ).toHaveLength(1);
  expect(calls.filter((call) => call.method === 'PUT')).toHaveLength(0);
});

test('upload failures retain file and classification and never retry automatically', async ({
  page,
}) => {
  await setup(page);
  await login(page);
  await navigate(page, 'Documentos');
  await page.getByRole('button', { name: 'Subir documento', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Subir documento' });
  await modal.getByLabel('Archivo', { exact: true }).setInputFiles({
    name: 'retained.txt',
    mimeType: 'text/plain',
    buffer: Buffer.from('keep me'),
  });
  await modal
    .getByLabel('Clasificaci\u00f3n (opcional)', { exact: true })
    .fill('Retained classification');
  let submissions = 0;
  await page.route('**/documents/with-metadata', (route) => {
    submissions++;
    return route.abort();
  });
  await modal.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(modal.getByRole('alert')).toContainText('No se pudo confirmar el resultado');
  await expect(modal.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true })).toHaveValue(
    'Retained classification',
  );
  expect(
    await modal.getByLabel('Archivo', { exact: true }).evaluate((input) => input.files[0].name),
  ).toBe('retained.txt');
  expect(submissions).toBe(1);
  await expect(page.getByRole('heading', { name: 'retained.txt', exact: true })).toHaveCount(0);
});

test('invalid pending tags focus the tag input while preserving the file', async ({ page }) => {
  const calls = await setup(page);
  await login(page);
  await navigate(page, 'Documentos');
  await page.getByRole('button', { name: 'Subir documento', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Subir documento' });
  await modal.getByLabel('Archivo', { exact: true }).setInputFiles({
    name: 'retained.txt',
    mimeType: 'text/plain',
    buffer: Buffer.from('keep me'),
  });
  await modal.getByLabel('Nueva etiqueta', { exact: true }).fill('x'.repeat(41));
  await modal.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(modal.getByLabel('Nueva etiqueta', { exact: true })).toBeFocused();
  await expect(modal.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue('x'.repeat(41));
  expect(calls.filter((call) => call.path.endsWith('/with-metadata'))).toHaveLength(0);
});
