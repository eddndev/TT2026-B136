import { test, expect } from '@playwright/test';
import { setup, login, caseId, id, document } from './helpers.mjs';
import { versionSetup, append } from './version-helpers.mjs';

const deferred = () => {
  let resolve;
  const promise = new Promise((finish) => (resolve = finish));
  return { promise, resolve };
};

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
