import { test, expect } from '@playwright/test';
import { setup, login, openDocument, caseId, id, document, validReport } from './helpers.mjs';

const deferred = () => {
  let resolve;
  const promise = new Promise((finish) => (resolve = finish));
  return { promise, resolve };
};

for (const action of ['verify', 'evidence']) {
  test(`seal keeps ${action} unavailable until its automatic reads finish`, async ({ page }) => {
    await setup(page);
    await login(page);
    await openDocument(page);
    await expect(page.locator('.document-metadata')).toHaveAttribute('aria-busy', 'false');
    await expect(
      page.getByRole('button', { name: 'Actualizar historial', exact: true }),
    ).toBeEnabled();
    const release = deferred(),
      listStarted = deferred(),
      historyStarted = deferred();
    let active = 0,
      rejected = 0,
      actions = 0;
    const automatic = async (route, started, json) => {
      active++;
      started.resolve();
      await release.promise;
      active--;
      return route.fulfill({ json });
    };
    await page.route(`**/cases/${caseId}/documents?*`, (route) =>
      automatic(route, listStarted, {
        documents: [{ ...document, sealed: true }],
        has_more: false,
      }),
    );
    await page.route(`**/documents/${id}/versions?*`, (route) =>
      automatic(route, historyStarted, {
        versions: [{ ...document, sealed: true }],
        has_more: false,
        next_before_version: null,
        first_available_version: 1,
      }),
    );
    await page.route(`**/documents/${id}/versions/1/seal`, (route) =>
      route.fulfill({ json: { ...document, sealed: true } }),
    );
    await page.route(`**/documents/${id}/versions/1/${action}`, (route) => {
      actions++;
      if (active >= 2) {
        rejected++;
        return route.fulfill({ status: 503, json: { error: { code: 'server_busy' } } });
      }
      return route.fulfill(
        action === 'verify'
          ? { json: validReport }
          : {
              contentType: 'application/zip',
              body: 'immutable evidence',
              headers: { 'X-Document-Id': id, 'X-Document-Version': '1' },
            },
      );
    });
    await page.getByRole('button', { name: 'Sellar documento', exact: true }).click();
    await page.getByRole('button', { name: 'Confirmar sellado', exact: true }).click();
    await Promise.all([listStarted.promise, historyStarted.promise]);
    const button = page.getByRole('button', {
      name: action === 'verify' ? 'Verificar integridad' : 'Descargar evidencia',
      exact: true,
    });
    try {
      await expect(button).toBeDisabled();
      expect(active).toBe(2);
      expect(actions).toBe(0);
    } finally {
      release.resolve();
    }
    await expect(button).toBeEnabled();
    const download = action === 'evidence' ? page.waitForEvent('download') : null;
    await button.click();
    if (download) await download;
    else
      await expect(page.getByText('Verificaci\u00f3n v\u00e1lida', { exact: true })).toBeVisible();
    expect(rejected).toBe(0);
    expect(actions).toBe(1);
    await expect(page.getByRole('alert')).toHaveCount(0);
  });
}

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
