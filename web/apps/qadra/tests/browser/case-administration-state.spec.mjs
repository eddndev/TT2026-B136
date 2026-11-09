import { test, expect } from '@playwright/test';
import { setup, login, navigate, caseRecord, document } from './helpers.mjs';
import { administration, profile } from './case-administration-helpers.mjs';

async function summary(page) {
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await expect(
    page.getByRole('heading', { name: 'Resumen del expediente', exact: true }),
  ).toBeVisible();
}

test('profile conflict retains draft and explicit replacement preserves independent status and stage', async ({
  page,
}) => {
  await setup(page);
  let current = administration(caseRecord, 1, profile),
    writes = [];
  await page.route(`**/api/v1/cases/${caseRecord.id}/administration`, async (route) => {
    if (route.request().method() === 'PUT') {
      const body = route.request().postDataJSON();
      writes.push(body);
      if (writes.length === 1) {
        current = administration({ ...caseRecord, title: 'Cambio concurrente' }, 2, profile);
        return route.fulfill({ status: 409, json: { error: { code: 'case_revision_conflict' } } });
      }
      current = administration({ ...caseRecord, ...body }, 3, body.profile);
    }
    await route.fulfill({ json: current });
  });
  await summary(page);
  await page.getByRole('button', { name: 'Editar ficha penal', exact: true }).click();
  // The form takes the place of the values inside the summary card.
  await expect(page.locator('.case-summary .case-editor')).toBeVisible();
  await expect(page.locator('.case-summary .case-values')).toHaveCount(0);
  await page.getByLabel('T\u00edtulo del expediente', { exact: true }).fill('Mi borrador');
  await page.getByRole('button', { name: 'Guardar ficha penal', exact: true }).click();
  await expect(page.getByLabel('T\u00edtulo del expediente', { exact: true })).toHaveValue(
    'Mi borrador',
  );
  await page.getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Valores actuales guardados' })).toContainText(
    'Cambio concurrente',
  );
  expect(writes.length).toBe(1);
  await page.getByRole('button', { name: 'Guardar mis cambios', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Mi borrador', exact: true })).toBeVisible();
  expect(writes[1].expected_revision).toBe(2);
  expect(Object.keys(writes[1]).sort()).toEqual([
    'expected_revision',
    'profile',
    'reference',
    'title',
  ]);
});

test('closed case retains version reading but disables every document mutation', async ({
  page,
}) => {
  await setup(page);
  await page.route(`**/api/v1/cases/${caseRecord.id}/administration`, (route) =>
    route.fulfill({ json: administration(caseRecord, 2, profile, 'closed') }),
  );
  await login(page);
  await page.getByRole('button', { name: `Abrir ${document.name}`, exact: true }).click();
  for (const name of [
    'Subir documento',
    'Agregar versi\u00f3n',
    'Editar clasificaci\u00f3n',
    'Sellar documento',
  ])
    await expect(page.getByRole('button', { name, exact: true })).toBeDisabled();
  await expect(
    page.getByRole('button', { name: 'Actualizar historial', exact: true }),
  ).toBeEnabled();
});
