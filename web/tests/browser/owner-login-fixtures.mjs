import { expect } from '@playwright/test';
import { setup } from './helpers.mjs';
import { absoluteSession } from '../fixtures/session.mjs';
import {
  publicReceipt,
  selection,
  statement,
  signature,
  challenge,
} from '../fixtures/owner-login.mjs';

export { publicReceipt, selection, signature };
export const loginPath = '/api/v1/auth/certificate-login';
export const owner = { id: selection.ownerId, email: 'owner@example.test', role: 'owner' };
const states = new WeakMap();
const workspaceReads = new Set([
  '/api/v1/dashboard',
  '/api/v1/document-integrity-incidents',
  '/api/v1/case-administrations',
]);

export function holdLogin(state, path) {
  const gate = { path, entered: false, release() {} };
  gate.promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.gates.push(gate);
  return gate;
}

export async function ownerLoginSetup(page) {
  const fallbackCalls = await setup(page, 'owner');
  expect(fallbackCalls, 'Availability must be requested only after choosing the method').toEqual(
    [],
  );
  const state = {
    calls: [],
    unexpected: [],
    starts: [],
    proofs: [],
    mfas: [],
    gates: [],
    availability: true,
    nextProof: null,
    nextMfa: null,
    pendingMfa: null,
    capture: null,
    current: null,
  };
  states.set(page, state);
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    const call = {
      path: url.pathname,
      method: request.method(),
      search: url.search,
      body: request.postData(),
      headers: request.headers(),
    };
    state.calls.push(call);
    const reply = (json, status = 200) =>
      route.fulfill({
        status,
        json,
        headers: { 'Cache-Control': 'no-store' },
      });
    const fail = (status, code) => reply({ error: { code } }, status);
    const unexpected = () => {
      state.unexpected.push(call);
      return fail(501, 'unexpected_owner_login_request');
    };
    const wait = async () => {
      const gate = state.gates.find((item) => !item.entered && item.path === call.path);
      if (gate) {
        gate.entered = true;
        await gate.promise;
      }
    };
    if (call.path.startsWith('/api/v1/auth/')) {
      if (call.search || url.href.includes('?') || call.headers.authorization) return unexpected();
      if (call.path === `${loginPath}/availability`) {
        if (call.method !== 'GET' || call.body !== null) return unexpected();
        if (state.availability === 'disconnect') return route.abort();
        if (state.availability === 404) return fail(404, 'not_found');
        return reply(
          state.availability === 'malformed'
            ? { enabled: 'true' }
            : { enabled: state.availability },
        );
      }
      if (call.method !== 'POST' || !call.headers['content-type']?.startsWith('application/json'))
        return unexpected();
      const input = request.postDataJSON();
      if (call.path === `${loginPath}/start`) {
        expect(input).toEqual({ owner_id: selection.ownerId, binding_id: selection.bindingId });
        const bytes = Buffer.from(statement),
          index = state.starts.length + 1;
        bytes.fill(0x73, 10, 26);
        bytes.fill(0x74, 26, 58);
        bytes.writeUInt32BE(0xffffffff, 58);
        bytes.writeBigUInt64BE(9223372036854775807n, 78);
        bytes.fill(index, 134, 166);
        const value = challenge(bytes, Buffer.alloc(32, index).toString('base64url'));
        state.capture = value;
        state.pendingMfa = null;
        state.starts.push({ ...call, input, value });
        await wait();
        return reply(value);
      }
      if (call.path === `${loginPath}/proof`) {
        const captured = state.capture;
        if (!captured) return unexpected();
        expect(input).toEqual({
          challenge_token: captured.challenge_token,
          signature_base64: signature,
        });
        state.capture = null;
        state.proofs.push({ ...call, input });
        const outcome = state.nextProof;
        state.nextProof = null;
        const value = {
          challenge_token: Buffer.alloc(32, 100 + state.proofs.length).toString('base64url'),
          expires_in_seconds: 300,
        };
        state.pendingMfa = { token: value.challenge_token, certificate: true };
        await wait();
        if (outcome === 'disconnect') return route.abort();
        if (outcome === 401) return fail(401, 'invalid_credentials');
        return reply(value);
      }
      if (call.path === '/api/v1/auth/login') {
        expect(input).toEqual({ email: 'hatz@example.com', password: 'a-long-password' });
        state.pendingMfa = { token: 'password-challenge', certificate: false };
        await wait();
        return reply({ challenge_token: 'password-challenge', expires_in_seconds: 300 });
      }
      if (/^\/api\/v1\/auth\/mfa\/(totp|recovery)$/.test(call.path)) {
        if (!state.pendingMfa) return unexpected();
        expect(input).toEqual({
          challenge_token: state.pendingMfa.token,
          code: call.path.endsWith('/recovery') ? 'recovery-one' : '123456',
        });
        const certificate = state.pendingMfa.certificate,
          outcome = state.nextMfa;
        state.pendingMfa = state.nextMfa = null;
        state.mfas.push({ ...call, input, certificate });
        const value = absoluteSession(outcome?.user ?? owner, `owner-session-${state.mfas.length}`);
        await wait();
        if (outcome?.status) return fail(outcome.status, 'mfa_rejected');
        state.current = value;
        return reply(value);
      }
      return unexpected();
    }
    if (
      call.method === 'GET' &&
      workspaceReads.has(call.path) &&
      state.current &&
      call.headers.authorization === `Bearer ${state.current.access_token}`
    )
      return route.fallback();
    return unexpected();
  });
  return state;
}

export async function checkOwnerLoginRequests(page) {
  const state = states.get(page);
  if (!state) return;
  state.gates.forEach((gate) => gate.release());
  expect(
    state.unexpected,
    'Only the declared authentication and workspace requests are allowed',
  ).toEqual([]);
  for (const call of state.calls.filter((item) => item.path.startsWith(loginPath))) {
    expect(call.headers.authorization).toBeUndefined();
    expect(call.search).toBe('');
  }
}
