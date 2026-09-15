import { test, expect } from '@playwright/test';

const caseId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const otherId = 'cccccccc-cccc-4ccc-8ccc-cccccccccccc';
const id = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const caseRecord = { id: caseId, title: 'Defensa inicial', reference: 'NUC-123', created_by: id };
const doc = {
  id,
  case_id: caseId,
  name: 'acta.pdf',
  version: 1,
  digest: 'a'.repeat(64),
  sealed: false,
};

async function setup(page, role = 'owner') {
  const requests = [];
  await page.route('**/api/v1/**', async (route) => {
    const url = new URL(route.request().url());
    const path = url.pathname;
    requests.push({ path, search: url.search, method: route.request().method() });
    const user = { id, email: 'owner@example.com', role };
    if (path.endsWith('/login')) return route.fulfill({ json: { challenge_token: 'challenge' } });
    if (path.includes('/mfa/')) return route.fulfill({ json: { user, access_token: 'session' } });
    if (path.endsWith('/me')) return route.fulfill({ json: user });
    if (path.endsWith('/logout')) return route.fulfill({ status: 204 });
    if (path.endsWith('/cases'))
      return route.fulfill({
        json:
          route.request().method() === 'POST'
            ? caseRecord
            : [caseRecord, { ...caseRecord, id: otherId, title: 'Otro expediente' }],
      });
    if (path.endsWith(`/cases/${caseId}`)) return route.fulfill({ json: caseRecord });
    if (path.endsWith(`/cases/${otherId}`))
      return route.fulfill({ json: { ...caseRecord, id: otherId, title: 'Otro expediente' } });
    if (path.endsWith('/documents'))
      return route.fulfill({
        json: {
          documents: url.searchParams.get('name') ? [] : [doc],
          has_more: !url.searchParams.get('name') && url.searchParams.get('offset') === '0',
        },
      });
    if (path.endsWith(`/documents/${id}`)) return route.fulfill({ json: doc });
    if (path.includes('/members/')) return route.fulfill({ status: 204 });
    return route.fulfill({ status: 404 });
  });
  await page.goto('/');
  await page.getByLabel('Correo electr\u00f3nico').fill('owner@example.com');
  await page.getByLabel('Contrase\u00f1a', { exact: true }).fill('a-long-password');
  await page.getByRole('button', { name: 'Continuar', exact: true }).click();
  await page.getByLabel('C\u00f3digo de 6 d\u00edgitos').fill('123456');
  await page.getByRole('button', { name: 'Verificar y entrar' }).click();
  await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
  return requests;
}

async function navigate(page, name) {
  if (await page.getByRole('button', { name: 'Abrir men\u00fa', exact: true }).isVisible())
    await page.getByRole('button', { name: 'Abrir men\u00fa', exact: true }).click();
  await page.getByRole('navigation').getByRole('button', { name, exact: true }).click();
}

async function chooseCase(page) {
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
}

test('persistent case selection lists metadata and GET detail without verifying', async ({
  page,
}) => {
  const requests = await setup(page);
  await navigate(page, 'Expedientes');
  await expect(page.getByRole('button', { name: /Defensa inicial/ })).toBeVisible();
  await page.screenshot({
    path: 'test-results/cases-list-desktop.png',
    fullPage: true,
    animations: 'disabled',
  });
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
  await expect(page.getByRole('heading', { name: 'Documentos' })).toBeVisible();
  await expect(page.getByText('acta.pdf', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: /acta.pdf/ }).click();
  await expect(page.getByRole('heading', { name: 'acta.pdf' })).toBeVisible();
  expect(requests.some((r) => r.path.endsWith(`/documents/${id}`) && r.method === 'GET')).toBe(
    true,
  );
  expect(requests.some((r) => r.path.endsWith('/verify'))).toBe(false);
  await page.evaluate(() => scrollTo(0, 0));
  await page.screenshot({
    path: 'test-results/cases-document-desktop.png',
    fullPage: true,
    animations: 'disabled',
  });
});

test('document pagination and filters query server and clear old rows', async ({ page }) => {
  const requests = await setup(page);
  await chooseCase(page);
  await page.getByRole('button', { name: 'Siguiente', exact: true }).click();
  await expect.poll(() => requests.some((r) => r.search.includes('offset=50'))).toBe(true);
  await page.getByLabel('Buscar por nombre').fill('inexistente');
  await page.getByRole('button', { name: 'Buscar', exact: true }).click();
  await expect(page.getByText('No encontramos coincidencias')).toBeVisible();
  expect(
    requests.some((r) => r.search.includes('name=inexistente') && r.search.includes('offset=0')),
  ).toBe(true);
  await page.getByLabel('Filtrar por estado').selectOption('sealed');
  await expect.poll(() => requests.some((r) => r.search.includes('sealed=true'))).toBe(true);
});

test('client can select assigned case metadata but never requests documents', async ({ page }) => {
  const requests = await setup(page, 'client');
  await chooseCase(page);
  await expect(page.getByText('Acceso documental pendiente')).toBeVisible();
  expect(requests.some((r) => r.path.includes('/documents'))).toBe(false);
  await expect(page.getByRole('button', { name: 'Nuevo expediente' })).toHaveCount(0);
});

test('owner creates a persistent case; mobile layout preserves Qadra', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const requests = await setup(page);
  await navigate(page, 'Expedientes');
  await page.screenshot({
    path: 'test-results/cases-list-mobile.png',
    fullPage: true,
    animations: 'disabled',
  });
  await page.getByRole('button', { name: 'Nuevo expediente' }).click();
  await page.getByLabel('T\u00edtulo del expediente').fill('Defensa inicial');
  await page.getByLabel('Referencia del expediente').fill('NUC-123');
  await page.getByRole('button', { name: 'Crear expediente', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Documentos', exact: true })).toBeVisible();
  expect(requests.some((r) => r.path.endsWith('/cases') && r.method === 'POST')).toBe(true);
  const inputBox = await page.getByLabel('Buscar por nombre').boundingBox();
  const searchBox = await page.getByRole('button', { name: 'Buscar', exact: true }).boundingBox();
  const filterBox = await page.getByLabel('Filtrar por estado').boundingBox();
  expect(inputBox.width).toBeGreaterThanOrEqual(150);
  expect(inputBox.x + inputBox.width).toBeLessThanOrEqual(searchBox.x);
  expect(searchBox.y + searchBox.height).toBeLessThanOrEqual(filterBox.y);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({
    path: 'test-results/cases-mobile.png',
    fullPage: true,
    animations: 'disabled',
  });
});
