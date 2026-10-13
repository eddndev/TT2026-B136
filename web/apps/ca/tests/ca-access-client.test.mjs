import test from 'node:test';
import assert from 'node:assert/strict';
import { createCaAccessClient } from '../src/lib/ca-access-client.mjs';

const operator = { id: '171e1bf7-c543-4b73-826e-d972ac894119', email: 'ca@example.test' };
const password = 'synthetic ca password';
const challenge = (token = 'private-challenge') => ({
  challenge_token: token, expires_in_seconds: 300,
});
const status = (changes = {}) => ({
  operator: { ...operator },
  policy: { absolute_ttl_seconds: 300, idle_ttl_seconds: 60 },
  server_now_unix_ms: 1_000_000,
  absolute_expires_at_unix_ms: 1_300_000,
  idle_expires_at_unix_ms: 1_060_000,
  ...changes,
});
const session = (token = 'private-bearer', changes = {}) => ({
  ...status(), access_token: token, token_type: 'Bearer', expires_in_seconds: 60, ...changes,
});

function harness() {
  const queue = [], calls = [], states = [], timers = new Map();
  let clock = 0, timerId = 0;
  const client = createCaAccessClient({
    fetcher(url, options) {
      const call = { url, options };
      calls.push(call);
      return new Promise((resolve, reject) => queue.push({ ...call, resolve, reject }));
    },
    now: () => clock,
    setTimer(callback, delay) {
      const id = ++timerId;
      timers.set(id, { callback, at: clock + delay });
      return id;
    },
    clearTimer: (id) => timers.delete(id),
    onState: (value) => states.push(value),
  });
  return {
    client, calls, states,
    take(path, scope = 'auth') {
      const request = queue.shift();
      assert.equal(request?.url, `/api/v1/${scope}/${path}`);
      assert.equal(request.options.cache, 'no-store');
      assert.equal(request.options.credentials, 'omit');
      assert.equal(request.options.redirect, 'error');
      return request;
    },
    at(value) { clock = value; },
    advance(value) {
      clock = value;
      for (const [id, timer] of [...timers]) {
        if (timer.at <= clock) {
          timers.delete(id);
          timer.callback();
        }
      }
    },
  };
}

function answer(request, value, code = 200) {
  request.resolve(code === 204 ? new Response(null, { status: code }) : Response.json(value, { status: code }));
}

async function login(h, token = 'private-challenge') {
  const pending = h.client.login(operator.email, password);
  answer(h.take('login'), challenge(token));
  await pending;
  assert.equal(h.client.snapshot().phase, 'mfa');
}

async function establish(h, token = 'private-bearer') {
  await login(h);
  const pending = h.client.mfa('123456', 'totp');
  answer(h.take('mfa/totp'), session(token));
  await pending;
}

test('CA commands use only the admitted bearer and discard replies after logout', async () => {
  const h = harness();
  await assert.rejects(h.client.command({ action: 'status' }), { code: 'invalid_session' });
  await establish(h);
  const pending = h.client.command({ action: 'status' });
  const operation = h.take('operations', 'ca');
  assert.equal(operation.options.headers.Authorization, 'Bearer private-bearer');
  assert.deepEqual(JSON.parse(operation.options.body), { action: 'status' });
  const logout = h.client.logout();
  answer(h.take('logout'), null, 204);
  await logout;
  answer(operation, { root_fingerprint: 'public fingerprint' });
  await assert.rejects(pending, { code: 'auth_attempt_superseded' });
  assert.equal(h.client.snapshot().phase, 'signed-out');
});

test('CA command failures retain uncertain requests without retrying mutations', async () => {
  const h = harness();
  await establish(h);
  const command = { action: 'resume-issue', operation_id: 'fixed-operation' };
  const pending = h.client.command(command);
  h.take('operations', 'ca').reject(new Error('reply lost'));
  await assert.rejects(pending, { code: 'network_error' });
  assert.equal(h.client.snapshot().phase, 'active');
  assert.equal(h.calls.filter((call) => call.url.includes('/ca/')).length, 1);
  const denied = h.client.command(command);
  answer(h.take('operations', 'ca'), { error: { code: 'invalid_session' } }, 401);
  await assert.rejects(denied, { code: 'invalid_session' });
  assert.equal(h.client.snapshot().phase, 'expired');
});

function safeSnapshot(value) {
  const encoded = JSON.stringify(value);
  for (const secret of [password, 'private-challenge', 'private-bearer', 'RECOVERY-0', '123456'])
    assert.ok(!encoded.includes(secret), 'a snapshot exposed a credential');
  for (const field of ['access_token', 'challenge_token', 'password', 'code', 'role', 'user'])
    assert.ok(!encoded.includes(`"${field}":`), `a snapshot exposed ${field}`);
}

for (const [mode, code] of [['totp', '123456'], ['recovery', 'RECOVERY-0']]) {
  test(`${mode} requires a password challenge and publishes only the operator`, async (t) => {
    for (const name of ['localStorage', 'sessionStorage', 'indexedDB', 'caches']) {
      const previous = Object.getOwnPropertyDescriptor(globalThis, name);
      Object.defineProperty(globalThis, name, { configurable: true, get() { throw new Error('persistent credential storage is forbidden'); } });
      t.after(() => previous ? Object.defineProperty(globalThis, name, previous) : delete globalThis[name]);
    }
    const h = harness();
    await assert.rejects(h.client.mfa(code, mode));
    assert.equal(h.calls.length, 0);
    const pending = h.client.login(operator.email, password);
    const request = h.take('login');
    assert.equal(request.options.method, 'POST');
    assert.equal(request.options.headers.Authorization, undefined);
    assert.deepEqual(JSON.parse(request.options.body), { email: operator.email, password });
    answer(request, challenge());
    await pending;
    assert.equal(h.client.snapshot().phase, 'mfa');
    await assert.rejects(h.client.sessionStatus());
    const verified = h.client.mfa(code, mode);
    const factor = h.take(`mfa/${mode}`);
    assert.equal(factor.options.headers.Authorization, undefined);
    assert.deepEqual(JSON.parse(factor.options.body), { challenge_token: 'private-challenge', code });
    answer(factor, session());
    await verified;
    assert.equal(h.client.snapshot().phase, 'active');
    assert.deepEqual(h.client.snapshot().operator, operator);
    assert.equal(h.client.snapshot().remainingMs, 60_000);
    h.states.forEach(safeSnapshot);
    safeSnapshot(h.client.snapshot());
    h.client.dispose();
  });
}

for (const failure of ['rejected', 'server', 'network']) {
  test(`an MFA ${failure} consumes the challenge and requires a fresh password login`, async () => {
    const h = harness();
    await login(h);
    const pending = assert.rejects(h.client.mfa('123456', 'totp'));
    const request = h.take('mfa/totp');
    if (failure === 'network') request.reject(new TypeError('private network detail'));
    else answer(request, { error: { code: failure === 'rejected' ? 'mfa_rejected' : 'internal_error' } }, failure === 'rejected' ? 401 : 500);
    await pending;
    assert.equal(h.client.snapshot().phase, 'signed-out');
    const calls = h.calls.length;
    await assert.rejects(h.client.mfa('RECOVERY-0', 'recovery'));
    assert.equal(h.calls.length, calls);
    assert.equal(h.client.snapshot().operator, null);
  });
}

test('a consumed or expired challenge cannot be submitted again', async () => {
  const h = harness();
  await login(h);
  const pending = h.client.mfa('123456', 'totp');
  const request = h.take('mfa/totp');
  await assert.rejects(h.client.mfa('RECOVERY-0', 'recovery'));
  assert.equal(h.calls.length, 2);
  answer(request, session());
  await pending;
  await login(h, 'expiring-challenge');
  h.advance(300_000);
  const calls = h.calls.length;
  await assert.rejects(h.client.mfa('123456', 'totp'));
  assert.equal(h.calls.length, calls);
});

for (const [method, path, verb] of [['sessionStatus', 'session', 'GET'], ['recordActivity', 'activity', 'POST']]) {
  test(`${method} uses the current bearer and accepts only server supplied session times`, async () => {
    const h = harness();
    await establish(h);
    const pending = h.client[method]({ idle_ttl_seconds: 86400, server_now_unix_ms: 0 });
    const request = h.take(path);
    assert.equal(request.options.method, verb);
    assert.equal(request.options.headers.Authorization, 'Bearer private-bearer');
    assert.equal(request.options.headers['Content-Type'], undefined);
    assert.equal(request.options.body, undefined);
    answer(request, status());
    await pending;
    assert.equal(h.client.snapshot().remainingMs, 60_000);
    assert.deepEqual(h.client.snapshot().operator, operator);
  });
}

test('idle expiration subtracts transport time and prevents further protected requests', async () => {
  const h = harness();
  await login(h);
  h.at(100);
  const pending = h.client.mfa('123456', 'totp');
  const request = h.take('mfa/totp');
  h.at(400);
  answer(request, session());
  await pending;
  assert.equal(h.client.snapshot().remainingMs, 59_700);
  h.advance(60_100);
  assert.equal(h.client.snapshot().phase, 'expired');
  assert.equal(h.client.snapshot().operator, null);
  assert.equal(h.client.snapshot().remainingMs, 0);
  const calls = h.calls.length;
  await assert.rejects(h.client.sessionStatus());
  await assert.rejects(h.client.recordActivity());
  assert.equal(h.calls.length, calls);
});

test('only a confirmed activity reply can extend idle time', async () => {
  const h = harness();
  await establish(h);
  h.at(10_000);
  const pending = h.client.recordActivity();
  const request = h.take('activity');
  assert.equal(h.client.snapshot().remainingMs, 50_000);
  answer(request, status({ server_now_unix_ms: 1_010_000, idle_expires_at_unix_ms: 1_070_000 }));
  await pending;
  h.advance(60_000);
  assert.equal(h.client.snapshot().phase, 'active');
  assert.equal(h.client.snapshot().remainingMs, 10_000);
  h.advance(70_000);
  assert.equal(h.client.snapshot().phase, 'expired');
});

test('office session responses never establish CA access', async () => {
  const h = harness();
  await login(h);
  const pending = assert.rejects(h.client.mfa('123456', 'totp'));
  const value = session();
  delete value.operator;
  value.user = { ...operator, role: 'owner' };
  answer(h.take('mfa/totp'), value);
  await pending;
  assert.equal(h.client.snapshot().phase, 'signed-out');
  assert.equal(h.client.snapshot().operator, null);
  h.states.forEach(safeSnapshot);
});

for (const changed of ['operator', 'absolute']) {
  test(`a changed ${changed} in session status closes local admission`, async () => {
    const h = harness();
    await establish(h);
    const pending = assert.rejects(h.client.sessionStatus());
    answer(h.take('session'), status(changed === 'operator'
      ? { operator: { ...operator, id: '5bc0ee34-22c7-49b0-b0d0-10e02dc7109c' } }
      : { absolute_expires_at_unix_ms: 1_300_001 }));
    await pending;
    assert.equal(h.client.snapshot().phase, 'expired');
    assert.equal(h.client.snapshot().operator, null);
  });
}

for (const code of [401, 403]) {
  test(`HTTP ${code} on the current session discards its bearer`, async () => {
    const h = harness();
    await establish(h);
    const pending = assert.rejects(h.client.sessionStatus());
    answer(h.take('session'), { error: { code: code === 401 ? 'invalid_session' : 'permission_denied' } }, code);
    await pending;
    assert.equal(h.client.snapshot().phase, 'expired');
    assert.equal(h.client.snapshot().operator, null);
    const calls = h.calls.length;
    await assert.rejects(h.client.recordActivity());
    assert.equal(h.calls.length, calls);
  });
}

for (const stage of ['login', 'mfa']) {
  test(`a late ${stage} result cannot replace a newer login attempt`, async () => {
    const h = harness();
    if (stage === 'mfa') await login(h);
    const old = assert.rejects(stage === 'login'
      ? h.client.login(operator.email, password)
      : h.client.mfa('123456', 'totp'));
    const request = h.take(stage === 'login' ? 'login' : 'mfa/totp');
    await login(h, 'new-challenge');
    answer(request, stage === 'login' ? challenge('old-challenge') : session('old-bearer'));
    await old;
    assert.equal(h.client.snapshot().phase, 'mfa');
    const current = h.client.mfa('123456', 'totp');
    const currentRequest = h.take('mfa/totp');
    assert.equal(JSON.parse(currentRequest.options.body).challenge_token, 'new-challenge');
    answer(currentRequest, session());
    await current;
    assert.equal(h.client.snapshot().phase, 'active');
  });
}

test('logout clears memory immediately even if revocation fails and a status reply arrives late', async () => {
  const h = harness();
  await establish(h);
  const stale = assert.rejects(h.client.sessionStatus());
  const oldRequest = h.take('session');
  const pending = assert.rejects(h.client.logout());
  const request = h.take('logout');
  assert.equal(request.options.method, 'POST');
  assert.equal(request.options.headers.Authorization, 'Bearer private-bearer');
  assert.equal(request.options.body, undefined);
  assert.equal(h.client.snapshot().phase, 'signed-out');
  assert.equal(h.client.snapshot().operator, null);
  answer(oldRequest, status());
  request.reject(new TypeError('offline'));
  await Promise.all([stale, pending]);
  const calls = h.calls.length;
  await assert.rejects(h.client.sessionStatus());
  assert.equal(h.calls.length, calls);
});

test('cancelling authentication makes its eventual response unusable', async () => {
  const h = harness();
  const pending = assert.rejects(h.client.login(operator.email, password));
  const request = h.take('login');
  h.client.cancelAuthentication();
  answer(request, challenge());
  await pending;
  assert.equal(h.client.snapshot().phase, 'signed-out');
  const calls = h.calls.length;
  await assert.rejects(h.client.mfa('123456', 'totp'));
  assert.equal(h.calls.length, calls);
});
