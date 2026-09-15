import { test, expect } from '@playwright/test';
import {
  caseId,
  otherCaseId,
  id,
  document,
  setup,
  login,
  navigate,
  selectCase,
} from './helpers.mjs';

async function waitForRendering(page) {
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
}

test('a document detail arriving after logout cannot enter the next session', async ({ page }) => {
  await setup(page, 'owner', []);
  await login(page);
  let release;
  await page.route(
    `**/cases/${caseId}/documents/${id}`,
    (route) =>
      new Promise((resolve) => {
        release = async () => {
          await route.fulfill({ json: { ...document, name: 'private-old.pdf' } });
          resolve();
        };
      }),
  );
  await page.getByLabel('Identificador del documento').fill(id);
  await page.getByRole('button', { name: 'Abrir documento' }).click();
  await expect.poll(() => typeof release).toBe('function');
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n' }).click();
  await login(page, false, false);
  const finished = page.waitForEvent('requestfinished', (request) =>
    request.url().endsWith(`/documents/${id}`),
  );
  await release();
  await finished;
  await waitForRendering(page);
  await expect(page.getByText('private-old.pdf')).toHaveCount(0);
  await expect(page.getByText('Aqu\u00ed empieza tu trabajo')).toBeVisible();
});

test('old case list cannot replace documents after another case is selected', async ({ page }) => {
  await setup(page);
  await login(page, false, false);
  let release;
  await page.route(
    `**/cases/${caseId}/documents?**`,
    (route) =>
      new Promise((resolve) => {
        release = async () => {
          await route.fulfill({
            json: { documents: [{ ...document, name: 'old-case.pdf' }], has_more: false },
          });
          resolve();
        };
      }),
  );
  await selectCase(page);
  await expect.poll(() => typeof release).toBe('function');
  await page.route(`**/cases/${otherCaseId}/documents?**`, (route) =>
    route.fulfill({
      json: {
        documents: [{ ...document, case_id: otherCaseId, name: 'other-case.pdf' }],
        has_more: false,
      },
    }),
  );
  await page.getByRole('button', { name: 'Cambiar expediente' }).click();
  await page.getByRole('button', { name: /Otro expediente/ }).click();
  await page.getByRole('link', { name: 'Documentos', exact: true }).click();
  await expect(page.getByText('other-case.pdf', { exact: true })).toBeVisible();
  const finished = page.waitForEvent('requestfinished', (request) =>
    request.url().includes(`/cases/${caseId}/documents?`),
  );
  await release();
  await finished;
  await waitForRendering(page);
  await expect(page.getByText('old-case.pdf')).toHaveCount(0);
  await expect(page.getByText('other-case.pdf', { exact: true })).toBeVisible();
});

test('late unfiltered page cannot override newer server search results', async ({ page }) => {
  await setup(page);
  await login(page);
  let release;
  await page.route(`**/cases/${caseId}/documents?**`, (route) => {
    if (new URL(route.request().url()).searchParams.has('name'))
      return route.fulfill({ json: { documents: [], has_more: false } });
    return new Promise((resolve) => {
      release = async () => {
        await route.fulfill({ json: { documents: [document], has_more: false } });
        resolve();
      };
    });
  });
  await page.getByRole('button', { name: 'Actualizar', exact: true }).click();
  await expect.poll(() => typeof release).toBe('function');
  await page.getByLabel('Buscar por nombre').fill('missing');
  await page.getByRole('button', { name: 'Buscar', exact: true }).click();
  await expect(page.getByText('No encontramos coincidencias')).toBeVisible();
  const finished = page.waitForEvent(
    'requestfinished',
    (request) =>
      !new URL(request.url()).searchParams.has('name') && request.url().includes('/documents?'),
  );
  await release();
  await finished;
  await waitForRendering(page);
  await expect(page.getByText('contrato.pdf', { exact: true })).toHaveCount(0);
});

test('revoked access clears document rows and open metadata on the next read', async ({ page }) => {
  await setup(page);
  await login(page);
  await page.getByRole('button', { name: /contrato.pdf/ }).click();
  await expect(page.getByRole('heading', { name: 'contrato.pdf' })).toBeVisible();
  await page.route(`**/cases/${caseId}/documents?**`, (route) =>
    route.fulfill({ status: 404, json: { error: { code: 'case_not_found' } } }),
  );
  await page.getByRole('button', { name: 'Actualizar', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('ya no tienes acceso');
  await expect(page.getByText('contrato.pdf', { exact: true })).toHaveCount(0);
});

test('returning to documents consumes the previous upload and filter intent', async ({ page }) => {
  await setup(page, 'owner', []);
  await login(page);
  await navigate(page, 'Inicio');
  await page.getByRole('button', { name: 'Subir documento', exact: true }).click();
  await page.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await page.goBack();
  await page.getByRole('button', { name: 'Abrir expediente', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Subir documento' })).not.toBeVisible();
  await page.goBack();
  await page.getByRole('button', { name: /Pendientes de sello/ }).click();
  await expect(page.getByLabel('Filtrar por estado')).toHaveValue('pending');
  await page.goBack();
  await page.getByRole('button', { name: 'Abrir expediente', exact: true }).click();
  await expect(page.getByLabel('Filtrar por estado')).toHaveValue('all');
});

test('search cancellation releases the open-document control after a delayed detail', async ({
  page,
}) => {
  await setup(page);
  await login(page);
  let release;
  await page.route(
    `**/cases/${caseId}/documents/${id}`,
    (route) =>
      new Promise((resolve) => {
        release = async () => {
          await route.fulfill({ json: document });
          resolve();
        };
      }),
  );
  await page.getByLabel('Identificador del documento').fill(id);
  await page.getByRole('button', { name: 'Abrir documento', exact: true }).click();
  await expect.poll(() => typeof release).toBe('function');
  await page.getByLabel('Buscar por nombre').fill('missing');
  await page.getByRole('button', { name: 'Buscar', exact: true }).click();
  await expect(page.getByText('No encontramos coincidencias')).toBeVisible();
  const finished = page.waitForEvent('requestfinished', (request) =>
    request.url().endsWith(`/documents/${id}`),
  );
  await release();
  await finished;
  await waitForRendering(page);
  await expect(page.getByRole('button', { name: 'Abrir documento', exact: true })).toBeEnabled();
});

test('new upload remains selected when an earlier document detail arrives later', async ({
  page,
}) => {
  await setup(page);
  await login(page);
  let release;
  await page.route(
    `**/cases/${caseId}/documents/${id}`,
    (route) =>
      new Promise((resolve) => {
        release = async () => {
          await route.fulfill({ json: { ...document, name: 'older-detail.pdf' } });
          resolve();
        };
      }),
  );
  await page.getByLabel('Identificador del documento').fill(id);
  await page.getByRole('button', { name: 'Abrir documento', exact: true }).click();
  await expect.poll(() => typeof release).toBe('function');
  await page.getByRole('button', { name: 'Subir documento', exact: true }).click();
  await page.getByLabel('Archivo', { exact: true }).setInputFiles({
    name: 'new-upload.pdf',
    mimeType: 'application/pdf',
    buffer: Buffer.from('new contents'),
  });
  await page.route(`**/cases/${caseId}/documents/with-metadata`, (route) =>
    route.fulfill({
      status: 201,
      json: { ...document, id: 'dddddddd-dddd-4ddd-8ddd-dddddddddddd', name: 'new-upload.pdf' },
    }),
  );
  await page.route(`**/documents/dddddddd-dddd-4ddd-8ddd-dddddddddddd/metadata`, (route) =>
    route.fulfill({
      json: {
        case_id: caseId,
        id: 'dddddddd-dddd-4ddd-8ddd-dddddddddddd',
        metadata_revision: 0,
        document_type: null,
        classification: null,
        tags: [],
      },
    }),
  );
  await page.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'new-upload.pdf' })).toBeVisible();
  const finished = page.waitForEvent('requestfinished', (request) =>
    request.url().endsWith(`/documents/${id}`),
  );
  await release();
  await finished;
  await waitForRendering(page);
  await expect(page.getByRole('heading', { name: 'new-upload.pdf' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'older-detail.pdf' })).toHaveCount(0);
});
