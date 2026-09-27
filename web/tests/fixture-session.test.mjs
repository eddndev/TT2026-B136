import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { currentLivePlan } from '../../scripts/web-live-plan.mjs';

for (const partition of ['1/3', '2/3', '3/3', '']) {
  test(`fixture authentication uses an issued, unreserved code: ${partition || 'full'}`, async (t) => {
    const directory = await mkdtemp(join(tmpdir(), 'tt-fixture-session-'));
    const previousEnv = { ...process.env };
    const previousFetch = globalThis.fetch;
    t.after(async () => {
      globalThis.fetch = previousFetch;
      process.env = previousEnv;
      await rm(directory, { recursive: true, force: true });
    });
    process.env.TT_WEB_LIVE_SHARD = partition;
    process.env.TT_WEB_FIXTURES = join(directory, 'fixture.json');
    process.env.API_PROXY_TARGET = 'http://fixture.invalid';
    process.env.IDENTITY_TEST_DATABASE_URL = 'stubbed-no-database';
    const needed = new Set(currentLivePlan().fixtures);
    const account = (name) => ({
      email: `${name}@example.com`,
      password: 'synthetic fixture password',
      recoveryCodes: Array.from({ length: 8 }, (_, index) => `${name}-code-${index}`),
    });
    const fixture = account('bootstrap');
    const reserved = new Set(fixture.recoveryCodes.slice(0, 5));
    if (needed.has('caseAdministration')) reserved.add(fixture.recoveryCodes[5]);
    if (needed.has('caseStages')) {
      reserved.add(fixture.recoveryCodes[6]);
      fixture.caseStages = { owner: account('stages') };
      for (const code of fixture.caseStages.owner.recoveryCodes.slice(0, 4)) reserved.add(code);
    }
    if (needed.has('participants')) reserved.add(fixture.recoveryCodes[7]);
    await writeFile(process.env.TT_WEB_FIXTURES, JSON.stringify(fixture));
    let selected;
    let authenticated = false;
    let loggedOut = false;
    globalThis.fetch = async (url, options) => {
      const path = new URL(url).pathname;
      const body = options.body && JSON.parse(options.body);
      if (path === '/api/v1/auth/login') {
        selected = [fixture, fixture.caseStages?.owner].find(
          (candidate) => candidate?.email === body.email,
        );
        assert.ok(selected, 'login must use a prepared account');
        return Response.json({ challenge_token: 'synthetic-challenge' });
      }
      if (path === '/api/v1/auth/mfa/recovery') {
        assert.ok(selected.recoveryCodes.includes(body.code), 'code must exist');
        assert.ok(!reserved.has(body.code), 'code must not belong to another consumer');
        authenticated = true;
        return Response.json({ access_token: 'synthetic-session' });
      }
      if (path === '/api/v1/auth/logout') {
        loggedOut = true;
        return new Response(null, { status: 204 });
      }
      assert.equal(path, '/api/v1/users');
      assert.equal(options.headers.Authorization, 'Bearer synthetic-session');
      throw new Error('fixture provisioning boundary');
    };
    const script = new URL('../../scripts/web-hearing-fixtures.mjs', import.meta.url);
    script.searchParams.set('partition', partition);
    await assert.rejects(import(script.href), /fixture provisioning boundary/);
    assert.equal(authenticated, true);
    assert.equal(loggedOut, true);
  });
}
