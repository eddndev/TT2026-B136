import { test, expect } from '@playwright/test';

const token = Buffer.alloc(32, 90).toString('base64url');
const secret = 'a new private password';

async function wire(page, reply = 204) {
  const calls = [];
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request();
    calls.push({ url: request.url(), headers: request.headers(), body: request.postDataJSON() });
    if (reply === 'disconnect') return route.abort();
    return route.fulfill({
      status: reply,
      contentType: 'application/json',
      body: reply === 204 ? '' : JSON.stringify({ status: 'accepted' }),
    });
  });
  return calls;
}

test('request gives the same neutral acknowledgement without logging in', async ({ page }) => {
  const calls = await wire(page, 202);
  await page.goto('/');
  await page.getByRole('button', { name: 'Olvid\u00e9 mi contrase\u00f1a', exact: true }).click();
  await page.getByLabel('Correo electr\u00f3nico', { exact: true }).fill('member@example.test');
  await page.getByRole('button', { name: 'Solicitar enlace', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Si la solicitud puede atenderse');
  expect(calls).toHaveLength(1);
  expect(calls[0].url).toMatch(/\/auth\/password-reset\/request$/);
  expect(calls[0].body).toEqual({ email: 'member@example.test' });
  expect(calls[0].headers.authorization).toBeUndefined();
  await expect(page.getByRole('button', { name: 'Solicitar enlace', exact: true })).toHaveCount(0);
});

test('a link is removed before confirmation and never submits automatically', async ({ page }) => {
  const calls = await wire(page);
  await page.goto(`/#password-reset=${token}`);
  await expect(
    page.getByRole('heading', { name: 'Elige una nueva contrase\u00f1a.' }),
  ).toBeVisible();
  await expect(page).toHaveURL(/\/$/);
  expect(calls).toHaveLength(0);
  await page.getByLabel('Nueva contrase\u00f1a', { exact: true }).fill(secret);
  await page.getByLabel('Repite la nueva contrase\u00f1a', { exact: true }).fill(secret);
  await page.getByRole('button', { name: 'Cambiar contrase\u00f1a', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('Contrase\u00f1a actualizada');
  expect(calls).toHaveLength(1);
  expect(calls[0].body).toEqual({ token, new_password: secret });
  expect(calls[0].headers.authorization).toBeUndefined();
  expect(
    await page.evaluate(() =>
      JSON.stringify([Object.entries(localStorage), Object.entries(sessionStorage)]),
    ),
  ).not.toContain(token);
  await expect(page.locator('input[type="password"]')).toHaveCount(0);
  await expect(page.getByText('Conserva tu segundo factor')).toBeVisible();
});

test('an uncertain completion clears secrets and never offers an automatic replay', async ({
  page,
}) => {
  const calls = await wire(page, 'disconnect');
  await page.goto(`/#password-reset=${token}`);
  await page.getByLabel('Nueva contrase\u00f1a', { exact: true }).fill(secret);
  await page.getByLabel('Repite la nueva contrase\u00f1a', { exact: true }).fill(secret);
  await page.getByRole('button', { name: 'Cambiar contrase\u00f1a', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('No pudimos confirmar');
  expect(calls).toHaveLength(1);
  await expect(
    page.getByRole('button', { name: 'Cambiar contrase\u00f1a', exact: true }),
  ).toHaveCount(0);
  await expect(page.locator('input[type="password"]')).toHaveCount(0);
  await page.getByRole('button', { name: 'Volver al inicio de sesi\u00f3n', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  expect(calls).toHaveLength(1);
});

test('a malformed link is removed and cannot call completion', async ({ page }) => {
  const calls = await wire(page);
  await page.goto('/#password-reset=invalid');
  await expect(page.getByRole('alert')).toContainText('enlace no es v\u00e1lido');
  await expect(page).toHaveURL(/\/$/);
  await expect(page.locator('input[type="password"]')).toHaveCount(0);
  expect(calls).toHaveLength(0);
});

test('password mismatch and byte bounds are checked before submitting', async ({ page }) => {
  const calls = await wire(page);
  await page.goto(`/#password-reset=${token}`);
  await page.getByLabel('Nueva contrase\u00f1a', { exact: true }).fill(secret);
  await page
    .getByLabel('Repite la nueva contrase\u00f1a', { exact: true })
    .fill('a different private password');
  await page.getByRole('button', { name: 'Cambiar contrase\u00f1a', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('no coinciden');
  expect(calls).toHaveLength(0);
  await page.getByLabel('Nueva contrase\u00f1a', { exact: true }).fill('short');
  await page.getByLabel('Repite la nueva contrase\u00f1a', { exact: true }).fill('short');
  await page.getByRole('button', { name: 'Cambiar contrase\u00f1a', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('12 y 1024 bytes');
  expect(calls).toHaveLength(0);
});

test('request transport errors never display provider or account details', async ({ page }) => {
  const calls = await wire(page, 503);
  await page.goto('/');
  await page.getByRole('button', { name: 'Olvid\u00e9 mi contrase\u00f1a', exact: true }).click();
  await page.getByLabel('Correo electr\u00f3nico', { exact: true }).fill('member@example.test');
  await page.getByRole('button', { name: 'Solicitar enlace', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('no est\u00e1 disponible');
  expect(calls).toHaveLength(1);
});

test('a link opened after sign-in screen mount replaces navigation immediately', async ({
  page,
}) => {
  const calls = await wire(page);
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  await page.evaluate((value) => {
    location.hash = `#password-reset=${value}`;
  }, token);
  await expect(page).toHaveURL(/\/$/);
  await expect(
    page.getByRole('heading', { name: 'Elige una nueva contrase\u00f1a.' }),
  ).toBeVisible();
  expect(calls).toHaveLength(0);
});

test('a replacement link owns its token and ignores the prior completion response', async ({
  page,
}) => {
  const replacement = Buffer.alloc(32, 70).toString('base64url');
  let release;
  const held = new Promise((resolve) => {
    release = resolve;
  });
  const calls = [];
  await page.route('**/api/v1/auth/password-reset/complete', async (route) => {
    calls.push(route.request().postDataJSON());
    if (calls.length === 1) await held;
    await route.fulfill({ status: 204 });
  });
  try {
    await page.goto(`/#password-reset=${token}`);
    await page.getByLabel('Nueva contrase\u00f1a', { exact: true }).fill(secret);
    await page.getByLabel('Repite la nueva contrase\u00f1a', { exact: true }).fill(secret);
    await page.getByRole('button', { name: 'Cambiar contrase\u00f1a', exact: true }).click();
    await expect.poll(() => calls.length).toBe(1);
    await page.evaluate((value) => {
      location.hash = `#password-reset=${value}`;
    }, replacement);
    await expect(page).toHaveURL(/\/$/);
    await expect(page.getByLabel('Nueva contrase\u00f1a', { exact: true })).toBeEnabled();
    release();
    await page.getByLabel('Nueva contrase\u00f1a', { exact: true }).fill(secret);
    await page.getByLabel('Repite la nueva contrase\u00f1a', { exact: true }).fill(secret);
    await page.getByRole('button', { name: 'Cambiar contrase\u00f1a', exact: true }).click();
    await expect(page.getByRole('status')).toContainText('Contrase\u00f1a actualizada');
    expect(calls.map((call) => call.token)).toEqual([token, replacement]);
  } finally {
    release();
  }
});
