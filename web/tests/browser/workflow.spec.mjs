import { test, expect } from '@playwright/test';
import {
  caseId,
  id,
  document,
  validReport,
  setup,
  login,
  navigate,
  openDocument,
} from './helpers.mjs';

test.beforeEach(async ({ page }) => {
  page.runtimeErrors = [];
  page.on('pageerror', (error) => page.runtimeErrors.push(error.message));
});
test.afterEach(async ({ page }) => expect(page.runtimeErrors).toEqual([]));

test('overview offers case actions and browser navigation without losing session', async ({
  page,
}) => {
  await setup(page);
  await login(page, false, false);
  await expect(page.getByRole('region', { name: 'Acciones del despacho' })).toContainText(
    'Expedientes',
  );
  await page.getByRole('button', { name: 'Seleccionar expediente', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Expedientes', exact: true })).toBeVisible();
  await page.goBack();
  await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
  await page.getByRole('button', { name: 'C\u00f3mo funciona' }).click();
  await expect(
    page.getByRole('heading', { name: 'Un recorrido claro, de principio a fin' }),
  ).toBeVisible();
});

test('an existing unsealed UUID opens by GET with an actionable seal step', async ({ page }) => {
  const requests = await setup(page);
  await login(page);
  await openDocument(page);
  await expect(page.getByRole('button', { name: 'Sellar documento', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Verificar integridad' })).toBeDisabled();
  expect(requests.some((item) => item.path.endsWith('/verify'))).toBe(false);
});

test('upload normalizes names and refreshed server filters preserve stored records', async ({
  page,
}) => {
  const requests = await setup(page, 'owner', []);
  await login(page);
  await page.getByRole('button', { name: 'Subir documento', exact: true }).click();
  await page.getByLabel('Archivo', { exact: true }).setInputFiles({
    name: 'Dem\u00e1nda inicial.pdf',
    mimeType: 'application/pdf',
    buffer: Buffer.from('evidence'),
  });
  await expect(page.getByLabel('Nombre del documento')).toHaveValue('Demanda-inicial.pdf');
  await page.getByRole('button', { name: 'Cargar documento' }).click();
  await expect(page.getByRole('heading', { name: 'Demanda-inicial.pdf' })).toBeVisible();
  expect(
    requests.find((item) => item.path.endsWith('/documents') && item.method === 'POST').headers[
      'x-document-name'
    ],
  ).toBe('Demanda-inicial.pdf');
  await page.getByRole('button', { name: 'Vista de tarjetas' }).click();
  await expect(page.locator('.document-card')).toHaveCount(1);
  await page.getByLabel('Filtrar por estado').selectOption('sealed');
  await expect(page.getByText('No encontramos coincidencias')).toBeVisible();
  await page.getByRole('button', { name: 'Limpiar filtros' }).click();
  await expect(page.locator('.document-card')).toHaveCount(1);
  await navigate(page, 'Inicio');
  await page.screenshot({ path: 'test-results/overview-desktop.png', fullPage: true });
});

test('mobile menu supports Escape and password visibility is explicit', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await setup(page);
  await page.getByRole('button', { name: 'Mostrar contrase\u00f1a' }).click();
  await expect(page.getByLabel('Contrase\u00f1a', { exact: true })).toHaveAttribute('type', 'text');
  await page.getByRole('button', { name: 'Ocultar contrase\u00f1a' }).click();
  await login(page, false, false);
  await page.getByRole('button', { name: 'Abrir men\u00fa' }).click();
  await expect(page.getByRole('dialog', { name: 'Men\u00fa del despacho' })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog', { name: 'Men\u00fa del despacho' })).not.toBeVisible();
  await expect(page.getByRole('button', { name: 'Abrir men\u00fa' })).toBeFocused();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: 'test-results/overview-mobile.png', fullPage: true });
});

test('initial enrollment is acknowledged before returning to login', async ({ page }) => {
  await setup(page);
  await page.getByRole('button', { name: 'Configurar acceso inicial' }).click();
  await page.getByLabel('Correo electr\u00f3nico').fill('hatz@example.com');
  await page.getByLabel('Contrase\u00f1a', { exact: true }).fill('long-test-password');
  await page.getByRole('button', { name: 'Crear administrador' }).click();
  await expect(page.getByRole('button', { name: 'Finalizar' })).toBeDisabled();
  await page.getByLabel('Ya guard\u00e9 la clave y los c\u00f3digos').check();
  await page.getByRole('button', { name: 'Finalizar' }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  expect(await page.evaluate(() => document.body.textContent.includes('TESTSECRET'))).toBe(false);
});

test('failed verification clears a previous verdict and reopening detail resets it', async ({
  page,
}) => {
  await setup(page, 'owner', [{ ...document, sealed: true }]);
  await login(page);
  await openDocument(page);
  await page.getByRole('button', { name: 'Verificar integridad' }).click();
  await expect(page.getByText('Verificaci\u00f3n v\u00e1lida', { exact: true })).toBeVisible();
  await page.route('**/documents/*/verify', (route) =>
    route.fulfill({ status: 500, json: { error: { code: 'internal_error' } } }),
  );
  await page.getByRole('button', { name: 'Verificar integridad' }).click();
  await expect(page.getByRole('alert')).toContainText('servidor');
  await expect(page.getByText('Verificaci\u00f3n v\u00e1lida', { exact: true })).toHaveCount(0);
  await page.unroute('**/documents/*/verify');
  await page.getByRole('button', { name: 'Verificar integridad' }).click();
  await expect(page.getByText('Verificaci\u00f3n v\u00e1lida', { exact: true })).toBeVisible();
  await openDocument(page);
  await expect(page.getByText('Verificaci\u00f3n v\u00e1lida', { exact: true })).toHaveCount(0);
});

test('failed evidence verdict shows the failed component', async ({ page }) => {
  await setup(page, 'owner', [{ ...document, sealed: true }]);
  await login(page);
  await openDocument(page);
  await page.route('**/documents/*/verify', (route) =>
    route.fulfill({
      json: {
        ...validReport,
        verdict: 'not_valid',
        integrity: { status: 'failed', detail: 'The stored digest does not match' },
      },
    }),
  );
  await page.getByRole('button', { name: 'Verificar integridad' }).click();
  await expect(page.getByText('Verificaci\u00f3n no v\u00e1lida', { exact: true })).toBeVisible();
  await page
    .getByText('Ver detalles t\u00e9cnicos de la verificaci\u00f3n', { exact: true })
    .click();
  await expect(page.getByText('The stored digest does not match')).toBeVisible();
});

test('a broken audit shows index zero and clears verdict when recheck fails', async ({ page }) => {
  await setup(page);
  await login(page, false, false);
  await navigate(page, 'Auditor\u00eda');
  await page.route('**/audit/verify', (route) =>
    route.fulfill({ json: { valid: false, entries: null, first_broken_index: 0 } }),
  );
  await page.getByRole('button', { name: 'Verificar cadena' }).click();
  await expect(page.getByText('Primer \u00edndice roto: 0')).toBeVisible();
  await page.route('**/audit/verify', (route) => route.fulfill({ status: 502 }));
  await page.getByRole('button', { name: 'Verificar cadena' }).click();
  await expect(page.getByRole('alert')).toContainText('servidor');
  await expect(page.getByText('Primer \u00edndice roto: 0')).toHaveCount(0);
});

test('login upload seal verify download logout follow the case HTTP contract', async ({ page }) => {
  const requests = await setup(page, 'owner', []);
  await login(page);
  await page.getByRole('button', { name: 'Subir documento', exact: true }).click();
  await page.getByLabel('Archivo', { exact: true }).setInputFiles({
    name: 'contrato.pdf',
    mimeType: 'application/pdf',
    buffer: Buffer.from('document bytes'),
  });
  await page.getByRole('button', { name: 'Cargar documento' }).click();
  await expect(page.getByRole('heading', { name: 'contrato.pdf' })).toBeVisible();
  await page.getByRole('button', { name: 'Sellar documento', exact: true }).click();
  await page.getByRole('button', { name: 'Confirmar sellado' }).click();
  await expect(page.getByText('Documento sellado correctamente.')).toBeVisible();
  await page.getByRole('button', { name: 'Verificar integridad' }).click();
  await expect(page.getByText('Verificaci\u00f3n v\u00e1lida', { exact: true })).toBeVisible();
  const downloaded = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Descargar evidencia' }).click();
  expect((await downloaded).suggestedFilename()).toBe(`evidencia-${id}.zip`);
  const upload = requests.find(
    (item) => item.path.endsWith('/documents') && item.method === 'POST',
  );
  expect(upload.path).toBe(`/api/v1/cases/${caseId}/documents`);
  expect(upload.headers.authorization).toBe('Bearer test-token-1');
  expect(upload.body).toBe('document bytes');
  expect(
    requests.filter((item) => item.path.endsWith('/documents') && item.method === 'GET').length,
  ).toBeGreaterThanOrEqual(3);
  await page.evaluate(() => scrollTo(0, 0));
  await page.screenshot({ path: 'test-results/document-desktop.png', fullPage: true });
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n' }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  expect(await page.evaluate(() => Object.keys(localStorage))).toEqual([]);
});

test('paralegal cannot seal or manage assignments', async ({ page }) => {
  await setup(page, 'paralegal');
  await login(page, true);
  await openDocument(page);
  await expect(page.getByRole('button', { name: 'Sellar documento', exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Equipo', exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Asignaciones', exact: true })).toHaveCount(0);
});

test('rejected MFA requests fresh credentials; expired session clears case content', async ({
  page,
}) => {
  await setup(page);
  await page.route('**/auth/mfa/totp', (route) =>
    route.fulfill({ status: 401, json: { error: { code: 'mfa_rejected' } } }),
  );
  await page.getByLabel('Correo electr\u00f3nico').fill('hatz@example.com');
  await page.getByLabel('Contrase\u00f1a', { exact: true }).fill('a-long-password');
  await page.getByRole('button', { name: 'Continuar', exact: true }).click();
  await page.getByLabel('C\u00f3digo de 6 d\u00edgitos', { exact: true }).fill('123456');
  await page.getByRole('button', { name: 'Verificar y entrar' }).click();
  await expect(page.getByRole('alert')).toContainText('rechazado');
  await page.unroute('**/auth/mfa/totp');
  await login(page);
  await page.route('**/documents/*', (route) =>
    route.fulfill({ status: 401, json: { error: { code: 'invalid_session' } } }),
  );
  await page.getByLabel('Identificador del documento').fill(id);
  await page.getByRole('button', { name: 'Abrir documento' }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
});

test('owner enrollment and audit work in a mobile viewport', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await setup(page);
  await page.screenshot({ path: 'test-results/login-mobile.png', fullPage: true });
  await login(page, false, false);
  await navigate(page, 'Equipo');
  await page.getByLabel('Correo del nuevo usuario').fill('new@example.com');
  await page.getByLabel('Contrase\u00f1a inicial').fill('long-test-password');
  await page.getByRole('button', { name: 'Crear usuario' }).click();
  await expect(page.getByText('TESTSECRET', { exact: true })).toBeVisible();
  await page.getByLabel('Ya guard\u00e9 la clave y los c\u00f3digos').check();
  await page.getByRole('button', { name: 'Finalizar' }).click();
  await navigate(page, 'Auditor\u00eda');
  await page.getByRole('button', { name: 'Verificar cadena' }).click();
  await expect(page.getByText('12 eventos verificados')).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});
