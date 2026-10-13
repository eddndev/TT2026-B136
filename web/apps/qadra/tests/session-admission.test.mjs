import test from 'node:test';
import assert from 'node:assert/strict';
import { createApi } from '../src/lib/api.mjs';

function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
function fixture() {
  const calls = [],
    expired = [];
  const api = createApi(
    (url, options) => {
      const reply = deferred();
      calls.push({ url, options, ...reply });
      return reply.promise;
    },
    () => expired.push(true),
  );
  async function login(token = 'current') {
    const pending = api.mfa('challenge', '123456', 'totp');
    calls.at(-1).resolve(Response.json({ access_token: token, user: { id: token } }));
    await pending;
  }
  return { api, calls, expired, login };
}

test('local invalidation blocks new business calls before sending bytes', async () => {
  const f = fixture();
  await f.login();
  f.api.invalidateSession();
  for (const run of [
    () => f.api.me(),
    () => f.api.caseDocuments('case').upload(new Blob(['x']), 'x.txt'),
    () => f.api.sessionStatus(),
    () => f.api.recordActivity(),
  ]) {
    await assert.rejects(run(), (error) => error.code === 'session_inactive');
  }
  assert.equal(f.calls.length, 1);
  assert.deepEqual(f.expired, []);
  await f.login('replacement');
  const read = f.api.me();
  assert.equal(f.calls.at(-1).options.headers.Authorization, 'Bearer replacement');
  f.calls.at(-1).resolve(Response.json({ id: 'replacement' }));
  await read;
});

test('local invalidation rejects old reads and an unfinished MFA attempt', async () => {
  const f = fixture();
  await f.login();
  const read = assert.rejects(f.api.me());
  const reading = f.calls.at(-1);
  const mfa = assert.rejects(f.api.mfa('challenge', '123456', 'totp'));
  const authenticating = f.calls.at(-1);
  f.api.invalidateSession();
  reading.resolve(Response.json({ private: 'old' }));
  authenticating.resolve(Response.json({ access_token: 'late', user: { id: 'old' } }));
  await Promise.all([read, mfa]);
  await assert.rejects(f.api.me(), (error) => error.code === 'session_inactive');
  assert.equal(f.calls.length, 3);
});

test('a closed admission gate allows status revalidation but not business or activity', async () => {
  const f = fixture();
  await f.login();
  f.api.setSessionGuard(() => false);
  await assert.rejects(f.api.me(), (error) => error.code === 'session_inactive');
  await assert.rejects(f.api.recordActivity(), (error) => error.code === 'session_inactive');
  assert.equal(f.calls.length, 1);
  const status = f.api.sessionStatus();
  assert.equal(f.calls.at(-1).url, '/api/v1/auth/session');
  f.calls.at(-1).resolve(Response.json({ live: true }));
  await status;
  f.api.setSessionGuard(() => true);
  const read = f.api.me();
  f.calls.at(-1).resolve(Response.json({ live: true }));
  await read;
});

test('logout closes admission immediately even when remote revocation fails', async () => {
  const f = fixture();
  await f.login();
  const revoked = assert.rejects(f.api.logout());
  const request = f.calls.at(-1);
  assert.equal(request.options.headers.Authorization, 'Bearer current');
  await assert.rejects(f.api.me(), (error) => error.code === 'session_inactive');
  request.reject(new Error('offline'));
  await revoked;
  assert.equal(f.calls.length, 2);
});

for (const status of [204, 401]) {
  test(`a late logout ${status} cannot affect a replacement session`, async () => {
    const f = fixture();
    await f.login();
    const pending = f.api.logout();
    const result = status === 401 ? assert.rejects(pending) : pending;
    const request = f.calls.at(-1);
    await f.login('replacement');
    request.resolve(
      status === 204
        ? new Response(null, { status })
        : Response.json({ error: { code: 'invalid_session' } }, { status }),
    );
    await result;
    assert.deepEqual(f.expired, []);
    const read = f.api.me();
    assert.equal(f.calls.at(-1).options.headers.Authorization, 'Bearer replacement');
    f.calls.at(-1).resolve(Response.json({ id: 'replacement' }));
    await read;
  });
}

test('an old guard disposer cannot remove a newer registration of the same function', async () => {
  const f = fixture();
  await f.login();
  const guard = () => false;
  const old = f.api.setSessionGuard(guard);
  const current = f.api.setSessionGuard(guard);
  old();
  // A valid remote response would make an accidentally admitted call resolve.
  const attempted = f.api.me();
  if (f.calls.length > 1) f.calls.at(-1).resolve(Response.json({ id: 'current' }));
  await assert.rejects(attempted, (error) => error.code === 'session_inactive');
  assert.equal(f.calls.length, 1);
  current();
  const read = f.api.me();
  f.calls.at(-1).resolve(Response.json({ id: 'current' }));
  await read;
});
