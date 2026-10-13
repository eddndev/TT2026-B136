import { expect } from '@playwright/test';
import { sessionSetup, checkSessionRequests } from './session-inactivity-helpers.mjs';
import {
  preparation,
  receipt,
  submission,
  withdrawn,
  publicPem,
} from '../fixtures/owner-certificates.mjs';

export const bindingsPath = '/api/v1/auth/certificate-bindings';
export const currentPath = `${bindingsPath}/current`;
export const principalPath = '/api/v1/auth/me';
const states = new WeakMap();
const clone = (value) => structuredClone(value);
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

export function holdOwnerRead(state, path) {
  const gate = { path, entered: false, release() {} };
  gate.promise = new Promise((resolve) => {
    gate.release = resolve;
  });
  state.ownerGates.push(gate);
  return gate;
}

export async function checkOwnerCertificateRequests(page) {
  const state = states.get(page);
  state?.ownerGates.forEach((gate) => gate.release());
  await checkSessionRequests(page);
  if (state)
    expect(state.nextOwnerWrite, 'Only explicitly armed certificate writes may be sent').toBeNull();
}

export async function ownerCertificateSetup(page) {
  const state = await sessionSetup(page);
  Object.assign(state, {
    ownerGates: [],
    ownerPreparations: [],
    ownerWrites: [],
    ownerRecords: new Map(),
    preparedBindings: new Map(),
    nextOwnerWrite: null,
  });
  states.set(page, state);
  state.ownCurrent = (owner = state.current.user.id) =>
    [...state.ownerRecords.values()].find(
      (row) => row.owner_id === owner && row.withdrawal === null,
    ) ?? null;
  await page.route(/\/api\/v1\/auth\/(me|certificate-bindings(?:[/?]|$))/, async (route) => {
    const request = route.request(),
      url = new URL(request.url());
    const call = {
      path: url.pathname,
      method: request.method(),
      search: url.search,
      body: request.postData(),
      headers: request.headers(),
      at: state.now,
    };
    state.calls.push(call);
    const reply = (json, status = 200) =>
      route.fulfill({ status, json, headers: { 'Cache-Control': 'no-store' } });
    const fail = (code, status) => reply({ error: { code } }, status);
    const unexpected = () => {
      state.unexpected.push(call);
      return fail('unimplemented_owner_certificate_request', 501);
    };
    const wait = async () => {
      const gate = state.ownerGates.find((value) => !value.entered && value.path === call.path);
      if (gate) {
        gate.entered = true;
        await gate.promise;
      }
    };
    if (
      !state.current ||
      call.headers.authorization !== `Bearer ${state.current.token}` ||
      state.now >= Math.min(state.current.absolute, state.current.deadline)
    )
      return fail('invalid_session', 401);
    if (call.search || url.href.includes('?')) return unexpected();
    const owner = state.current.user;
    if (call.path === principalPath) {
      if (call.method !== 'GET' || call.body !== null) return unexpected();
      const value = clone(owner);
      await wait();
      return reply(value);
    }
    if (owner.role !== 'owner') return fail('permission_denied', 403);
    if (call.path === currentPath) {
      if (call.method !== 'GET' || call.body !== null) return unexpected();
      const value = clone(state.ownCurrent());
      await wait();
      return reply(value);
    }
    const parts = call.path.slice(bindingsPath.length + 1).split('/');
    const [id, action] = parts;
    if (!uuid.test(id) || id === '00000000-0000-0000-0000-000000000000' || parts.length > 2)
      return unexpected();
    if (call.method === 'GET' && !action && call.body === null) {
      const value = clone(state.ownerRecords.get(id));
      await wait();
      return value?.owner_id === owner.id ? reply(value) : fail('owner_certificate_not_found', 404);
    }
    if (call.method !== 'POST' || !call.headers['content-type']?.startsWith('application/json'))
      return unexpected();
    const input = request.postDataJSON();
    if (action === 'prepare') {
      expect(Object.keys(input)).toEqual(['certificate_base64']);
      expect(Buffer.from(input.certificate_base64, 'base64')).toEqual(publicPem);
      const prepared = preparation(owner.id, id);
      state.preparedBindings.set(id, clone(prepared));
      state.ownerPreparations.push({ ...call, id, values: clone(input), prepared });
      await wait();
      return reply(prepared);
    }
    const mode = state.nextOwnerWrite;
    if (!mode || mode.action !== action) return unexpected();
    state.nextOwnerWrite = null;
    state.ownerWrites.push({ ...call, id, action, values: clone(input) });
    let result;
    if (action === 'register') {
      const prepared = state.preparedBindings.get(id);
      expect(prepared, 'Registration must use an explicitly prepared UUID').toBeTruthy();
      expect(Object.keys(input).sort()).toEqual([
        'certificate_der_base64',
        'signature_base64',
        'statement_base64',
      ]);
      expect(input).toEqual(submission(prepared, input.signature_base64));
      expect(Buffer.from(input.signature_base64, 'base64')).toHaveLength(384);
      result = receipt(prepared, input.signature_base64);
    } else if (action === 'withdraw') {
      expect(input).toEqual({ expected_revision: 1 });
      const original = state.ownerRecords.get(id);
      expect(original?.owner_id).toBe(owner.id);
      result = original.withdrawal ? original : withdrawn(original);
    } else return unexpected();
    if (mode.commit !== false) state.ownerRecords.set(id, clone(result));
    await wait();
    return mode.status
      ? fail(mode.status === 401 ? 'invalid_session' : 'server_busy', mode.status)
      : reply(mode.response ?? result);
  });
  return state;
}
