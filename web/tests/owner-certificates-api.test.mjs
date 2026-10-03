import test from 'node:test';
import assert from 'node:assert/strict';
import { createApi } from '../src/lib/api.mjs';
import {
  bindingId,
  ownerId,
  publicPem,
  preparation,
  receipt,
  submission,
  withdrawn,
} from './fixtures/owner-certificates.mjs';

const base = '/api/v1/auth/certificate-bindings';
const json = (value, status = 200) =>
  new Response(JSON.stringify(value), {
    status,
    headers: { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' },
  });

test('own certificate facade discovers current and sends exact public requests through the common client', async () => {
  const calls = [],
    prepared = preparation(),
    original = receipt(prepared),
    terminal = withdrawn(original);
  const facade = createApi(async (path, options) => {
    calls.push({ path, options });
    if (path.endsWith('/current')) return json(null);
    if (path.endsWith('/prepare')) return json(prepared);
    if (path.endsWith('/withdraw')) return json(terminal);
    return json(original);
  }).ownerCertificates(ownerId);
  assert.equal(await facade.current(), null);
  assert.deepEqual(await facade.prepare(bindingId, publicPem.toString('base64')), prepared);
  assert.deepEqual(await facade.register(bindingId, submission(prepared)), original);
  assert.deepEqual(await facade.get(bindingId), original);
  assert.deepEqual(await facade.withdraw(bindingId, 1), terminal);
  assert.deepEqual(
    calls.map(({ path, options }) => [path, options.method]),
    [
      [`${base}/current`, 'GET'],
      [`${base}/${bindingId}/prepare`, 'POST'],
      [`${base}/${bindingId}/register`, 'POST'],
      [`${base}/${bindingId}`, 'GET'],
      [`${base}/${bindingId}/withdraw`, 'POST'],
    ],
  );
  assert.equal(calls[0].options.body, undefined);
  assert.equal(calls[3].options.body, undefined);
  assert.deepEqual(JSON.parse(calls[1].options.body), {
    certificate_base64: publicPem.toString('base64'),
  });
  assert.deepEqual(JSON.parse(calls[2].options.body), submission(prepared));
  assert.deepEqual(JSON.parse(calls[4].options.body), { expected_revision: 1 });
  for (const { options } of calls) {
    assert.equal(options.cache, 'no-store');
    assert.equal(options.credentials, 'omit');
    assert.equal(options.redirect, 'error');
  }
});

test('own certificate responses cannot substitute another account, UUID, terminal current or rounded counters', async () => {
  for (const [operation, mutate] of [
    [
      'current',
      (value) => {
        value.owner_id = bindingId;
      },
    ],
    [
      'get',
      (value) => {
        value.binding_id = ownerId;
      },
    ],
    ['current', (value) => Object.assign(value, withdrawn(value))],
    [
      'get',
      (value) => {
        value.registration.account_revision = 9007199254740992;
      },
    ],
    [
      'get',
      (value) => {
        value.registration.auth_generation = 9007199254740992;
      },
    ],
    [
      'get',
      (value) => {
        value.registration.trust.crl_number = 9007199254740992;
      },
    ],
    [
      'prepare',
      (value) => {
        value.statement_base64 = Buffer.alloc(149).toString('base64');
      },
    ],
  ]) {
    const value = operation === 'prepare' ? preparation() : receipt();
    mutate(value);
    const facade = createApi(async () => json(value)).ownerCertificates(ownerId);
    await assert.rejects(
      operation === 'prepare'
        ? facade.prepare(bindingId, publicPem.toString('base64'))
        : operation === 'get'
          ? facade.get(bindingId)
          : facade.current(),
    );
  }
});

test('disposed certificate facade and a replaced MFA session reject responses already in flight', async () => {
  for (const replaceSession of [false, true]) {
    let release, started;
    const entered = new Promise((resolve) => {
      started = resolve;
    });
    const held = new Promise((resolve) => {
      release = resolve;
    });
    const api = createApi(async (path) => {
      if (path.includes('/mfa/'))
        return json({ access_token: 'another-session', user: { id: bindingId, role: 'owner' } });
      started();
      await held;
      return json(receipt());
    });
    const facade = api.ownerCertificates(ownerId),
      pending = facade.current();
    await entered;
    if (replaceSession) await api.mfa('challenge', '123456', 'totp');
    else facade.dispose();
    release();
    await assert.rejects(pending);
  }
});
