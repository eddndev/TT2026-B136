import { test, expect } from '@playwright/test';
const token = Buffer.alloc(32, 7).toString('base64url');
const password = 'the recipient password';
const id = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const enrollment = {
  id,
  revision: '1',
  operation_id: id,
  email: 'recipient@example.test',
  role: 'litigator',
  expires_at: '2099-01-01T00:00:00Z',
  totp_secret_base32: 'JBSWY3DPEHPK3PXP',
  otpauth_uri: 'otpauth://totp/Qadra:test?secret=JBSWY3DPEHPK3PXP',
  recovery_codes: ['recovery-one', 'recovery-two'],
};
const receipt = {
  id,
  revision: '1',
  operation_id: id,
  user_id: id,
  email: enrollment.email,
  role: 'litigator',
  accepted_at: '2026-10-11T00:00:00Z',
};

test('recipient confirms real MFA step and receives no session before normal login', async ({
  page,
}) => {
  const calls = [];
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request();
    calls.push({ url: request.url(), headers: request.headers(), data: request.postDataJSON() });
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(request.url().endsWith('/prepare') ? enrollment : receipt),
    });
  });
  await page.goto(`/#invitation=${token}`);
  await expect(page.getByRole('heading', { name: 'Acepta tu invitacion' })).toBeVisible({
    timeout: 5000,
  });
  await expect(page).toHaveURL(/\/$/);
  expect(calls).toHaveLength(0);
  await page.getByLabel('Tu contrasena', { exact: true }).fill(password);
  await page.getByRole('button', { name: 'Preparar segundo factor', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Confirma tu segundo factor' })).toBeVisible();
  await page.getByLabel('Ya guarde mis codigos de recuperacion').check();
  await page.getByLabel('Tu contrasena', { exact: true }).fill(password);
  await page.getByLabel('Codigo de 6 digitos', { exact: true }).fill('123456');
  await page.getByRole('button', { name: 'Confirmar alta', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Cuenta creada');
  expect(calls).toHaveLength(2);
  expect(calls.every((c) => !c.headers.authorization)).toBe(true);
  const stored = await page.evaluate(() =>
    JSON.stringify([Object.entries(localStorage), Object.entries(sessionStorage)]),
  );
  for (const secret of [token, password, enrollment.totp_secret_base32, 'recovery-one'])
    expect(stored).not.toContain(secret);
  await expect(page.locator('input[type="password"]')).toHaveCount(0);
  await page.getByRole('button', { name: 'Ir a iniciar sesion', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
});

test('lost completion is checked by receipt without sending the completion twice', async ({
  page,
}) => {
  const calls = [];
  await page.route('**/api/v1/auth/invitations/**', async (route) => {
    const request = route.request(),
      action = request.url().split('/').at(-1);
    calls.push({ action, data: request.postDataJSON() });
    if (action === 'finish') return route.abort();
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(action === 'prepare' ? enrollment : { receipt }),
    });
  });
  await page.goto(`/#invitation=${token}`);
  await page.getByLabel('Tu contrasena', { exact: true }).fill(password);
  await page.getByRole('button', { name: 'Preparar segundo factor', exact: true }).click();
  await page.getByLabel('Ya guarde mis codigos de recuperacion').check();
  await page.getByLabel('Tu contrasena', { exact: true }).fill(password);
  await page.getByLabel('Codigo de 6 digitos', { exact: true }).fill('123456');
  await page.getByRole('button', { name: 'Confirmar alta', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Comprobar resultado' })).toBeVisible();
  await expect(page.getByLabel('Tu contrasena', { exact: true })).toHaveValue('');
  await page.reload();
  await page.goto(`/#invitation=${token}`);
  await expect(page.getByRole('button', { name: 'Comprobar resultado' })).toBeVisible();
  expect(calls.map((c) => c.action)).toEqual(['prepare', 'finish']);
  await page.getByLabel('Tu contrasena', { exact: true }).fill(password);
  await page.getByRole('button', { name: 'Comprobar resultado' }).click();
  await expect(page.getByRole('status')).toContainText('Cuenta creada');
  expect(calls.map((c) => c.action)).toEqual(['prepare', 'finish', 'receipt']);
  expect(calls[2].data.operation_id).toBe(calls[1].data.operation_id);
});

test('a delayed enrollment response is discarded when a new invitation replaces it', async ({
  page,
}) => {
  let release;
  const waiting = new Promise((resolve) => {
    release = resolve;
  });
  await page.route('**/api/v1/auth/invitations/prepare', async (route) => {
    await waiting;
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(enrollment),
    });
  });
  await page.goto(`/#invitation=${token}`);
  await page.getByLabel('Tu contrasena', { exact: true }).fill(password);
  const request = page.waitForRequest('**/api/v1/auth/invitations/prepare');
  await page.getByRole('button', { name: 'Preparar segundo factor', exact: true }).click();
  await request;
  await page.evaluate((next) => {
    location.hash = `invitation=${next}`;
  }, Buffer.alloc(32, 8).toString('base64url'));
  await expect(
    page.getByRole('button', { name: 'Preparar segundo factor', exact: true }),
  ).toBeEnabled();
  release();
  await expect(page.getByRole('heading', { name: 'Acepta tu invitacion' })).toBeVisible();
  await expect(page.getByText(enrollment.totp_secret_base32, { exact: true })).toHaveCount(0);
  await expect(page.getByLabel('Tu contrasena', { exact: true })).toHaveValue('');
});
