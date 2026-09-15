import test from 'node:test';
import assert from 'node:assert/strict';
import { createApi } from '../src/lib/api.mjs';

for (const status of [200, 401]) {
  test(`a late HTTP ${status} response cannot affect a replacement session`, async () => {
    let release;
    let sequence = 0;
    let expired = 0;
    const api = createApi(
      async (url, options) => {
        if (url.endsWith('/mfa/totp'))
          return Response.json({ access_token: `token-${++sequence}` });
        if (url.endsWith('/logout')) return new Response(null, { status: 204 });
        if (url.endsWith('/me')) return Response.json({ bearer: options.headers.Authorization });
        return new Promise((resolve) => (release = resolve));
      },
      () => expired++,
    );
    await api.mfa('challenge', '123456', 'totp');
    const pending = api.caseDocuments('case-id').verify('document');
    await api.logout();
    await api.mfa('new-challenge', '123456', 'totp');
    release(
      Response.json(
        status === 200
          ? { verdict: 'valid' }
          : {
              error: { code: 'invalid_session' },
            },
        { status },
      ),
    );
    await assert.rejects(pending, /La sesi\u00f3n de esta solicitud termin\u00f3/);
    assert.equal(expired, 0);
    assert.equal((await api.me()).bearer, 'Bearer token-2');
  });
}
