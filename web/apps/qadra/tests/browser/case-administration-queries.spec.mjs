import { test, expect } from '@playwright/test';
import { setup, login, navigate, caseRecord, otherCaseId } from './helpers.mjs';
import { administration, overview, fillPenal, profile } from './case-administration-helpers.mjs';

test('staff filters combine exact identifiers with literal title and exclusive cursor before pages', async ({
  page,
}) => {
  const calls = [];
  await setup(page);
  await page.route('**/api/v1/case-administrations?*', (route) => {
    const query = new URL(route.request().url()).searchParams;
    calls.push(query);
    const more = !query.get('after_id');
    return route.fulfill({
      json: {
        cases: [
          overview(
            administration(
              more ? caseRecord : { ...caseRecord, id: otherCaseId, title: 'Pagina siguiente' },
            ),
          ),
        ],
        has_more: more,
        next_after_id: more ? caseRecord.id : null,
      },
    });
  });
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByLabel('Buscar por t\u00edtulo').fill('Parte%');
  await page.getByLabel('NUC exacto').fill('NUC-A');
  await page.getByLabel('Carpeta judicial exacta').fill('CJ-A');
  await page
    .getByRole('combobox', { name: 'Estado administrativo', exact: true })
    .selectOption('closed');
  await page.getByRole('combobox', { name: 'Ficha penal', exact: true }).selectOption('complete');
  await page.getByRole('button', { name: 'Buscar expedientes', exact: true }).click();
  await page.getByRole('button', { name: 'Siguiente', exact: true }).click();
  await expect(page.getByRole('button', { name: /Pagina siguiente/ })).toBeVisible();
  expect(Object.fromEntries(calls.at(-1))).toEqual({
    limit: '50',
    title: 'Parte%',
    nuc: 'NUC-A',
    judicial_case_number: 'CJ-A',
    status: 'closed',
    profile: 'complete',
    after_id: caseRecord.id,
  });
  await page.getByRole('button', { name: 'Anterior', exact: true }).click();
  await expect(page.getByRole('button', { name: /Defensa inicial/ })).toBeVisible();
  expect(calls.at(-1).has('after_id')).toBe(false);
});

test('penal validation rejects duplicate descriptions while preserving pending Unicode text and server conflict draft', async ({
  page,
}) => {
  await setup(page);
  let writes = 0;
  await page.route('**/api/v1/penal-cases', (route) => {
    writes++;
    return route.fulfill({ status: 409, json: { error: { code: 'case_identifier_conflict' } } });
  });
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: 'Nuevo expediente penal', exact: true }).click();
  await fillPenal(page);
  await page.getByRole('button', { name: 'Agregar descripci\u00f3n', exact: true }).click();
  await page
    .getByLabel('Nueva descripci\u00f3n de delito', { exact: true })
    .fill(' Descripcion inicial ');
  await page.getByRole('button', { name: 'Crear expediente penal', exact: true }).click();
  await expect(page.locator('.case-offenses').getByRole('alert')).toContainText('ya est');
  expect(writes).toBe(0);
  await expect(page.getByLabel('Nueva descripci\u00f3n de delito', { exact: true })).toHaveValue(
    ' Descripcion inicial ',
  );
  await page.getByLabel('Nueva descripci\u00f3n de delito', { exact: true }).fill('\u00c1, B');
  await page.getByRole('button', { name: 'Crear expediente penal', exact: true }).click();
  await expect(page.locator('.case-editor').getByRole('alert')).toContainText('NUC o la carpeta');
  expect(writes).toBe(1);
  await expect(page.getByLabel('T\u00edtulo del expediente', { exact: true })).toHaveValue(
    'Alta penal',
  );
  await expect(page.locator('.case-offense-list')).toContainText('\u00c1, B');
});

test('history uses positive exclusive revisions and retains captured provenance', async ({
  page,
}) => {
  await setup(page);
  const queries = [];
  await page.route(`**/cases/${caseRecord.id}/administration/history?*`, (route) => {
    const query = new URL(route.request().url()).searchParams;
    queries.push(query);
    const more = !query.get('before_revision');
    return route.fulfill({
      json: {
        revisions: [administration(caseRecord, more ? 3 : 1, more ? profile : null).administration],
        has_more: more,
        next_before_revision: more ? 3 : null,
      },
    });
  });
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await page.getByRole('button', { name: 'Ver historial administrativo', exact: true }).click();
  await page.getByRole('button', { name: 'Cargar revisiones anteriores', exact: true }).click();
  await expect(page.locator('.case-history details')).toHaveCount(2);
  expect(queries.at(-1).get('before_revision')).toBe('3');
  await page.locator('.case-history summary').first().click();
  await expect(page.locator('.case-history details[open]')).toContainText('owner@example.com');
  await expect(page.locator('.case-history details[open] code')).toHaveText('c'.repeat(64));
});

for (const code of ['case_revision_exhausted', 'internal_error'])
  test(`case edit preserves draft without misleading CAS refresh for ${code}`, async ({ page }) => {
    await setup(page);
    await page.route(`**/cases/${caseRecord.id}/administration`, (route) =>
      route.request().method() === 'GET'
        ? route.fulfill({ json: administration(caseRecord, 1, profile) })
        : route.fulfill({
            status: code === 'internal_error' ? 500 : 409,
            json: { error: { code } },
          }),
    );
    await login(page, false, false);
    await navigate(page, 'Expedientes');
    await page.getByRole('button', { name: /Defensa inicial/ }).click();
    await page.getByRole('button', { name: 'Editar ficha penal', exact: true }).click();
    await page.getByLabel('Referencia interna', { exact: true }).fill('Preservada');
    await page.getByRole('button', { name: 'Guardar ficha penal', exact: true }).click();
    await expect(page.locator('.case-editor').getByRole('alert')).toBeVisible();
    await expect(page.getByLabel('Referencia interna', { exact: true })).toHaveValue('Preservada');
    await expect(
      page.getByRole('button', { name: 'Consultar datos actuales', exact: true }),
    ).toHaveCount(0);
    if (code === 'case_revision_exhausted')
      await expect(
        page.getByRole('button', { name: 'Guardar ficha penal', exact: true }),
      ).toBeDisabled();
  });

test('mobile penal form keeps eight long descriptions controls and multiline text readable', async ({
  page,
}, testInfo) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await setup(page);
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: 'Nuevo expediente penal', exact: true }).click();
  await fillPenal(page);
  for (let i = 0; i < 8; i++) {
    await page
      .getByLabel('Nueva descripci\u00f3n de delito', { exact: true })
      .fill(`${i} ${'Texto largo, '.repeat(8)}`);
    await page.getByRole('button', { name: 'Agregar descripci\u00f3n', exact: true }).click();
  }
  await expect(page.locator('.case-offense-list li')).toHaveCount(8);
  const field = await page.getByLabel('NUC', { exact: true }).boundingBox();
  expect(field.width).toBeGreaterThan(200);
  const input = await page
    .getByLabel('Nueva descripci\u00f3n de delito', { exact: true })
    .boundingBox();
  const add = await page
    .getByRole('button', { name: 'Agregar descripci\u00f3n', exact: true })
    .boundingBox();
  expect(input.y + input.height <= add.y || input.x + input.width <= add.x).toBe(true);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.evaluate(() => {
    document.activeElement?.blur();
    scrollTo(0, 0);
  });
  await page.screenshot({ path: testInfo.outputPath('penal-create-mobile.png'), fullPage: true });
});
