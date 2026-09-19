import { expect } from '@playwright/test';
import { setup, login, caseId, id, document } from './helpers.mjs';

export { caseId, id, document };
export const contentBytes = Buffer.from([0, 255, 1, 13, 10, 128]);
export const contentPath = `/api/v1/cases/${caseId}/documents/${id}/versions/1/content`;
export function contentResponse() {
  return {
    body: contentBytes,
    headers: {
      'Content-Type': 'application/octet-stream',
      'Content-Disposition': 'attachment; filename="contrato-v1.pdf"',
      'Cache-Control': 'no-store',
      'X-Content-Type-Options': 'nosniff',
      'X-Case-Id': caseId,
      'X-Document-Id': id,
      'X-Document-Version': '1',
      'X-Document-Digest': document.digest,
    },
  };
}
export async function contentSetup(page, role = 'owner') {
  const calls = await setup(page, role);
  const records = [{ ...document }, { ...document, version: 2, name: 'actual.pdf' }];
  await page.route(`**/cases/${caseId}/documents/**`, async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const suffix = url.pathname.split(`/documents/${id}`)[1];
    if (suffix?.startsWith('/metadata')) return route.fallback();
    calls.push({ path: url.pathname, method: request.method() });
    if (suffix === '') return route.fulfill({ json: records.at(-1) });
    if (suffix === '/versions')
      return route.fulfill({
        json: {
          versions: [...records].reverse(),
          has_more: false,
          next_before_version: null,
          first_available_version: 1,
        },
      });
    if (suffix === '/versions/1/content') return route.fulfill(contentResponse());
    const version = Number(suffix?.split('/')[2]);
    if (/^\/versions\/\d+$/.test(suffix || ''))
      return route.fulfill({ json: records.find((record) => record.version === version) });
    return route.fallback();
  });
  await login(page);
  await page.locator('.reference-panel').getByLabel('Identificador del documento').fill(id);
  await page.getByRole('button', { name: 'Abrir documento', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'actual.pdf', exact: true })).toBeVisible();
  await expect(page.locator('.document-metadata')).toHaveAttribute('aria-busy', 'false');
  await page.getByRole('button', { name: /Versi\u00f3n 1.*contrato.pdf/ }).click();
  await expect(page.getByRole('heading', { name: 'contrato.pdf', exact: true })).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Actualizar historial', exact: true }),
  ).toBeEnabled();
  return { calls, records };
}
export async function downloadBytes(download) {
  const chunks = [];
  for await (const chunk of await download.createReadStream()) chunks.push(chunk);
  return Buffer.concat(chunks);
}
