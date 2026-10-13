import { test, expect } from '@playwright/test';
import { setup, login, navigate, caseRecord } from './helpers.mjs';
import { administration, overview, fillPenal, profile } from './case-administration-helpers.mjs';

async function summary(page) {
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
}

test('uncertain edit can explicitly consult current values while retaining the original draft', async ({
  page,
}) => {
  await setup(page);
  let current = administration(caseRecord, 1, profile),
    writes = 0;
  await page.route(`**/cases/${caseRecord.id}/administration`, (route) => {
    if (route.request().method() === 'GET') return route.fulfill({ json: current });
    writes++;
    current = administration({ ...caseRecord, reference: 'Otra referencia' }, 2, profile);
    return route.abort('failed');
  });
  await summary(page);
  await page.getByRole('button', { name: 'Editar ficha penal', exact: true }).click();
  await page.getByLabel('Referencia interna', { exact: true }).fill('Mi referencia');
  await page.getByRole('button', { name: 'Guardar ficha penal', exact: true }).click();
  await expect(page.locator('.case-editor').getByRole('alert')).toContainText(
    'confirmar el resultado',
  );
  await page.getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
  await expect(page.locator('.case-comparison')).toContainText('Otra referencia');
  await expect(page.getByLabel('Referencia interna', { exact: true })).toHaveValue('Mi referencia');
  expect(writes).toBe(1);
});

test('uncertain status can reconcile already applied state without resending', async ({ page }) => {
  await setup(page);
  let current = administration(caseRecord),
    writes = 0;
  await page.route(`**/cases/${caseRecord.id}/administration`, (route) =>
    route.fulfill({ json: current }),
  );
  await page.route('**/administrative-status', (route) => {
    writes++;
    current = administration(caseRecord, 2, null, 'closed');
    return route.abort('failed');
  });
  await summary(page);
  await page.getByRole('button', { name: 'Cerrar administrativamente', exact: true }).click();
  await page.getByRole('button', { name: 'Confirmar cierre administrativo', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('button', { name: 'Consultar datos actuales', exact: true }).click();
  await expect(
    dialog.getByText('El expediente ya tiene el estado solicitado.', { exact: false }),
  ).toBeVisible();
  expect(writes).toBe(1);
});

test('uncertain creation queries permitted matching cases without inferring success or losing its form', async ({
  page,
}) => {
  await setup(page);
  let writes = 0;
  const queries = [];
  await page.route('**/api/v1/penal-cases', (route) => {
    writes++;
    return route.abort('failed');
  });
  await page.route('**/api/v1/case-administrations?*', (route) => {
    const query = new URL(route.request().url()).searchParams;
    queries.push(query);
    return route.fulfill({
      json: {
        cases: [overview(administration(caseRecord, 1, profile))],
        has_more: false,
        next_after_id: null,
      },
    });
  });
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: 'Nuevo expediente penal', exact: true }).click();
  await fillPenal(page);
  await page.getByRole('button', { name: 'Crear expediente penal', exact: true }).click();
  await expect(page.locator('.case-editor').getByRole('alert')).toContainText(
    'confirmar el resultado',
  );
  await page.getByLabel('NUC', { exact: true }).fill('BORRADOR-NUEVO');
  await page.getByRole('button', { name: 'Consultar expedientes guardados', exact: true }).click();
  await expect(
    page.getByRole('region', { name: 'Coincidencias guardadas', exact: true }),
  ).toContainText('Defensa inicial');
  await expect(page.getByLabel('T\u00edtulo del expediente', { exact: true })).toHaveValue(
    'Alta penal',
  );
  expect(queries.at(-1).get('nuc')).toBe('NUC-PENAL');
  expect(queries.at(-1).get('judicial_case_number')).toBe('CJ-PENAL');
  expect(queries.at(-1).get('status')).toBe('all');
  expect(writes).toBe(1);
});
