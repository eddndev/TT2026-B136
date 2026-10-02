import { expect } from '@playwright/test';
import { caseId, id, document, setup, openDocument } from './helpers.mjs';

const owner = {
  id: '11111111-1111-4111-8111-111111111111',
  email: 'hatz@example.com',
  role: 'owner',
};
const other = {
  id: '22222222-2222-4222-8222-222222222222',
  email: 'other@example.test',
  role: 'owner',
};
const epoch = Date.UTC(2026, 9, 2, 18);
const idleMilliseconds = 40000;
export const classification = '  Clasificacion todavia sin guardar  ';
export const pendingTag = '  etiqueta sin confirmar  ';
const sessions = new WeakMap();
export const metadataPath = `/api/v1/cases/${caseId}/documents/${id}/metadata`;
export const casePath = `/api/v1/cases/${caseId}/administration`;
export const documentsPath = `/api/v1/cases/${caseId}/documents`;
export const editorName = 'Editar clasificaci\u00f3n';
export const classificationLabel = 'Clasificaci\u00f3n (opcional)';

export async function sessionSetup(page) {
  await page.clock.install({ time: new Date(epoch) });
  await page.clock.pauseAt(new Date(epoch + 1000));

  await setup(page, 'owner', [
    {
      ...document,
      current_metadata: {
        metadata_revision: 1,
        document_type: 'Escrito',
        classification: 'Civil',
        tags: ['guardada'],
      },
    },
  ]);
  const state = {
    now: epoch + 1000,
    calls: [],
    unexpected: [],
    writes: [],
    grants: [],
    current: null,
    challenge: null,
    allowed: true,
    gate: null,
  };
  sessions.set(page, state);
  const reads = new Set([
    '/api/v1/dashboard',
    '/api/v1/document-integrity-incidents',
    '/api/v1/case-administrations',
    casePath,
    `${casePath}/history`,
    documentsPath,
    `${documentsPath}/${id}`,
    metadataPath,
    `${metadataPath}/history`,
    `${documentsPath}/${id}/versions`,
    `${documentsPath}/${id}/versions/1`,
  ]);
  const metadata = () => ({
    user: state.current.user,
    policy: { absolute_ttl_seconds: 86400, idle_ttl_seconds: idleMilliseconds / 1000 },
    server_now_unix_ms: state.now,
    absolute_expires_at_unix_ms: state.current.absolute,
    idle_expires_at_unix_ms: state.current.deadline,
  });
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    const call = {
      path: url.pathname,
      method: request.method(),
      body: request.postData(),
      headers: request.headers(),
      search: url.search,
      at: state.now,
    };
    state.calls.push(call);
    const reply = (json, status = 200) =>
      route.fulfill({
        status,
        json,
        headers: { 'Cache-Control': 'no-store' },
      });
    if (call.path === '/api/v1/auth/login' && call.method === 'POST') {
      const credentials = request.postDataJSON();
      const user = [owner, other].find((account) => account.email === credentials.email);
      if (!user || credentials.password !== 'a-long-password')
        return reply({ error: { code: 'invalid_credentials' } }, 401);
      state.challenge = { user, token: `challenge-${state.grants.length + 1}` };
      return reply({ challenge_token: state.challenge.token, expires_in_seconds: 300 });
    }
    if (/^\/api\/v1\/auth\/mfa\/(totp|recovery)$/.test(call.path) && call.method === 'POST') {
      const input = request.postDataJSON(),
        recovery = call.path.endsWith('/recovery');
      if (
        !state.challenge ||
        input.challenge_token !== state.challenge.token ||
        input.code !== (recovery ? 'recovery-one' : '123456')
      )
        return reply({ error: { code: 'mfa_rejected' } }, 401);
      state.current = {
        user: state.challenge.user,
        token: `idle-session-${state.grants.length + 1}`,
        absolute: state.now + 86400000,
        deadline: state.now + idleMilliseconds,
      };
      state.challenge = null;
      const grant = {
        ...metadata(),
        access_token: state.current.token,
        token_type: 'Bearer',
        expires_in_seconds: idleMilliseconds / 1000,
      };
      state.grants.push({ factor: recovery ? 'recovery' : 'totp', ...grant });
      return reply(grant);
    }
    if (
      !state.current ||
      call.headers.authorization !== `Bearer ${state.current.token}` ||
      state.now >= Math.min(state.current.absolute, state.current.deadline)
    )
      return reply({ error: { code: 'invalid_session' } }, 401);
    if (call.path === '/api/v1/auth/logout' && call.method === 'POST') {
      state.current = null;
      return route.fulfill({ status: 204, headers: { 'Cache-Control': 'no-store' } });
    }
    if (call.path === '/api/v1/auth/me' && call.method === 'GET') return reply(state.current.user);
    if (['/api/v1/auth/session', '/api/v1/auth/activity'].includes(call.path)) {
      const method = call.path.endsWith('/activity') ? 'POST' : 'GET';
      if (call.method !== method || call.body !== null || request.url().includes('?')) {
        state.unexpected.push(call);
        return reply({ error: { code: 'invalid_session_request' } }, 400);
      }
      if (method === 'POST')
        state.current.deadline = Math.min(state.current.absolute, state.now + idleMilliseconds);
      return reply(metadata());
    }
    if (call.method !== 'GET') {
      state.writes.push(call);
      return reply({ error: { code: 'unexpected_business_write' } }, 501);
    }
    if (!reads.has(call.path)) {
      state.unexpected.push(call);
      return reply({ error: { code: 'unimplemented_test_request' } }, 501);
    }
    if (state.gate?.path === call.path) {
      state.gate.entered = true;
      await state.gate.promise;
    }
    if (!state.allowed && (call.path === casePath || call.path.startsWith(documentsPath)))
      return reply({ error: { code: 'case_not_found' } }, 404);
    return route.fallback();
  });
  state.advance = async (milliseconds) => {
    state.now += milliseconds;
    await page.clock.runFor(milliseconds);
  };
  state.activity = () => state.calls.filter((call) => call.path === '/api/v1/auth/activity');
  return state;
}

export async function checkSessionRequests(page) {
  const state = sessions.get(page);
  if (state) {
    state.gate?.release();
    expect(state.unexpected, 'Every requested API route must have an explicit contract').toEqual(
      [],
    );
    expect(state.writes, 'Re-entry must never replay a business mutation').toEqual([]);
  }
}

export async function signInOther(page) {
  await page.getByLabel('Correo electr\u00f3nico').fill(other.email);
  await page.getByLabel('Contrase\u00f1a', { exact: true }).fill('a-long-password');
  await page.getByRole('button', { name: 'Continuar', exact: true }).click();
  await page.getByLabel('C\u00f3digo de 6 d\u00edgitos', { exact: true }).fill('123456');
  await page.getByRole('button', { name: 'Verificar y entrar' }).click();
  await expect(page.getByRole('heading', { name: 'Tu mesa de trabajo' })).toBeVisible();
}

export async function openEditor(page) {
  await openDocument(page);
  await page.getByRole('button', { name: editorName, exact: true }).click();
  const editor = page.getByRole('dialog', { name: editorName });
  await expect(editor).toBeVisible();
  return editor;
}

export async function draft(page) {
  const editor = await openEditor(page);
  await editor.getByLabel(classificationLabel, { exact: true }).fill(classification);
  await editor.getByLabel('Nueva etiqueta', { exact: true }).fill(pendingTag);
  await expect(editor.getByLabel('Nueva etiqueta', { exact: true })).toBeFocused();
  return editor.elementHandle();
}

export async function expire(page, state, element) {
  await state.advance(state.current.deadline - state.now + 1);
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  await expect(page.getByRole('alert')).toContainText(/sesi\u00f3n.*(termin|venc|expir)/i);
  await expect(page.getByLabel('Correo electr\u00f3nico')).toBeFocused();
  await expect(page.getByLabel('Correo electr\u00f3nico')).toBeEditable();
  await expect(page.locator('.app-layout')).toHaveCount(0);
  await expect(page.locator('dialog')).toHaveCount(0);
  await expect(page.getByText(document.name, { exact: true })).toHaveCount(0);
  expect(await element.evaluate((node) => node.isConnected)).toBe(false);
}

export async function blankEditor(page) {
  const editor = await openEditor(page);
  await expect(editor.getByLabel(classificationLabel, { exact: true })).toHaveValue('Civil');
  await expect(editor.getByLabel('Nueva etiqueta', { exact: true })).toHaveValue('');
}
