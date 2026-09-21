import test from 'node:test';
import assert from 'node:assert/strict';
import { createApi } from '../src/lib/api.mjs';
import { memberId, memberRecord, memberPage, memberActor } from './fixtures/members.mjs';

const client = (value) => createApi(async () => Response.json(value)).members();

test('directory queries normalize literal email prefixes and retain opaque cursor bytes', async () => {
  const row = memberRecord(),
    calls = [],
    next = 'u1:next+token%_';
  const api = createApi(async (url, options) => {
    calls.push({ url, ...options });
    return Response.json(memberPage([row], next));
  }).members();
  const query = {
    limit: 1,
    status: 'all',
    role: 'paralegal',
    email_prefix: ' PERSON ',
    cursor: 'u1:opaque+token%_',
  };
  assert.deepEqual(await api.list(query), memberPage([row], next));
  const url = new URL(calls[0].url, 'http://localhost');
  assert.equal(url.pathname, '/api/v1/users');
  assert.deepEqual(Object.fromEntries(url.searchParams), {
    ...query,
    limit: '1',
    email_prefix: 'person',
  });
  assert.equal(calls[0].method, 'GET');
  assert.equal(calls[0].cache, 'no-store');
  const defaults = createApi(async (path) => {
    assert.equal(path, '/api/v1/users?limit=50&status=active');
    return Response.json(memberPage());
  }).members();
  assert.deepEqual(await defaults.list(), memberPage());
});

test('directory rejects invalid filters before transport and treats SQL wildcards literally', async () => {
  const calls = [];
  const api = createApi(async (url) => {
    calls.push(url);
    return Response.json(memberPage());
  }).members();
  for (const query of [
    { limit: 0 },
    { limit: 101 },
    { limit: '1' },
    { status: 'pending' },
    { role: 'admin' },
    { email_prefix: 'a\nb' },
    { email_prefix: 'a'.repeat(255) },
    { email_prefix: 'caf\u00e9' },
    { cursor: '' },
    { cursor: 'a'.repeat(769) },
    { cursor: 'bad\nvalue' },
    { cursor: '\u00e1' },
    { selection: 'assigned' },
    { after_id: memberId(2) },
  ])
    await assert.rejects(() => api.list(query));
  assert.equal(calls.length, 0);
  await api.list({ email_prefix: ' %_\\ ' });
  assert.equal(new URL(calls[0], 'http://localhost').searchParams.get('email_prefix'), '%_\\');
});

test('member summaries preserve revisions above 2^53 and reject secret or malformed fields', async () => {
  const original = memberRecord();
  assert.deepEqual(await client(original).get(original.id), original);
  for (const change of [
    { id: memberId(3) },
    { email: 'UPPER@example.test' },
    { email: 'missing-at' },
    { role: 'administrator' },
    { active: 'true' },
    { revision: 9007199254740992 },
    { revision: '01' },
    { revision: '+1' },
    { revision: '9223372036854775808' },
    { password_hash: 'secret' },
    { auth_generation: 0 },
    { recovery_codes: [] },
  ])
    await assert.rejects(() => client({ ...original, ...change }).get(original.id));
  for (const revision of ['0', '9223372036854775807']) {
    const row = { ...original, revision };
    assert.deepEqual(await client(row).get(row.id), row);
  }
});

test('directory pages enforce order, bounds, filters and complete cursor envelopes', async () => {
  const first = memberRecord(2),
    second = memberRecord(3);
  for (const page of [
    memberPage([second, first]),
    memberPage([first, first]),
    memberPage([], 'next'),
    { ...memberPage([first]), next_cursor: 'next' },
    { ...memberPage([first]), has_more: 'false' },
    { ...memberPage(), total: 0 },
    memberPage([{ ...first, active: false }]),
    memberPage([first], '\n'),
  ])
    await assert.rejects(() => client(page).list());
  await assert.rejects(() => client(memberPage([first, second])).list({ limit: 1 }));
  await assert.rejects(() => client(memberPage([first])).list({ role: 'owner' }));
  await assert.rejects(() => client(memberPage([first])).list({ email_prefix: 'missing' }));
  assert.deepEqual(
    await client(memberPage([{ ...first, active: false }])).list({ status: 'inactive' }),
    memberPage([{ ...first, active: false }]),
  );
});

test('access changes send exact decimal CAS and validate the confirmed state', async () => {
  const before = memberRecord(),
    input = { expected_revision: before.revision, role: 'litigator', active: false };
  const after = { ...before, role: 'litigator', active: false, revision: '9007199254740994' };
  const api = createApi(async (url, options) => {
    assert.equal(url, `/api/v1/users/${before.id}/access`);
    assert.equal(options.method, 'PUT');
    assert.deepEqual(JSON.parse(options.body), input);
    return Response.json(after);
  }).members();
  assert.deepEqual(await api.changeAccess(before.id, input), after);
  for (const changed of [
    { id: memberId(3) },
    { role: 'owner' },
    { active: true },
    { revision: '9007199254740995' },
  ])
    await assert.rejects(() => client({ ...after, ...changed }).changeAccess(before.id, input));
  assert.deepEqual(
    await client(before).changeAccess(before.id, {
      expected_revision: before.revision,
      role: before.role,
      active: before.active,
    }),
    before,
  );
});

test('access commands reject unknown fields, invalid identities and noncanonical revisions before requests', async () => {
  let calls = 0;
  const api = createApi(async () => {
    calls++;
    return Response.json(memberRecord());
  }).members();
  const valid = { expected_revision: '0', role: 'owner', active: true };
  for (const input of [
    { ...valid, expected_revision: 0 },
    { ...valid, expected_revision: '00' },
    { ...valid, expected_revision: '9223372036854775808' },
    { ...valid, role: 'admin' },
    { ...valid, active: null },
    { ...valid, email: 'other@example.test' },
    { ...valid, password: 'never-accepted' },
    { role: 'owner', active: true },
  ])
    await assert.rejects(() => api.changeAccess(memberId(2), input));
  await assert.rejects(() => api.get('../users'));
  await assert.rejects(() => api.changeAccess('not-a-uuid', valid));
  assert.equal(calls, 0);
});

test('revision conflicts and last Owner errors remain typed and never trigger an automatic retry', async () => {
  for (const code of [
    'user_revision_conflict',
    'last_active_owner',
    'user_access_version_exhausted',
  ]) {
    let calls = 0;
    const api = createApi(async () => {
      calls++;
      return Response.json({ error: { code } }, { status: 409 });
    }).members();
    await assert.rejects(
      () =>
        api.changeAccess(memberId(2), { expected_revision: '1', role: 'client', active: false }),
      (error) => error.code === code && error.status === 409,
    );
    assert.equal(calls, 1);
  }
});

test('disposed directory scopes reject late results and cannot issue another request', async () => {
  let release,
    calls = 0;
  const api = createApi(() => {
    calls++;
    return new Promise((resolve) => {
      release = resolve;
    });
  }).members();
  const pending = assert.rejects(api.list());
  api.dispose();
  release(Response.json(memberPage([memberRecord()])));
  await pending;
  await assert.rejects(() => api.get(memberId(2)));
  assert.equal(calls, 1);
});

test('a real self access change clears the local session without depending on remote logout', async () => {
  const calls = [],
    after = { ...memberActor, role: 'paralegal', revision: '5' };
  let expired = 0;
  const api = createApi(
    async (url, options) => {
      calls.push({ url, ...options });
      if (url.endsWith('/totp'))
        return Response.json({ access_token: 'old-token', user: memberActor });
      if (url.endsWith('/access')) return Response.json(after);
      return Response.json(memberActor);
    },
    () => expired++,
  );
  await api.mfa('challenge', '123456', 'totp');
  assert.deepEqual(
    await api.members().changeAccess(memberActor.id, {
      expected_revision: '4',
      role: 'paralegal',
      active: true,
    }),
    after,
  );
  assert.equal(expired, 1);
  await api.me();
  assert.equal(calls.at(-1).headers.Authorization, undefined);
  assert.equal(
    calls.some((call) => call.url.endsWith('/logout')),
    false,
  );
});

test('a no-op on the current self revision preserves the current session', async () => {
  const calls = [];
  let expired = 0;
  const api = createApi(
    async (url, options) => {
      calls.push({ url, ...options });
      return Response.json(
        url.endsWith('/totp') ? { access_token: 'current-token', user: memberActor } : memberActor,
      );
    },
    () => expired++,
  );
  await api.mfa('challenge', '123456', 'totp');
  await api
    .members()
    .changeAccess(memberActor.id, { expected_revision: '4', role: 'owner', active: true });
  await api.me();
  assert.equal(expired, 0);
  assert.equal(calls.at(-1).headers.Authorization, 'Bearer current-token');
});
