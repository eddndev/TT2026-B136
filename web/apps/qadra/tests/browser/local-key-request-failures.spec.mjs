import { test, expect } from '@playwright/test';
import { login } from './helpers.mjs';
import {
  ownerCertificateSetup,
  checkOwnerCertificateRequests,
} from './owner-certificate-fixtures.mjs';
import { openOwnerCertificates } from './owner-certificate-ui.mjs';

test.afterEach(async ({ page }) => checkOwnerCertificateRequests(page));

async function beginGeneration(page) {
  const state = await ownerCertificateSetup(page);
  await login(page, false, false);
  await openOwnerCertificates(page);
  const form = page.getByRole('region', { name: 'Generar clave y solicitud', exact: true });
  await form.getByLabel('Nombre com\u00fan (CN)', { exact: true }).fill('Failure test holder');
  await form.getByLabel('Organizaci\u00f3n (O)', { exact: true }).fill('Despacho Juridico Demo');
  await form.getByLabel('Pa\u00eds (C)', { exact: true }).fill('MX');
  await form
    .getByLabel('Contrase\u00f1a de la nueva clave', { exact: true })
    .fill('Disposable failure password 6813');
  await form
    .getByLabel('Confirmar contrase\u00f1a', { exact: true })
    .fill('Disposable failure password 6813');
  await form.getByRole('button', { name: 'Generar clave y solicitud', exact: true }).click();
  return { form, state };
}

async function expectSafeFailure(form, state, message) {
  await expect(form.getByRole('alert')).toContainText(message);
  await expect(form.getByLabel('Contrase\u00f1a de la nueva clave', { exact: true })).toHaveValue(
    '',
  );
  await expect(form.getByLabel('Confirmar contrase\u00f1a', { exact: true })).toHaveValue('');
  await expect(
    form.getByRole('button', { name: 'Generar clave y solicitud', exact: true }),
  ).toBeEnabled();
  await expect(
    form.getByRole('button', { name: 'Descargar clave cifrada', exact: true }),
  ).toHaveCount(0);
  await expect(
    form.getByRole('button', { name: 'Descargar solicitud CSR', exact: true }),
  ).toHaveCount(0);
  expect(state.ownerPreparations).toEqual([]);
  expect(state.ownerWrites).toEqual([]);
}

test('WASM loading failure explains unavailable local cryptography without creating a request', async ({
  page,
}) => {
  let aborted = false;
  await page.route(
    (url) =>
      /browser_crypto_bg(?:-[\w-]+)?\.wasm$/.test(url.pathname) &&
      !url.searchParams.has('import') &&
      !url.searchParams.has('url'),
    async (route) => {
      aborted = true;
      await route.abort('failed');
    },
  );
  const { form, state } = await beginGeneration(page);
  await expect.poll(() => aborted, { message: 'The actual WASM fetch must be aborted' }).toBe(true);
  await expectSafeFailure(
    form,
    state,
    /No se pudo (?:cargar|iniciar) la criptograf(?:i|\u00ed)a local/,
  );
});

test('unavailable secure randomness explains the capability failure and releases password inputs', async ({
  page,
}) => {
  let instrumented = false;
  await page.route('**/local-crypto-worker.mjs*', async (route) => {
    const response = await route.fetch();
    const body = await response.text();
    instrumented = true;
    await route.fulfill({
      response,
      body: `Object.defineProperty(self.crypto, 'getRandomValues', {
        value() { throw new DOMException('Synthetic entropy unavailable', 'OperationError'); }
      });\n${body}`,
    });
  });
  const { form, state } = await beginGeneration(page);
  await expect
    .poll(() => instrumented, { message: 'Only the local crypto Worker is instrumented' })
    .toBe(true);
  await expectSafeFailure(form, state, /fuente segura/);
});
