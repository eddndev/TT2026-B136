import { test, expect } from '@playwright/test';
import { setup, login, navigate, caseRecord } from './helpers.mjs';
import { administration, overview, profile } from './case-administration-helpers.mjs';
const deferred = () => {
  let resolve;
  const promise = new Promise((finish) => {
    resolve = finish;
  });
  return { promise, resolve };
};
async function summary(page) {
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await page.getByRole('button', { name: /Defensa inicial/ }).click();
}

test('a completed case navigation preserves search focus when its hashchange arrives', async ({
  page,
}) => {
  await setup(page);
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  const input = page.getByLabel('Buscar por t\u00edtulo');
  await input.fill('Nueva');
  await expect(input).toBeFocused();
  // The browser can deliver the hash event after a field in the new view receives focus.
  await page.evaluate(() => window.dispatchEvent(new HashChangeEvent('hashchange')));
  await expect(input).toBeFocused();
});

test('late staff index response cannot replace a newer filtered page', async ({ page }) => {
  await setup(page);
  const pending = deferred(),
    started = deferred();
  await page.route('**/api/v1/case-administrations?*', async (route) => {
    const title = new URL(route.request().url()).searchParams.get('title');
    if (!title) {
      started.resolve();
      await pending.promise;
    }
    await route.fulfill({
      json: {
        cases: [
          overview(
            administration({ ...caseRecord, title: title ? 'Nueva consulta' : 'Resultado viejo' }),
          ),
        ],
        has_more: false,
        next_after_id: null,
      },
    });
  });
  await login(page, false, false);
  await navigate(page, 'Expedientes');
  await started.promise;
  await page.getByLabel('Buscar por t\u00edtulo').fill('Nueva');
  // Submit permits superseding an in-flight read; only writes require a busy guard.
  await page.getByLabel('Buscar por t\u00edtulo').press('Enter');
  await expect(page.getByRole('button', { name: /Nueva consulta/ })).toBeVisible();
  const done = page.waitForEvent(
    'requestfinished',
    (request) =>
      request.url().includes('/case-administrations?') && !request.url().includes('title='),
  );
  pending.resolve();
  await done;
  await expect(page.getByRole('button', { name: /Resultado viejo/ })).toHaveCount(0);
});

test('late summary read cannot downgrade a confirmed administrative revision', async ({ page }) => {
  await setup(page);
  const pending = deferred(),
    started = deferred();
  let reads = 0;
  await page.route(`**/cases/${caseRecord.id}/administration`, async (route) => {
    if (route.request().method() === 'PUT')
      return route.fulfill({
        json: administration({ ...caseRecord, title: 'Cambio confirmado' }, 2, profile),
      });
    if (++reads === 2) {
      started.resolve();
      await pending.promise;
    }
    await route.fulfill({ json: administration(caseRecord, 1, profile) });
  });
  await summary(page);
  await page.getByRole('button', { name: 'Editar ficha penal', exact: true }).click();
  await page.getByLabel('T\u00edtulo del expediente', { exact: true }).fill('Cambio confirmado');
  await page.getByRole('button', { name: 'Actualizar resumen', exact: true }).click();
  await started.promise;
  await page.getByRole('button', { name: 'Guardar ficha penal', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Cambio confirmado', exact: true })).toBeVisible();
  const done = page.waitForEvent(
    'requestfinished',
    (request) => request.method() === 'GET' && request.url().endsWith('/administration'),
  );
  pending.resolve();
  await done;
  await expect(page.getByRole('heading', { name: 'Cambio confirmado', exact: true })).toBeVisible();
});

test('denied summary clears protected fields and rejects a pending edit response', async ({
  page,
}) => {
  await setup(page);
  const pending = deferred(),
    started = deferred();
  let reads = 0;
  await page.route(`**/cases/${caseRecord.id}/administration`, async (route) => {
    if (route.request().method() === 'PUT') {
      started.resolve();
      await pending.promise;
      return route.fulfill({ json: administration(caseRecord, 2, profile) });
    }
    if (++reads > 1)
      return route.fulfill({ status: 403, json: { error: { code: 'permission_denied' } } });
    return route.fulfill({ json: administration(caseRecord, 1, profile) });
  });
  await summary(page);
  await page.getByRole('button', { name: 'Editar ficha penal', exact: true }).click();
  await page.getByRole('button', { name: 'Guardar ficha penal', exact: true }).click();
  await started.promise;
  await page.getByRole('button', { name: 'Actualizar resumen', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Expediente no disponible', exact: true }),
  ).toBeVisible();
  const done = page.waitForEvent(
    'requestfinished',
    (request) => request.method() === 'PUT' && request.url().endsWith('/administration'),
  );
  pending.resolve();
  await done;
  await expect(page.locator('.case-summary, .case-editor')).toHaveCount(0);
  await expect(page.getByText('Fiscalia registrada', { exact: true })).toHaveCount(0);
});

test('abandoned case edit never selects its result inside another case', async ({ page }) => {
  await setup(page);
  const pending = deferred(),
    started = deferred();
  await page.route(`**/cases/${caseRecord.id}/administration`, async (route) => {
    if (route.request().method() === 'PUT') {
      started.resolve();
      await pending.promise;
    }
    return route.fulfill({ json: administration(caseRecord, 1, profile) });
  });
  await summary(page);
  await page.getByRole('button', { name: 'Editar ficha penal', exact: true }).click();
  await page.getByRole('button', { name: 'Guardar ficha penal', exact: true }).click();
  await started.promise;
  await page.getByRole('button', { name: 'Cambiar expediente', exact: true }).click();
  await page.getByRole('button', { name: /Otro expediente/ }).click();
  const done = page.waitForEvent(
    'requestfinished',
    (request) => request.method() === 'PUT' && request.url().endsWith('/administration'),
  );
  pending.resolve();
  await done;
  await expect(page.getByRole('heading', { name: 'Otro expediente', exact: true })).toBeVisible();
  await expect(page.getByText('Fiscalia registrada', { exact: true })).toHaveCount(0);
});
