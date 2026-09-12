import { test, expect } from '@playwright/test';

const id = '78ac67b1-ab36-49ea-9b08-f951f341f081';
const document = { id, name: 'contrato.pdf', version: 1, digest: 'a'.repeat(64), sealed: false };
const component = { status: 'passed', detail: 'Verified by test backend' };

test.beforeEach(async ({ page }) => {
  page.runtimeErrors = [];
  page.on('pageerror', (error) => page.runtimeErrors.push(error.message));
});
test.afterEach(async ({ page }) => expect(page.runtimeErrors).toEqual([]));

async function setup(page, role = 'owner') {
  const requests = [];
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    requests.push({
      path,
      method: request.method(),
      headers: request.headers(),
      body: request.postData(),
    });
    const user = { id: 'user', email: 'hatz@example.com', role };
    let body;
    if (path.endsWith('/login')) body = { challenge_token: 'challenge', expires_in_seconds: 300 };
    else if (/\/mfa\//.test(path))
      body = { access_token: 'test-token', user, expires_in_seconds: 86400 };
    else if (path.endsWith('/me')) body = user;
    else if (path.endsWith('/logout')) return route.fulfill({ status: 204 });
    else if (path.endsWith('/seal')) body = { ...document, sealed: true };
    else if (path.endsWith('/audit/verify'))
      body = { valid: true, entries: 12, first_broken_index: null };
    else if (path.endsWith('/verify'))
      body = {
        document_digest: document.digest,
        verdict: 'valid',
        integrity: component,
        signature: component,
        certificate: component,
        timestamp: component,
      };
    else if (path.endsWith('/evidence'))
      return route.fulfill({ contentType: 'application/zip', body: 'test archive' });
    else if (path.endsWith('/users') || path.endsWith('/bootstrap'))
      body = {
        user,
        totp_secret_base32: 'TESTSECRET',
        otpauth_uri: 'otpauth://totp/test',
        recovery_codes: ['recovery-one', 'recovery-two'],
      };
    else if (path.endsWith('/documents')) body = document;
    else return route.fulfill({ status: 404 });
    return route.fulfill({ json: body });
  });
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  return requests;
}

async function navigate(page, name) {
  if (await page.getByRole('button', { name: 'Abrir menu', exact: true }).isVisible()) {
    await page.getByRole('button', { name: 'Abrir menu', exact: true }).click();
  }
  await page.getByRole('navigation').getByRole('button', { name, exact: true }).click();
}

async function login(page, recovery = false, openDocuments = true) {
  await page.getByLabel('Correo electronico').fill('hatz@example.com');
  await page.getByLabel('Contrasena', { exact: true }).fill('a-long-password');
  await page.getByRole('button', { name: 'Continuar', exact: true }).click();
  if (recovery) await page.getByRole('button', { name: 'Usar codigo de recuperacion' }).click();
  await page
    .getByLabel(recovery ? 'Codigo de recuperacion' : 'Codigo de 6 digitos', { exact: true })
    .fill(recovery ? 'recovery-one' : '123456');
  await page.getByRole('button', { name: 'Verificar y entrar' }).click();
  await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
  if (openDocuments) await navigate(page, 'Documentos');
}

test('overview uses session counts and browser navigation without losing the session', async ({
  page,
}) => {
  await setup(page);
  await login(page, false, false);
  await expect(page.getByRole('region', { name: 'Resumen de esta sesion' })).toContainText(
    'Documentos abiertos',
  );
  await page.getByRole('button', { name: 'Subir documento', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Subir documento' })).toBeVisible();
  await page.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await page.goBack();
  await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
  await page.getByRole('button', { name: 'Como funciona' }).click();
  await expect(
    page.getByRole('heading', { name: 'Un recorrido claro, de principio a fin' }),
  ).toBeVisible();
});

test('an existing unsealed UUID opens with an actionable seal step', async ({ page }) => {
  await setup(page);
  await login(page);
  await page.route('**/documents/*/verify', (route) =>
    route.fulfill({ status: 409, json: { error: { code: 'document_not_sealed' } } }),
  );
  await page.getByLabel('Identificador del documento').fill(id);
  await page.getByRole('button', { name: 'Abrir documento' }).click();
  await expect(page.getByRole('heading', { name: 'Documento por identificador' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Sellar documento', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Verificar integridad' })).toBeDisabled();
});

test('upload normalizes the filename and cards, filters and overview reflect the session', async ({
  page,
}) => {
  const requests = await setup(page);
  await login(page);
  await page.getByRole('button', { name: 'Subir documento', exact: true }).click();
  await page.getByLabel('Archivo', { exact: true }).setInputFiles({
    name: 'Dem\u00e1nda inicial.pdf',
    mimeType: 'application/pdf',
    buffer: Buffer.from('evidence'),
  });
  await expect(page.getByLabel('Nombre del documento')).toHaveValue('Demanda-inicial.pdf');
  await page.getByRole('button', { name: 'Cargar documento' }).click();
  await expect(page.getByRole('heading', { name: 'contrato.pdf' })).toBeVisible();
  expect(requests.find((item) => item.path.endsWith('/documents')).headers['x-document-name']).toBe(
    'Demanda-inicial.pdf',
  );
  await page.getByRole('button', { name: 'Vista de tarjetas' }).click();
  await expect(page.locator('.document-card')).toHaveCount(1);
  await page.getByLabel('Filtrar por estado').selectOption('verified');
  await expect(page.getByText('No encontramos coincidencias')).toBeVisible();
  await page.getByRole('button', { name: 'Limpiar filtros' }).click();
  await expect(page.locator('.document-card')).toHaveCount(1);
  await navigate(page, 'Inicio');
  await expect(page.getByRole('button', { name: /Documentos abiertos/ })).toContainText('1');
  await expect(page.getByRole('button', { name: /Pendientes de sello/ })).toContainText('1');
  await page.screenshot({ path: 'test-results/overview-desktop.png', fullPage: true });
});

test('mobile menu supports Escape and password visibility is explicit', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await setup(page);
  await page.getByRole('button', { name: 'Mostrar contrasena' }).click();
  await expect(page.getByLabel('Contrasena', { exact: true })).toHaveAttribute('type', 'text');
  await page.getByRole('button', { name: 'Ocultar contrasena' }).click();
  await login(page, false, false);
  await page.getByRole('button', { name: 'Abrir menu' }).click();
  await expect(page.getByRole('dialog', { name: 'Menu del despacho' })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog', { name: 'Menu del despacho' })).not.toBeVisible();
  await expect(page.getByRole('button', { name: 'Abrir menu' })).toBeFocused();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: 'test-results/overview-mobile.png', fullPage: true });
});

test('initial enrollment is acknowledged before returning to login', async ({ page }) => {
  await setup(page);
  await page.getByRole('button', { name: 'Configurar acceso inicial' }).click();
  await page.getByLabel('Correo electronico').fill('hatz@example.com');
  await page.getByLabel('Contrasena', { exact: true }).fill('long-test-password');
  await page.getByRole('button', { name: 'Crear administrador' }).click();
  await expect(page.getByRole('button', { name: 'Finalizar' })).toBeDisabled();
  await page.getByLabel('Ya guarde el secreto y los codigos').check();
  await page.getByRole('button', { name: 'Finalizar' }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  expect(await page.evaluate(() => document.body.textContent.includes('TESTSECRET'))).toBe(false);
});

test('a failed verification never leaves a prior successful verdict visible', async ({ page }) => {
  await setup(page);
  await login(page);
  await page.getByLabel('Identificador del documento').fill(id);
  await page.getByRole('button', { name: 'Abrir documento' }).click();
  await expect(page.getByText('Verificacion valida', { exact: true })).toBeVisible();
  await page.route('**/documents/*/verify', (route) =>
    route.fulfill({ status: 500, json: { error: { code: 'internal' } } }),
  );
  await page.getByRole('button', { name: 'Verificar integridad' }).click();
  await expect(page.getByRole('alert')).toContainText('servidor');
  await expect(page.getByText('Verificacion valida', { exact: true })).toHaveCount(0);
});

test('reopening the same UUID replaces its previous verification verdict', async ({ page }) => {
  await setup(page);
  await login(page);
  const reference = page.locator('.reference-panel');
  await reference.getByLabel('Identificador del documento').fill(id);
  await reference.getByRole('button', { name: 'Abrir documento' }).click();
  await expect(page.getByText('Verificacion valida', { exact: true })).toBeVisible();
  await page.route('**/documents/*/verify', (route) =>
    route.fulfill({
      json: {
        document_digest: document.digest,
        verdict: 'not_valid',
        integrity: { status: 'failed', detail: 'The stored digest does not match' },
        signature: component,
        certificate: component,
        timestamp: component,
      },
    }),
  );
  await reference.getByRole('button', { name: 'Abrir documento' }).click();
  await expect(page.getByText('Verificacion no valida', { exact: true })).toBeVisible();
  await expect(page.getByText('Verificacion valida', { exact: true })).toHaveCount(0);
  await page.route('**/documents/*/verify', (route) =>
    route.fulfill({ status: 500, json: { error: { code: 'internal_error' } } }),
  );
  await reference.getByRole('button', { name: 'Abrir documento' }).click();
  await expect(reference.getByRole('alert')).toContainText('servidor');
  await expect(page.getByText('Verificacion no valida', { exact: true })).toHaveCount(0);
  await expect(page.getByText('Verificacion valida', { exact: true })).toHaveCount(0);
  await expect(page.getByText('Sin verificacion vigente', { exact: true })).toBeVisible();
});

test('a broken audit shows index zero and clears the verdict when a recheck fails', async ({
  page,
}) => {
  await setup(page);
  await login(page);
  await page.getByRole('button', { name: 'Auditoria', exact: true }).click();
  await page.route('**/audit/verify', (route) =>
    route.fulfill({ json: { valid: false, entries: null, first_broken_index: 0 } }),
  );
  await page.getByRole('button', { name: 'Verificar cadena' }).click();
  await expect(page.getByText('Primer indice roto: 0')).toBeVisible();
  await page.route('**/audit/verify', (route) => route.fulfill({ status: 502 }));
  await page.getByRole('button', { name: 'Verificar cadena' }).click();
  await expect(page.getByRole('alert')).toContainText('servidor');
  await expect(page.getByText('Primer indice roto: 0')).toHaveCount(0);
});

test('login, upload, seal, verify, download and logout follow the HTTP contract', async ({
  page,
}) => {
  const requests = await setup(page);
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
  await expect(page.getByText('Verificacion valida', { exact: true })).toBeVisible();
  const downloaded = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Descargar evidencia' }).click();
  expect((await downloaded).suggestedFilename()).toBe(`evidencia-${id}.zip`);
  const upload = requests.find((item) => item.path.endsWith('/documents'));
  expect(upload.headers['x-document-name']).toBe('contrato.pdf');
  expect(upload.headers.authorization).toBe('Bearer test-token');
  expect(upload.body).toBe('document bytes');
  await page.evaluate(() => scrollTo(0, 0));
  await page.screenshot({ path: 'test-results/document-desktop.png', fullPage: true });
  await page.getByRole('button', { name: 'Cerrar sesion' }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  expect(await page.evaluate(() => Object.keys(localStorage))).toEqual([]);
});

test('paralegal cannot seal; client cannot access document controls', async ({ page }) => {
  await setup(page, 'paralegal');
  await login(page, true);
  await page.getByLabel('Identificador del documento').fill(id);
  await page.getByRole('button', { name: 'Abrir documento' }).click();
  await expect(page.getByRole('button', { name: 'Sellar documento', exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Equipo', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Cerrar sesion' }).click();
  await page.unrouteAll();
  await setup(page, 'client');
  await login(page);
  await expect(page.getByRole('button', { name: 'Subir documento', exact: true })).toHaveCount(0);
  await expect(page.getByText('Acceso documental pendiente')).toBeVisible();
});

test('rejected MFA requires new credentials; expired session clears private content', async ({
  page,
}) => {
  await setup(page);
  await page.route('**/auth/mfa/totp', (route) =>
    route.fulfill({ status: 401, json: { error: { code: 'mfa_rejected' } } }),
  );
  await page.getByLabel('Correo electronico').fill('hatz@example.com');
  await page.getByLabel('Contrasena', { exact: true }).fill('a-long-password');
  await page.getByRole('button', { name: 'Continuar', exact: true }).click();
  await page.getByLabel('Codigo de 6 digitos', { exact: true }).fill('123456');
  await page.getByRole('button', { name: 'Verificar y entrar' }).click();
  await expect(page.getByRole('alert')).toContainText('rechazado');
  await expect(page.getByLabel('Contrasena', { exact: true })).toBeVisible();
  await page.unroute('**/auth/mfa/totp');
  await login(page);
  await page.route('**/documents/*/verify', (route) =>
    route.fulfill({ status: 401, json: { error: { code: 'invalid_session' } } }),
  );
  await page.getByLabel('Identificador del documento').fill(id);
  await page.getByRole('button', { name: 'Abrir documento' }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
});

test('owner enrollment and audit work; mobile navigation fits the viewport', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await setup(page);
  await page.screenshot({ path: 'test-results/login-mobile.png', fullPage: true });
  await login(page);
  await navigate(page, 'Equipo');
  await page.getByLabel('Correo del nuevo usuario').fill('new@example.com');
  await page.getByLabel('Contrasena temporal').fill('long-test-password');
  await page.getByRole('button', { name: 'Crear usuario' }).click();
  await expect(page.getByText('TESTSECRET', { exact: true })).toBeVisible();
  await page.getByLabel('Ya guarde el secreto y los codigos').check();
  await page.getByRole('button', { name: 'Finalizar' }).click();
  await navigate(page, 'Auditoria');
  await page.getByRole('button', { name: 'Verificar cadena' }).click();
  await expect(page.getByText('12 eventos verificados')).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: 'test-results/audit-mobile.png', fullPage: true });
});
