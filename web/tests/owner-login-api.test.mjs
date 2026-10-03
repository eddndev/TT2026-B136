import test from 'node:test';
import assert from 'node:assert/strict';
import {
  client,
  base,
  answer,
  start,
  prove,
  establish,
  deferred,
  superseded,
  session,
  selection,
  challenge,
  token,
  signature,
  mfa,
  mfaToken,
} from './fixtures/owner-login-client.mjs';

test('availability is explicit unprotected no-store GET and accepts only the derived boolean DTO', async () => {
  const h = client();
  assert.equal(h.calls.length, 0);
  for (const enabled of [false, true]) {
    const pending = h.api.certificateLoginAvailable();
    const request = h.take(`${base}/availability`);
    assert.equal(request.options.method, 'GET');
    assert.equal(request.options.body, undefined);
    assert.equal(request.options.headers.Authorization, undefined);
    answer(request, { enabled });
    assert.equal(await pending, enabled);
  }
  for (const value of [
    null,
    [],
    {},
    { enabled: 'true' },
    { enabled: true, owner: selection.ownerId },
  ]) {
    const pending = assert.rejects(h.api.certificateLoginAvailable());
    answer(h.take(`${base}/availability`), value);
    await pending;
  }
  for (const status of [404, 503]) {
    const pending = assert.rejects(
      h.api.certificateLoginAvailable(),
      (error) => error.status === status,
    );
    answer(h.take(`${base}/availability`), { error: { code: 'unavailable' } }, status);
    await pending;
  }
  assert.equal(h.expired, 0);
  for (const { options } of h.calls) {
    assert.equal(options.cache, 'no-store');
    assert.equal(options.credentials, 'omit');
    assert.equal(options.redirect, 'error');
  }
});

test('certificate first factor sends only selectors and proof then hands its sole MFA challenge to the common client', async () => {
  const h = client();
  await establish(h, 'existing-session');
  const pending = h.api.startCertificateLogin(selection);
  const prepare = h.take(`${base}/start`);
  assert.deepEqual(JSON.parse(prepare.options.body), {
    owner_id: selection.ownerId,
    binding_id: selection.bindingId,
  });
  assert.equal(prepare.options.headers.Authorization, undefined);
  assert.equal(prepare.options.method, 'POST');
  answer(prepare, challenge());
  await pending;
  assert.equal((await h.api.me()).bearer, 'Bearer existing-session');
  const calls = h.calls.length;
  await assert.rejects(h.api.mfa('unrelated-challenge', '123456', 'totp'));
  assert.equal(h.calls.length, calls);
  await assert.rejects(h.api.proveCertificateLogin(token, Buffer.alloc(383).toString('base64')));
  assert.equal(h.calls.length, calls);
  await assert.rejects(
    h.api.proveCertificateLogin(Buffer.alloc(32, 4).toString('base64url'), signature),
  );
  assert.equal(h.calls.length, calls);
  const proof = h.api.proveCertificateLogin(token, signature);
  const sent = h.take(`${base}/proof`);
  assert.deepEqual(JSON.parse(sent.options.body), {
    challenge_token: token,
    signature_base64: signature,
  });
  assert.equal(sent.options.headers.Authorization, undefined);
  answer(sent, mfa());
  assert.deepEqual(await proof, mfa());
  assert.equal((await h.api.me()).bearer, 'Bearer existing-session');
  const beforeReplay = h.calls.length;
  await assert.rejects(h.api.proveCertificateLogin(token, signature));
  assert.equal(h.calls.length, beforeReplay);
  await establish(h, 'certificate-session');
  assert.equal((await h.api.me()).bearer, 'Bearer certificate-session');
  assert.equal(h.expired, 0);
});

test('uncertain rejected or malformed proof consumes its local attempt without retries or MFA admission', async () => {
  for (const outcome of ['network', 'rejected', 'malformed']) {
    const h = client();
    await start(h);
    const pending = assert.rejects(h.api.proveCertificateLogin(token, signature));
    const sent = h.take(`${base}/proof`);
    if (outcome === 'network') sent.reject(new TypeError('private-transport-sentinel'));
    else if (outcome === 'rejected') answer(sent, { error: { code: 'invalid_credentials' } }, 401);
    else answer(sent, { ...mfa(), access_token: 'unexpected-session' });
    await pending;
    const count = h.calls.length;
    await assert.rejects(h.api.proveCertificateLogin(token, signature));
    await assert.rejects(h.api.mfa(mfaToken, '123456', 'totp'));
    assert.equal(h.calls.length, count);
    assert.equal(h.calls.filter((call) => call.url.endsWith('/proof')).length, 1);
    assert.equal(h.expired, 0);
    const fresh = { ...challenge(), challenge_token: Buffer.alloc(32, 0x34).toString('base64url') };
    await start(h, fresh);
    await prove(h, fresh.challenge_token);
    await establish(h, 'explicit-new-attempt');
    assert.equal((await h.api.me()).bearer, 'Bearer explicit-new-attempt');
  }
});

test('malformed start material never installs a usable proof or MFA challenge', async () => {
  for (const value of [
    { ...challenge(), statement_base64: Buffer.alloc(150).toString('base64') },
    { ...challenge(), expires_in_seconds: 0 },
    { ...challenge(), challenge_token: 'not-a-canonical-capture' },
  ]) {
    const h = client();
    const pending = assert.rejects(h.api.startCertificateLogin(selection));
    answer(h.take(`${base}/start`), value);
    await pending;
    const calls = h.calls.length;
    await assert.rejects(h.api.proveCertificateLogin(token, signature));
    await assert.rejects(h.api.mfa(mfaToken, '123456', 'totp'));
    assert.equal(h.calls.length, calls);
    assert.equal(h.expired, 0);
  }
});

test('certificate MFA rejects a coherent session for another account or role before installing its bearer', async () => {
  const otherOwner = {
    id: '77777777-7777-4777-8777-777777777777',
    email: 'other@example.test',
    role: 'owner',
  };
  const wrongRole = { id: selection.ownerId, email: 'owner@example.test', role: 'litigator' };
  for (const user of [otherOwner, wrongRole]) {
    for (const mode of ['totp', 'recovery']) {
      const h = client();
      await establish(h, 'existing-session');
      await start(h);
      await prove(h);
      const rejected = assert.rejects(h.api.mfa(mfaToken, '123456', mode));
      answer(h.take(`/auth/mfa/${mode}`), session('wrong-certificate-session', user));
      await rejected;
      assert.equal((await h.api.me()).bearer, 'Bearer existing-session');
      const count = h.calls.length;
      await assert.rejects(h.api.mfa(mfaToken, '123456', mode));
      assert.equal(h.calls.length, count);
      assert.equal(h.expired, 0);

      const password = h.api.login(otherOwner.email, 'password');
      answer(h.take('/auth/login'), { challenge_token: 'password-next', expires_in_seconds: 300 });
      await password;
      const next = h.api.mfa('password-next', '123456', mode);
      const value = session('password-other-account', { ...otherOwner, role: 'litigator' });
      answer(h.take(`/auth/mfa/${mode}`), value);
      assert.deepEqual(await next, value);
      assert.equal((await h.api.me()).bearer, 'Bearer password-other-account');
    }
  }
});

test('method changes and reset cancellation supersede late start proof or MFA without replacing another session', async () => {
  for (const stage of ['start', 'proof', 'mfa']) {
    for (const cancel of ['cancel', 'password', 'reset']) {
      const h = client();
      await establish(h, 'original');
      if (stage !== 'start') await start(h);
      if (stage === 'mfa') await prove(h);
      const old =
        stage === 'start'
          ? h.api.startCertificateLogin(selection)
          : stage === 'proof'
            ? h.api.proveCertificateLogin(token, signature)
            : h.api.mfa(mfaToken, '123456', 'totp');
      const rejected = superseded(old);
      const request = h.take(stage === 'mfa' ? '/auth/mfa/totp' : `${base}/${stage}`);
      if (cancel === 'reset') h.api.invalidateSession();
      else if (cancel === 'cancel') h.api.cancelAuthentication();
      else {
        const next = h.api.login('next@example.test', 'password');
        answer(h.take('/auth/login'), {
          challenge_token: 'password-next',
          expires_in_seconds: 300,
        });
        await next;
        await establish(h, 'replacement', 'password-next');
      }
      answer(
        request,
        stage === 'start' ? challenge() : stage === 'proof' ? mfa() : session('obsolete'),
      );
      await rejected;
      if (cancel === 'reset')
        await assert.rejects(h.api.me(), (error) => error.code === 'session_inactive');
      else
        assert.equal(
          (await h.api.me()).bearer,
          cancel === 'cancel' ? 'Bearer original' : 'Bearer replacement',
        );
      assert.equal(h.expired, 0);
    }
  }
});

test('late proof body cannot restore a cancelled MFA challenge and a new certificate start invalidates old password MFA', async () => {
  const h = client();
  await start(h);
  const body = deferred(),
    entered = deferred();
  const old = superseded(h.api.proveCertificateLogin(token, signature));
  h.take(`${base}/proof`).resolve({
    ok: true,
    status: 200,
    json() {
      entered.resolve();
      return body.promise;
    },
  });
  await entered.promise;
  h.api.cancelAuthentication();
  body.resolve(mfa());
  await old;
  const calls = h.calls.length;
  await assert.rejects(h.api.mfa(mfaToken, '123456', 'totp'));
  assert.equal(h.calls.length, calls);
  const password = h.api.login('owner@example.test', 'password');
  answer(h.take('/auth/login'), { challenge_token: 'password-old', expires_in_seconds: 300 });
  await password;
  const pendingMfa = superseded(h.api.mfa('password-old', '123456', 'totp'));
  const mfaRequest = h.take('/auth/mfa/totp');
  await start(h);
  answer(mfaRequest, session('obsolete-password-session'));
  await pendingMfa;
  assert.equal((await h.api.me()).bearer, null);
  await prove(h);
  await establish(h, 'current-certificate-session');
});
