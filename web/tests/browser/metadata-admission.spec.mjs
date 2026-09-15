import { test, expect } from '@playwright/test';
import { metadataSetup } from './metadata-helpers.mjs';
import { caseId, id } from './helpers.mjs';
const deferred = () => {
  let resolve;
  const promise = new Promise((finish) => (resolve = finish));
  return { promise, resolve };
};

for (const conflict of [false, true]) {
  test(`classification ${conflict ? 'conflict refresh' : 'save'} coordinates list and visible history before document actions`, async ({
    page,
  }) => {
    await metadataSetup(page, { conflict });
    await page
      .getByRole('button', { name: 'Ver historial de clasificaci\u00f3n', exact: true })
      .click();
    await expect(page.locator('.metadata-history')).toHaveAttribute('aria-busy', 'false');
    await page.getByRole('button', { name: 'Editar clasificaci\u00f3n', exact: true }).click();
    const modal = page.getByRole('dialog', { name: 'Editar clasificaci\u00f3n', exact: true });
    await modal.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true }).fill('Actualizada');
    const listStarted = deferred(),
      historyStarted = deferred(),
      release = deferred();
    await page.route(`**/cases/${caseId}/documents?*`, async (route) => {
      listStarted.resolve();
      await release.promise;
      return route.fallback();
    });
    await page.route(`**/documents/${id}/metadata/history?*`, async (route) => {
      historyStarted.resolve();
      await release.promise;
      return route.fallback();
    });
    await modal.getByRole('button', { name: 'Guardar clasificaci\u00f3n', exact: true }).click();
    if (conflict) {
      await expect(modal.getByRole('alert')).toBeVisible();
      await modal
        .getByRole('button', { name: 'Consultar clasificaci\u00f3n actual', exact: true })
        .click();
    }
    await Promise.all([listStarted.promise, historyStarted.promise]);
    try {
      await expect(
        page.getByRole('button', { name: 'Descargar evidencia', exact: true }),
      ).toBeDisabled();
      if (conflict) await expect(modal.locator('.dialog-actions .primary')).toBeDisabled();
    } finally {
      release.resolve();
    }
    if (conflict)
      await expect(
        modal.getByRole('button', { name: 'Guardar mis cambios', exact: true }),
      ).toBeEnabled();
    else
      await expect(
        page.getByRole('button', { name: 'Descargar evidencia', exact: true }),
      ).toBeEnabled();
  });
}
