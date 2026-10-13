import test from 'node:test';
import assert from 'node:assert/strict';
import { createApi } from '../src/lib/api.mjs';

function pending() {
  let resolve;
  const promise = new Promise((value) => {
    resolve = value;
  });
  return { promise, resolve };
}

const user = {
  id: 'cda191de-5707-41e9-9dc1-2a7ef272cc9b',
  email: 'owner@example.test',
  role: 'owner',
};
const state = {
  user,
  policy: { absolute_ttl_seconds: 86400, idle_ttl_seconds: 900 },
  server_now_unix_ms: 100000,
  absolute_expires_at_unix_ms: 86500000,
  idle_expires_at_unix_ms: 1000000,
};

function harness() {
  const requests = [];
  let expired = 0;
  const api = createApi(
    (url, options) => {
      if (url.endsWith('/auth/mfa/totp'))
        return Promise.resolve(
          Response.json({ access_token: JSON.parse(options.body).challenge_token, user }),
        );
      const item = { url, options, ...pending() };
      requests.push(item);
      return item.promise;
    },
    () => expired++,
  );
  return {
    api,
    requests,
    get expired() {
      return expired;
    },
    login: (token) => api.mfa(token, '123456', 'totp'),
  };
}

for (const [name, path, method] of [
  ['sessionStatus', '/auth/session', 'GET'],
  ['recordActivity', '/auth/activity', 'POST'],
]) {
  test(`${name} sends only the current bearer and returns server-confirmed deadlines`, async () => {
    const h = harness();
    await h.login('current');
    const result = h.api[name]({ idle_ttl_seconds: 86400, server_now_unix_ms: 0 });
    const request = h.requests.shift();
    assert.equal(request.url, `/api/v1${path}`);
    assert.equal(request.options.method, method);
    assert.equal(request.options.headers.Authorization, 'Bearer current');
    assert.equal(request.options.headers['Content-Type'], undefined);
    assert.equal(request.options.body, undefined);
    assert.equal(request.options.cache, 'no-store');
    assert.equal(request.options.credentials, 'omit');
    request.resolve(Response.json(state));
    assert.deepEqual(await result, state);
    assert.equal(h.expired, 0);
  });

  test(`${name} rejects an obsolete reply without expiring a replacement session`, async () => {
    const h = harness();
    await h.login('original');
    const result = assert.rejects(h.api[name]());
    const request = h.requests.shift();
    await h.login('replacement');
    request.resolve(Response.json({ error: { code: 'invalid_session' } }, { status: 401 }));
    await result;
    assert.equal(h.expired, 0);
    const next = h.api[name]();
    const current = h.requests.shift();
    assert.equal(current.options.headers.Authorization, 'Bearer replacement');
    current.resolve(Response.json(state));
    assert.deepEqual(await next, state);
  });

  test(`${name} preserves expiration handling for the current session`, async () => {
    const h = harness();
    await h.login('current');
    const result = assert.rejects(
      h.api[name](),
      (error) => error.status === 401 && error.code === 'invalid_session',
    );
    h.requests
      .shift()
      .resolve(Response.json({ error: { code: 'invalid_session' } }, { status: 401 }));
    await result;
    assert.equal(h.expired, 1);
  });
}
