import { test, expect } from '@playwright/test';
import { setup, login, caseId, id, document } from './helpers.mjs';
import { versionSetup, append } from './version-helpers.mjs';

const deferred = () => {
  let resolve;
  const promise = new Promise((finish) => (resolve = finish));
  return { promise, resolve };
};

test('upload finishes the list refresh before starting the metadata and version readers', async ({
  page,
}) => {
  await setup(page, 'owner', []);
  await login(page);
  const started = deferred(),
    release = deferred();
  let metadataReads = 0,
    historyReads = 0;
  await page.route(`**/cases/${caseId}/documents?*`, async (route) => {
    started.resolve();
    await release.promise;
    return route.fulfill({ json: { documents: [document], has_more: false } });
  });
  await page.route(`**/documents/${id}/metadata`, (route) => {
    metadataReads++;
    return route.fallback();
  });
  await page.route(`**/documents/${id}/versions?*`, (route) => {
    historyReads++;
    return route.fallback();
  });
  await page.getByRole('button', { name: 'Subir documento', exact: true }).click();
  const modal = page.getByRole('dialog', { name: 'Subir documento', exact: true });
  await modal.getByLabel('Archivo', { exact: true }).setInputFiles({
    name: 'uploaded.txt',
    mimeType: 'text/plain',
    buffer: Buffer.from('upload'),
  });
  await modal.getByRole('button', { name: 'Cargar documento', exact: true }).click();
  await started.promise;
  await expect(modal).not.toBeVisible();
  try {
    await expect(page.locator('.document-metadata')).toHaveCount(0);
    expect(metadataReads).toBe(0);
    expect(historyReads).toBe(0);
  } finally {
    release.resolve();
  }
  await expect(page.getByRole('heading', { name: 'uploaded.txt', exact: true })).toBeVisible();
  await expect(page.locator('.document-metadata')).toHaveAttribute('aria-busy', 'false');
  await expect(
    page.getByRole('button', { name: 'Actualizar historial', exact: true }),
  ).toBeEnabled();
  expect(metadataReads).toBe(1);
  expect(historyReads).toBe(1);
});

test('a new version waits for both automatic refreshes before offering sealing', async ({
  page,
}) => {
  await versionSetup(page);
  const listStarted = deferred(),
    historyStarted = deferred(),
    release = deferred();
  const current = { ...document, version: 2, name: 'version-two.txt' };
  await page.route(`**/cases/${caseId}/documents?*`, async (route) => {
    listStarted.resolve();
    await release.promise;
    return route.fulfill({ json: { documents: [current], has_more: false } });
  });
  await page.route(`**/documents/${id}/versions?limit=*`, async (route) => {
    historyStarted.resolve();
    await release.promise;
    return route.fulfill({
      json: {
        versions: [current, { ...document, sealed: true }],
        has_more: false,
        next_before_version: null,
        first_available_version: 1,
      },
    });
  });
  await append(page);
  await Promise.all([listStarted.promise, historyStarted.promise]);
  const seal = page.getByRole('button', { name: 'Sellar documento', exact: true });
  try {
    await expect(seal).toBeDisabled();
  } finally {
    release.resolve();
  }
  await expect(seal).toBeEnabled();
});

test('initial metadata and version readers keep document actions unavailable', async ({ page }) => {
  await setup(page);
  await login(page);
  const metadataStarted = deferred(),
    historyStarted = deferred();
  const releaseMetadata = deferred(),
    releaseHistory = deferred();
  await page.route(`**/documents/${id}/metadata`, async (route) => {
    metadataStarted.resolve();
    await releaseMetadata.promise;
    return route.fallback();
  });
  await page.route(`**/documents/${id}/versions?*`, async (route) => {
    historyStarted.resolve();
    await releaseHistory.promise;
    return route.fallback();
  });
  await page.getByRole('button', { name: 'Abrir contrato.pdf', exact: true }).click();
  await Promise.all([metadataStarted.promise, historyStarted.promise]);
  const seal = page.getByRole('button', { name: 'Sellar documento', exact: true });
  const historyFinished = page.waitForEvent('requestfinished', (request) =>
    request.url().includes('/versions?'),
  );
  releaseHistory.resolve();
  await historyFinished;
  await expect(page.locator('.version-row')).toHaveCount(1);
  try {
    await expect(seal).toBeDisabled();
    await expect(
      page.getByRole('button', { name: 'Editar clasificaci\u00f3n', exact: true }),
    ).toBeDisabled();
  } finally {
    releaseMetadata.resolve();
  }
  await expect(seal).toBeEnabled();
});
