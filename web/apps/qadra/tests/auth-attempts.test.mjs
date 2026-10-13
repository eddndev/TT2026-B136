import test from 'node:test';
import assert from 'node:assert/strict';
import { createApi } from '../src/lib/api.mjs';
import { memberActor } from './fixtures/members.mjs';

function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

function client() {
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
    queue,
    calls,
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

const session = (token) => ({ access_token: token, user: memberActor, expires_in_seconds: 86400 });
const challenge = (token) => ({ challenge_token: token, expires_in_seconds: 300 });
const superseded = (promise) =>
  assert.rejects(promise, (error) => {
    assert.equal(error.code, 'auth_attempt_superseded');
    assert.equal(error.status, undefined);
    return true;
  });
const answer = (exchange, value, status = 200) =>
  exchange.resolve(Response.json(value, { status }));
function settle(exchange, outcome, value) {
  if (outcome === 'network') exchange.reject(new TypeError('Disconnected'));
  else if (outcome === 'failure') answer(exchange, { error: { code: 'invalid_credentials' } }, 401);
  else answer(exchange, value);
}
async function establish(h, token, mode = 'totp', challengeToken = `challenge-${token}`) {
  const pending = h.api.mfa(challengeToken, '123456', mode);
  answer(h.take(`/auth/mfa/${mode}`), session(token));
  assert.deepEqual(await pending, session(token));
}
async function login(h, token) {
  const pending = h.api.login(`${token}@example.test`, 'password');
  answer(h.take('/auth/login'), challenge(token));
  assert.deepEqual(await pending, challenge(token));
}

test('the current login and captured challenge retain the existing API contract', async () => {
  const h = client();
  await establish(h, 'established');
  const pending = h.api.login('owner@example.test', 'password');
  const exchange = h.take('/auth/login');
  assert.equal(exchange.options.headers.Authorization, undefined);
  assert.deepEqual(JSON.parse(exchange.options.body), {
    email: 'owner@example.test',
    password: 'password',
  });
  assert.equal((await h.api.me()).bearer, 'Bearer established');
  answer(exchange, challenge('captured'));
  await pending;
  await establish(h, 'replacement', 'recovery', 'captured');
  assert.equal((await h.api.me()).bearer, 'Bearer replacement');
  assert.equal(h.expired, 0);
});

for (const outcome of ['success', 'failure', 'network']) {
  test(`a late login ${outcome} cannot supersede a newer challenge or session`, async () => {
    const h = client();
    const old = superseded(h.api.login('old@example.test', 'password'));
    const exchange = h.take('/auth/login');
    await login(h, 'new');
    await establish(h, 'new-session', 'totp', 'new');
    settle(exchange, outcome, challenge('old'));
    await old;
    assert.equal((await h.api.me()).bearer, 'Bearer new-session');
    assert.equal(h.expired, 0);
  });
}

for (const mode of ['totp', 'recovery']) {
  for (const outcome of ['success', 'failure', 'network']) {
    test(`a late ${mode} MFA ${outcome} cannot replace a newer MFA result`, async () => {
      const h = client();
      const old = superseded(h.api.mfa('old-challenge', '123456', mode));
      const exchange = h.take(`/auth/mfa/${mode}`);
      await establish(h, 'new-session');
      settle(exchange, outcome, session('old-session'));
      await old;
      assert.equal((await h.api.me()).bearer, 'Bearer new-session');
      assert.equal(h.expired, 0);
    });
  }
}

test('a challenge from an earlier accepted login cannot be submitted after a newer login', async () => {
  const h = client();
  await login(h, 'old');
  await login(h, 'new');
  const calls = h.calls.length;
  const rejected = superseded(h.api.mfa('old', '123456', 'totp'));
  if (h.queue.length) answer(h.take('/auth/mfa/totp'), session('obsolete'));
  await rejected;
  assert.equal(h.calls.length, calls, 'the obsolete challenge must not reach the server');
  await establish(h, 'new-session', 'totp', 'new');
  assert.equal((await h.api.me()).bearer, 'Bearer new-session');
});

test('starting another password login invalidates MFA before that login responds', async () => {
  const h = client();
  await login(h, 'old');
  const old = superseded(h.api.mfa('old', '123456', 'totp'));
  const exchange = h.take('/auth/mfa/totp');
  const next = h.api.login('new@example.test', 'password');
  const password = h.take('/auth/login');
  answer(exchange, session('obsolete'));
  await old;
  assert.equal((await h.api.me()).bearer, null);
  answer(password, challenge('new'));
  await next;
  await establish(h, 'new-session', 'totp', 'new');
});

for (const status of [200, 401]) {
  test(`supersession during an MFA HTTP ${status} body read cannot leak its result`, async () => {
    const h = client(),
      entered = deferred(),
      body = deferred();
    const old = superseded(h.api.mfa('old', '123456', 'totp'));
    h.take('/auth/mfa/totp').resolve({
      ok: status === 200,
      status,
      json() {
        entered.resolve();
        return body.promise;
      },
    });
    await entered.promise;
    await establish(h, 'new-session');
    body.resolve(status === 200 ? session('obsolete') : { error: { code: 'invalid_credentials' } });
    await old;
    assert.equal((await h.api.me()).bearer, 'Bearer new-session');
    assert.equal(h.expired, 0);
  });
}

test('a malformed obsolete MFA body is superseded instead of leaking a parse error', async () => {
  const h = client(),
    entered = deferred(),
    body = deferred();
  const old = superseded(h.api.mfa('old', '123456', 'totp'));
  h.take('/auth/mfa/totp').resolve({
    ok: true,
    status: 200,
    json() {
      entered.resolve();
      return body.promise;
    },
  });
  await entered.promise;
  await establish(h, 'new-session');
  body.reject(new SyntaxError('Invalid JSON'));
  await old;
  assert.equal((await h.api.me()).bearer, 'Bearer new-session');
});

test('a failed newer password attempt does not make an older captured challenge usable', async () => {
  const h = client();
  await login(h, 'old');
  const denied = assert.rejects(
    h.api.login('new@example.test', 'wrong'),
    (error) => error.status === 401,
  );
  answer(h.take('/auth/login'), { error: { code: 'invalid_credentials' } }, 401);
  await denied;
  const calls = h.calls.length;
  const old = superseded(h.api.mfa('old', '123456', 'totp'));
  if (h.queue.length) answer(h.take('/auth/mfa/totp'), session('obsolete'));
  await old;
  assert.equal(h.calls.length, calls);
  assert.equal((await h.api.me()).bearer, null);
});

for (const succeeds of [true, false]) {
  test(`logout invalidates pending MFA immediately even when logout ${succeeds ? 'succeeds' : 'fails'}`, async () => {
    const h = client();
    await establish(h, 'original');
    const old = superseded(h.api.mfa('pending', '123456', 'totp'));
    const exchange = h.take('/auth/mfa/totp');
    const logout = h.api.logout();
    const checkedLogout = succeeds
      ? logout
      : assert.rejects(logout, (error) => error.status === 503);
    const logoutRequest = h.take('/auth/logout');
    answer(exchange, session('obsolete'));
    await old;
    const sent = h.calls.length;
    await assert.rejects(h.api.me(), (error) => error.code === 'session_inactive');
    assert.equal(h.calls.length, sent);
    if (succeeds) logoutRequest.resolve(new Response(null, { status: 204 }));
    else answer(logoutRequest, { error: { code: 'unavailable' } }, 503);
    await checkedLogout;
    await assert.rejects(h.api.me(), (error) => error.code === 'session_inactive');
    assert.equal(h.calls.length, sent);
  });
}

test('a protected 401 invalidates pending authentication without a late revival', async () => {
  const h = client();
  await establish(h, 'original');
  const old = superseded(h.api.mfa('pending', '123456', 'totp'));
  const exchange = h.take('/auth/mfa/totp');
  const rejected = assert.rejects(h.api.audit(), (error) => error.status === 401);
  answer(h.take('/audit/verify'), { error: { code: 'invalid_session' } }, 401);
  await rejected;
  answer(exchange, session('obsolete'));
  await old;
  const sent = h.calls.length;
  await assert.rejects(h.api.me(), (error) => error.code === 'session_inactive');
  assert.equal(h.calls.length, sent);
  assert.equal(h.expired, 1);
});

test('a confirmed self access change invalidates pending MFA but a no-op does not', async () => {
  for (const changed of [true, false]) {
    const h = client();
    await establish(h, 'original');
    const pending = h.api.mfa('pending', '123456', 'totp');
    const checked = changed ? superseded(pending) : pending;
    const exchange = h.take('/auth/mfa/totp');
    const role = changed ? 'paralegal' : 'owner';
    const update = h.api.members().changeAccess(memberActor.id, {
      expected_revision: '4',
      role,
      active: true,
    });
    answer(h.take(`/users/${memberActor.id}/access`), {
      ...memberActor,
      role,
      revision: changed ? '5' : '4',
    });
    await update;
    answer(exchange, session('new-session'));
    await checked;
    if (changed) {
      const sent = h.calls.length;
      await assert.rejects(h.api.me(), (error) => error.code === 'session_inactive');
      assert.equal(h.calls.length, sent);
    } else assert.equal((await h.api.me()).bearer, 'Bearer new-session');
    assert.equal(h.expired, changed ? 1 : 0);
  }
});

test('current authentication failures preserve their typed errors and the established bearer', async () => {
  const h = client();
  await establish(h, 'original');
  const rejected = assert.rejects(h.api.login('owner@example.test', 'wrong'), (error) => {
    assert.equal(error.code, 'invalid_credentials');
    assert.equal(error.status, 401);
    return true;
  });
  answer(h.take('/auth/login'), { error: { code: 'invalid_credentials' } }, 401);
  await rejected;
  assert.equal((await h.api.me()).bearer, 'Bearer original');
  assert.equal(h.expired, 0);
});

for (const status of [200, 401]) {
  test(`old protected HTTP ${status} stays isolated after a replacement authentication`, async () => {
    const h = client();
    await establish(h, 'original');
    const rejected = assert.rejects(h.api.audit(), /La sesi\u00f3n de esta solicitud termin\u00f3/);
    const exchange = h.take('/audit/verify');
    await establish(h, 'replacement');
    answer(
      exchange,
      status === 200 ? { valid: true } : { error: { code: 'invalid_session' } },
      status,
    );
    await rejected;
    assert.equal((await h.api.me()).bearer, 'Bearer replacement');
    assert.equal(h.expired, 0);
  });
}

for (const status of [204, 503]) {
  test(`late logout HTTP ${status} cannot clear a session established while it was pending`, async () => {
    const h = client();
    await establish(h, 'original');
    const pending = h.api.logout();
    const rejected =
      status === 204 ? pending : assert.rejects(pending, (error) => error.status === 503);
    const exchange = h.take('/auth/logout');
    await establish(h, 'replacement');
    if (status === 204) exchange.resolve(new Response(null, { status }));
    else answer(exchange, { error: { code: 'unavailable' } }, status);
    await rejected;
    assert.equal((await h.api.me()).bearer, 'Bearer replacement');
    assert.equal(h.expired, 0);
  });
}
