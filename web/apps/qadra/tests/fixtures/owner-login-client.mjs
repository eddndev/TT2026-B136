import assert from 'node:assert/strict';
import { createApi } from '../../src/lib/api.mjs';
import { selection, challenge, token, signature, mfa, mfaToken } from './owner-login.mjs';
import { absoluteSession } from './session.mjs';

export { selection, challenge, token, signature, mfa, mfaToken };
export const base = '/auth/certificate-login';
export function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
export function client() {
  const queue = [],
    calls = [];
  let expired = 0;
  const api = createApi(
    (url, options) => {
      calls.push({ url, options });
      if (url.endsWith('/me'))
        return Promise.resolve(Response.json({ bearer: options.headers.Authorization ?? null }));
      const exchange = { ...deferred(), url, options };
      queue.push(exchange);
      return exchange.promise;
    },
    () => expired++,
  );
  return {
    api,
    calls,
    queue,
    get expired() {
      return expired;
    },
    take(path) {
      const exchange = queue.shift();
      assert.equal(exchange?.url, `/api/v1${path}`);
      return exchange;
    },
  };
}
export const answer = (exchange, value, status = 200) =>
  exchange.resolve(Response.json(value, { status }));
export const session = (
  bearer,
  user = {
    id: selection.ownerId,
    email: 'owner@example.test',
    role: 'owner',
  },
) => absoluteSession(user, bearer);
export const superseded = (pending) =>
  assert.rejects(pending, (error) => {
    assert.equal(error.code, 'auth_attempt_superseded');
    return true;
  });
export async function start(h, value = challenge()) {
  const pending = h.api.startCertificateLogin(selection);
  answer(h.take(`${base}/start`), value);
  assert.deepEqual(await pending, value);
}
export async function prove(h, challengeToken = token) {
  const pending = h.api.proveCertificateLogin(challengeToken, signature);
  answer(h.take(`${base}/proof`), mfa());
  assert.deepEqual(await pending, mfa());
}
export async function establish(h, bearer, challengeToken = mfaToken) {
  const pending = h.api.mfa(challengeToken, '123456', 'totp');
  const value = session(bearer);
  answer(h.take('/auth/mfa/totp'), value);
  assert.deepEqual(await pending, value);
}
