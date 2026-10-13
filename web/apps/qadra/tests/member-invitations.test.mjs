import test from 'node:test';
import assert from 'node:assert/strict';
import { takeInvitationLink } from '../src/lib/invitation-link.mjs';
import { createInvitationAcceptanceApi } from '../src/lib/invitation-acceptance-api.mjs';

const token = Buffer.alloc(32, 7).toString('base64url');
const operation = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
function browser(hash, fail = false) {
  const location = new URL(`https://qadra.example.test/${hash}`);
  return {
    location,
    history: {
      state: null,
      replaceState(_state, _title, url) {
        if (fail) throw new Error(token);
        assert.ok(!url.includes(token));
        location.hash = '';
      },
    },
    get sessionStorage() {
      throw new Error('no secret storage');
    },
  };
}
test('invitation capability is removed before use and not cached', () => {
  const page = browser(`#invitation=${token}`);
  assert.deepEqual(takeInvitationLink(page), { status: 'ready', token });
  assert.equal(page.location.hash, '');
  assert.equal(takeInvitationLink(page), null);
  assert.deepEqual(takeInvitationLink(browser('#invitation=wrong')), { status: 'invalid' });
  assert.deepEqual(takeInvitationLink(browser(`#invitation=${token}`, true)), {
    status: 'unavailable',
  });
  assert.equal(takeInvitationLink(browser('#cases')), null);
});
test('public enrollment carries no session and never retries an uncertain POST', async () => {
  const calls = [];
  const api = createInvitationAcceptanceApi(async (...args) => {
    calls.push(args);
    throw new Error(`private failure ${token}`);
  });
  await assert.rejects(api.prepare(token, operation, 'private password'), (error) => {
    assert.equal(error.code, 'invitation_uncertain');
    assert.ok(!error.message.includes(token));
    return true;
  });
  assert.equal(calls.length, 1);
  const [url, options] = calls[0];
  assert.equal(url, '/api/v1/auth/invitations/prepare');
  assert.deepEqual(JSON.parse(options.body), {
    token,
    operation_id: operation,
    password: 'private password',
  });
  assert.equal(options.credentials, 'omit');
  assert.equal(options.redirect, 'error');
  assert.equal(options.cache, 'no-store');
  assert.equal(options.referrerPolicy, 'no-referrer');
  assert.equal(options.headers.Authorization, undefined);
});
test('a rejected link is distinct from an uncertain completion and a receipt is read explicitly', async () => {
  const calls = [];
  const api = createInvitationAcceptanceApi(async (url, options) => {
    calls.push([url, JSON.parse(options.body)]);
    return calls.length === 1
      ? new Response('{}', { status: 400 })
      : new Response(JSON.stringify({ receipt: null }));
  });
  await assert.rejects(api.finish(token, operation, 'password', '123456'), {
    code: 'invitation_rejected',
  });
  assert.deepEqual(await api.receipt(token, operation, 'password'), { receipt: null });
  assert.equal(calls.length, 2);
  assert.equal(calls[1][0], '/api/v1/auth/invitations/receipt');
});
test('closing an enrollment ignores a late response containing MFA secrets', async () => {
  let respond;
  const api = createInvitationAcceptanceApi(
    () =>
      new Promise((resolve) => {
        respond = resolve;
      }),
  );
  const pending = api.prepare(token, operation, 'password');
  api.dispose();
  respond(new Response(JSON.stringify({ totp_secret_base32: 'private' })));
  await assert.rejects(pending, { code: 'invitation_closed' });
});
