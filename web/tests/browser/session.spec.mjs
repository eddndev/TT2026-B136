import { test, expect } from '@playwright/test';

const id = '78ac67b1-ab36-49ea-9b08-f951f341f081';

async function prepare(page) {
  let sequence = 0;
  await page.route('**/api/v1/**', (route) => {
    const path = new URL(route.request().url()).pathname;
    const user = { id: 'user', email: 'person@example.com', role: 'owner' };
    if (path.endsWith('/login')) return route.fulfill({ json: { challenge_token: 'challenge' } });
    if (path.endsWith('/mfa/totp'))
      return route.fulfill({
        json: {
          access_token: `token-${++sequence}`,
          user,
        },
      });
    if (path.endsWith('/me')) return route.fulfill({ json: user });
    if (path.endsWith('/logout')) return route.fulfill({ status: 204 });
    return route.fulfill({ status: 404 });
  });
  await page.goto('/');
}

async function enter(page) {
  await page.getByLabel('Correo electronico').fill('person@example.com');
  await page.getByLabel('Contrasena', { exact: true }).fill('long-test-password');
  await page.getByRole('button', { name: 'Continuar', exact: true }).click();
  await page.getByLabel('Codigo de 6 digitos').fill('123456');
  await page.getByRole('button', { name: 'Verificar y entrar' }).click();
  await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
}

test('a document response after logout does not enter the next session', async ({ page }) => {
  await prepare(page);
  await enter(page);
  let release;
  await page.route(
    '**/documents/*/verify',
    (route) =>
      new Promise((resolve) => {
        release = async () => {
          await route.fulfill({ json: { document_digest: 'a'.repeat(64), verdict: 'valid' } });
          resolve();
        };
      }),
  );
  await page
    .getByRole('navigation')
    .getByRole('button', { name: 'Documentos', exact: true })
    .click();
  await page.getByLabel('Identificador del documento').fill(id);
  await page.getByRole('button', { name: 'Abrir documento' }).click();
  await expect.poll(() => typeof release).toBe('function');
  await page.getByRole('button', { name: 'Cerrar sesion' }).click();
  await enter(page);
  const finished = page.waitForEvent('requestfinished', (request) =>
    request.url().endsWith('/verify'),
  );
  await release();
  await finished;
  await page.evaluate(
    () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))),
  );
  await expect(page.getByRole('button', { name: /Documentos abiertos/ })).toContainText('0');
  await page
    .getByRole('navigation')
    .getByRole('button', { name: 'Documentos', exact: true })
    .click();
  await expect(page.getByText('Tu archivo empieza con el primer documento')).toBeVisible();
});

test('returning to all documents consumes the previous upload or filter intent', async ({
  page,
}) => {
  await prepare(page);
  await enter(page);
  await page.getByRole('button', { name: 'Subir documento', exact: true }).click();
  await page.getByRole('button', { name: 'Cancelar', exact: true }).click();
  await page.goBack();
  await page.getByRole('button', { name: 'Ver todos', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'Subir documento' })).not.toBeVisible();
  await page.goBack();
  await page.getByRole('button', { name: /Pendientes de sello/ }).click();
  await expect(page.getByLabel('Filtrar por estado')).toHaveValue('pending');
  await page.goBack();
  await page.getByRole('button', { name: 'Ver todos', exact: true }).click();
  await expect(page.getByLabel('Filtrar por estado')).toHaveValue('all');
});
