import { test, expect } from '@playwright/test';
import { setup, login, openDocument, caseId, id, document, validReport } from './helpers.mjs';

const deferred = () => {
  let resolve;
  const promise = new Promise((finish) => (resolve = finish));
  return { promise, resolve };
};

for (const status of [503, 403]) {
  test(`a ${status} refresh after seal preserves committed evidence or clears denied data`, async ({
    page,
  }) => {
    await setup(page);
    await login(page);
    await openDocument(page);
    await expect(page.locator('.document-metadata')).toHaveAttribute('aria-busy', 'false');
    await expect(
      page.getByRole('button', { name: 'Actualizar historial', exact: true }),
    ).toBeEnabled();
    let seals = 0;
    await page.route(`**/documents/${id}/versions/1/seal`, (route) => {
      seals++;
      return route.fulfill({ json: { ...document, sealed: true } });
    });
    await page.route(`**/cases/${caseId}/documents?*`, (route) =>
      route.fulfill({
        status,
        json: { error: { code: status === 503 ? 'server_busy' : 'permission_denied' } },
      }),
    );
    await page.getByRole('button', { name: 'Sellar documento', exact: true }).click();
    await page.getByRole('button', { name: 'Confirmar sellado', exact: true }).click();
    await expect(page.getByRole('alert')).toBeVisible();
    if (status === 503) {
      await expect(
        page.locator('.detail-panel').getByText('Sellado', { exact: true }).first(),
      ).toBeVisible();
      await expect(
        page.getByRole('button', { name: 'Descargar evidencia', exact: true }),
      ).toBeEnabled();
      await expect(page.getByRole('button', { name: 'Sellar documento', exact: true })).toHaveCount(
        0,
      );
    } else {
      await expect(page.locator('.document-focus')).toHaveCount(0);
    }
    expect(seals).toBe(1);
  });
}
