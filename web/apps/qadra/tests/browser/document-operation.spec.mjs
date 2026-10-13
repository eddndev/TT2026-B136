import { test, expect } from '@playwright/test';
import { setup, login, openDocument, caseId, id, document } from './helpers.mjs';
import { versionSetup, append } from './version-helpers.mjs';
import { metadataSetup } from './metadata-helpers.mjs';
const deferred = () => {
  let resolve;
  const promise = new Promise((finish) => (resolve = finish));
  return { promise, resolve };
};

for (const action of ['seal', 'append', 'metadata']) {
  test(`the ${action} request blocks neighboring document operations until it settles`, async ({
    page,
  }) => {
    if (action === 'append') await versionSetup(page);
    else if (action === 'metadata') await metadataSetup(page);
    else {
      await setup(page);
      await login(page);
      await openDocument(page);
      await expect(page.locator('.document-metadata')).toHaveAttribute('aria-busy', 'false');
      await expect(
        page.getByRole('button', { name: 'Actualizar historial', exact: true }),
      ).toBeEnabled();
    }
    const started = deferred(),
      release = deferred();
    const path =
      action === 'seal'
        ? `/versions/1/seal`
        : action === 'append'
          ? '/versions?expected_version=*'
          : '/metadata';
    await page.route(`**/documents/${id}${path}`, async (route) => {
      started.resolve();
      await release.promise;
      const json =
        action === 'metadata'
          ? {
              case_id: caseId,
              id,
              metadata_revision: 2,
              document_type: null,
              classification: 'Actualizada',
              tags: [],
            }
          : { ...document, sealed: action === 'seal', version: action === 'append' ? 2 : 1 };
      return route.fulfill({ status: action === 'append' ? 201 : 200, json });
    });
    if (action === 'seal') {
      await page.getByRole('button', { name: 'Sellar documento', exact: true }).click();
      await page.getByRole('button', { name: 'Confirmar sellado', exact: true }).click();
    } else if (action === 'append') await append(page);
    else {
      await page.getByRole('button', { name: 'Editar clasificaci\u00f3n', exact: true }).click();
      const modal = page.getByRole('dialog', { name: 'Editar clasificaci\u00f3n', exact: true });
      await modal.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true }).fill('Actualizada');
      await modal.getByRole('button', { name: 'Guardar clasificaci\u00f3n', exact: true }).click();
    }
    await started.promise;
    try {
      await expect(
        page.getByRole('button', { name: 'Actualizar historial', exact: true }),
      ).toBeDisabled();
      if (action === 'metadata') {
        await expect(
          page.getByRole('button', { name: 'Agregar versi\u00f3n', exact: true }),
        ).toBeDisabled();
        await expect(
          page.getByRole('button', { name: 'Verificar integridad', exact: true }),
        ).toBeDisabled();
      } else {
        await expect(
          page.getByRole('button', { name: 'Editar clasificaci\u00f3n', exact: true }),
        ).toBeDisabled();
        await expect(
          page.getByRole('button', { name: 'Actualizar clasificaci\u00f3n', exact: true }),
        ).toBeDisabled();
      }
    } finally {
      release.resolve();
    }
    await expect(
      page.getByRole('button', { name: 'Actualizar historial', exact: true }),
    ).toBeEnabled();
  });
}
