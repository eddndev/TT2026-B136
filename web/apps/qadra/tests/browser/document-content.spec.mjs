import { test, expect } from '@playwright/test';
import { requestCompletion } from './request-completion.mjs';
import { selectCase, signOut } from './helpers.mjs';
import {
  contentSetup,
  contentResponse,
  contentPath,
  contentBytes,
  downloadBytes,
} from './document-content-helpers.mjs';

const downloadButton = (page) =>
  page.getByRole('button', { name: 'Descargar archivo', exact: true });

for (const width of [1440, 390]) {
  test(`downloads exact historical content without a seal at ${width}px`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize({ width, height: 1000 });
    const { calls, records } = await contentSetup(page, width === 390 ? 'paralegal' : 'owner');
    await expect(downloadButton(page)).toBeEnabled();
    await expect(
      page.getByRole('button', { name: 'Verificar integridad', exact: true }),
    ).toBeDisabled();
    await expect(
      page.getByRole('button', { name: 'Descargar evidencia', exact: true }),
    ).toBeDisabled();
    const downloaded = page.waitForEvent('download');
    await downloadButton(page).click();
    const result = await downloaded;
    expect(await downloadBytes(result)).toEqual(contentBytes);
    expect(result.suggestedFilename()).toBe('contrato-v1.pdf');
    await expect(page.getByText('Descarga del archivo iniciada.', { exact: true })).toBeVisible();
    await expect(
      page.getByText('Consultando versi\u00f3n hist\u00f3rica: 1', { exact: true }),
    ).toBeVisible();
    await expect(
      page.locator('.detail-panel').getByText('Pendiente de sello', { exact: true }),
    ).toBeVisible();
    expect(
      calls
        .filter((call) => call.path.endsWith('/content'))
        .map((call) => [call.method, call.path]),
    ).toEqual([['GET', contentPath]]);
    expect(calls.some((call) => /\/(seal|verify|evidence)$/.test(call.path))).toBe(false);
    expect(records.every((record) => record.sealed === false)).toBe(true);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    await page.screenshot({ path: testInfo.outputPath('document-content.png'), fullPage: true });
  });
}

test('validation failure shows a useful error and creates no downloadable object', async ({
  page,
}) => {
  await contentSetup(page);
  let requests = 0;
  const downloads = [];
  page.on('download', (value) => downloads.push(value));
  await page.evaluate(() => {
    const original = URL.createObjectURL;
    window.contentUrls = 0;
    URL.createObjectURL = function (...args) {
      window.contentUrls++;
      return original.apply(this, args);
    };
  });
  await page.route(`**${contentPath}`, (route) => {
    requests++;
    return route.fulfill({
      status: 409,
      json: { error: { code: 'document_content_validation_failed' } },
    });
  });
  await downloadButton(page).click();
  await expect(page.getByRole('alert')).toContainText('su validaci\u00f3n fall\u00f3');
  await expect(downloadButton(page)).toBeEnabled();
  expect(downloads).toHaveLength(0);
  expect(await page.evaluate(() => window.contentUrls)).toBe(0);
  expect(requests).toBe(1);
  await expect(page.getByRole('heading', { name: 'contrato.pdf', exact: true })).toBeVisible();
});

test('content access denial clears the document and its historical detail', async ({ page }) => {
  await contentSetup(page, 'paralegal');
  const downloads = [];
  page.on('download', (value) => downloads.push(value));
  await page.route(`**${contentPath}`, (route) =>
    route.fulfill({
      status: 403,
      json: { error: { code: 'permission_denied' } },
    }),
  );
  await downloadButton(page).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(page.locator('.document-focus')).toHaveCount(0);
  await expect(
    page.locator('.document-list').getByText('contrato.pdf', { exact: true }),
  ).toHaveCount(0);
  expect(downloads).toHaveLength(0);
});
