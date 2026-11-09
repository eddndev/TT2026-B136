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
  await expect(page.getByRole('heading', { name: 'Acepta tu invitaci\u00f3n' })).toBeVisible({
    timeout: 5000,
  });
  await expect(page).toHaveURL(/\/$/);
  expect(calls).toHaveLength(0);
  await page.getByLabel('Tu contrase\u00f1a', { exact: true }).fill(password);
  await page.getByRole('button', { name: 'Preparar segundo factor', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Confirma tu segundo factor' })).toBeVisible();
  await page.getByLabel('Ya guard\u00e9 mis c\u00f3digos de recuperaci\u00f3n').check();
  await page.getByLabel('Tu contrase\u00f1a', { exact: true }).fill(password);
  await page.getByLabel('C\u00f3digo de 6 d\u00edgitos', { exact: true }).fill('123456');
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
  await page.getByRole('button', { name: 'Ir a iniciar sesi\u00f3n', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho' })).toBeVisible();
});
