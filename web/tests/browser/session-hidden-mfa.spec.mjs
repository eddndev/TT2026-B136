import { test, expect } from '@playwright/test';
import { requestCompletion } from './request-completion.mjs';
import { sessionSetup } from './session-inactivity-helpers.mjs';
import {
  visibility,
  holdControl,
  finishVisibilityRequests,
} from './session-visibility-helpers.mjs';

const mfaPath = '/api/v1/auth/mfa/totp';
const sessionPath = '/api/v1/auth/session';
const mfaRequests = new WeakMap();
const business = (state) => state.calls.filter((call) => !call.path.startsWith('/api/v1/auth/'));

async function holdMfa(page) {
  const gate = { calls: [], release: () => {} };
  const pending = new Promise((resolve) => {
    gate.release = resolve;
  });
  mfaRequests.set(page, gate);
  await page.route(
    (url) => url.pathname === mfaPath,
    async (route) => {
      const request = route.request();
      gate.calls.push({
        method: request.method(),
        body: request.postDataJSON(),
        bearer: request.headers().authorization,
        search: new URL(request.url()).search,
      });
      await pending;
      return route.fallback();
    },
  );
  return gate;
}

test.afterEach(async ({ page }) => {
  const gate = mfaRequests.get(page);
  gate?.release();
  for (const call of gate?.calls || []) {
    expect(call).toEqual({
      method: 'POST',
      body: { challenge_token: 'challenge-1', code: '123456' },
      bearer: undefined,
      search: '',
    });
  }
  await finishVisibilityRequests(page);
});

test('MFA completed while hidden waits for a fresh visible session read before mounting business data', async ({
  page,
}) => {
  const state = await sessionSetup(page);
  const mfa = await holdMfa(page);
  await page.getByLabel('Correo electr\u00f3nico').fill('hatz@example.com');
  await page.getByLabel('Contrase\u00f1a', { exact: true }).fill('a-long-password');
  await page.getByRole('button', { name: 'Continuar', exact: true }).click();
  await page.getByLabel('C\u00f3digo de 6 d\u00edgitos', { exact: true }).fill('123456');
  const completed = requestCompletion(
    page,
    (request) => new URL(request.url()).pathname === mfaPath,
  );
  await page.getByRole('button', { name: 'Verificar y entrar', exact: true }).click();
  await expect.poll(() => mfa.calls.length).toBe(1);
  await visibility(page, 'hidden');
  mfa.release();
  expect((await completed).failed).toBe(false);
  await expect(page.getByRole('button', { name: 'Verificando...', exact: true })).toHaveCount(0);
  expect(state.grants).toHaveLength(1);
  await expect(page.locator('.app-layout')).toHaveCount(0);
  await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toHaveCount(0);
  expect(business(state)).toEqual([]);
  expect(state.activity()).toEqual([]);

  const check = await holdControl(page, sessionPath, state.current.token);
  await visibility(page, 'visible');
  await expect.poll(() => check.calls.length).toBe(1);
  await expect(page.locator('.app-layout')).toHaveCount(0);
  expect(business(state)).toEqual([]);
  expect(state.activity()).toEqual([]);
  check.release();
  await expect(page.locator('.app-layout')).not.toHaveAttribute('inert', '');
  await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
  await expect
    .poll(() =>
      business(state)
        .map((call) => call.path)
        .sort(),
    )
    .toEqual(['/api/v1/dashboard', '/api/v1/document-integrity-incidents']);
  await expect(
    page.getByRole('button', { name: 'Actualizar indicadores', exact: true }),
  ).toBeEnabled();
  await expect(
    page.getByRole('region', { name: 'Indicadores operativos' }).getByText('2', { exact: true }),
  ).toBeVisible();
  await expect(page.locator('.app-layout').getByRole('alert')).toHaveCount(0);
  const read = state.calls.findIndex((call) => call.path === sessionPath);
  expect(read).toBeGreaterThan(-1);
  for (const call of business(state)) {
    expect(state.calls.indexOf(call)).toBeGreaterThan(read);
    expect(call.method).toBe('GET');
    expect(call.headers.authorization).toBe(`Bearer ${state.current.token}`);
  }
  expect(state.activity()).toEqual([]);
});
