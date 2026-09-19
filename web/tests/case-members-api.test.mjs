import test from 'node:test';
import assert from 'node:assert/strict';
import { createApi } from '../src/lib/api.mjs';
import {
  memberId,
  memberRecord,
  assignedMember,
  caseMemberPage,
  memberCaseId,
  otherMemberCaseId,
} from './fixtures/members.mjs';

const client = (value) => createApi(async () => Response.json(value)).caseMembers(memberCaseId);

test('assigned member lists preserve inactive membership and exact assignment time', async () => {
  const row = assignedMember(2, { active: false });
  const api = createApi(async (url) => {
    assert.equal(url, `/api/v1/cases/${memberCaseId}/members?limit=50&selection=assigned`);
    return Response.json(caseMemberPage([row]));
  }).caseMembers(memberCaseId);
  assert.deepEqual(await api.list(), caseMemberPage([row]));
});

test('available members are active and unassigned, with literal normalized email prefix', async () => {
  const row = { ...memberRecord(), assigned_at: null };
  const api = createApi(async (path) => {
    const url = new URL(path, 'http://localhost');
    assert.equal(url.searchParams.get('selection'), 'available');
    assert.equal(url.searchParams.get('email_prefix'), 'person');
    assert.equal(url.searchParams.get('cursor'), 'm1:opaque');
    return Response.json(caseMemberPage([row]));
  }).caseMembers(memberCaseId);
  assert.deepEqual(
    await api.list({
      selection: 'available',
      role: 'paralegal',
      email_prefix: ' PERSON ',
      cursor: 'm1:opaque',
    }),
    caseMemberPage([row]),
  );
  for (const changed of [{ active: false }, { assigned_at: '2026-09-19T10:00:00Z' }])
    await assert.rejects(() =>
      client(caseMemberPage([{ ...row, ...changed }])).list({ selection: 'available' }),
    );
});

test('member pages reject wrong case, timestamps, order, excessive results and missing fields', async () => {
  const row = assignedMember();
  for (const page of [
    caseMemberPage([row], null, otherMemberCaseId),
    caseMemberPage([{ ...row, assigned_at: null }]),
    caseMemberPage([{ ...row, assigned_at: '2026-02-30T10:00:00Z' }]),
    caseMemberPage([{ ...row, assigned_at: '2026-09-19T10:00:00-06:00' }]),
    caseMemberPage([assignedMember(3), row]),
    caseMemberPage([row, row]),
    caseMemberPage([], 'next'),
    caseMemberPage([memberRecord()]),
    { ...caseMemberPage(), total: 0 },
  ])
    await assert.rejects(() => client(page).list());
  await assert.rejects(() => client(caseMemberPage([row, assignedMember(3)])).list({ limit: 1 }));
  await assert.rejects(() => client(caseMemberPage([row])).list({ role: 'owner' }));
});

test('case member query rejects directory-only status and invalid scope before transport', async () => {
  let calls = 0;
  const api = createApi(async () => {
    calls++;
    return Response.json(caseMemberPage());
  });
  const scoped = api.caseMembers(memberCaseId);
  for (const query of [
    { status: 'active' },
    { selection: 'all' },
    { limit: 101 },
    { role: 'admin' },
    { email_prefix: '\n' },
    { cursor: 'a'.repeat(769) },
  ])
    await assert.rejects(() => scoped.list(query));
  await assert.rejects(async () => api.caseMembers('../cases').list());
  assert.equal(calls, 0);
});

test('assignment and removal reuse exact existing idempotent PUT and DELETE endpoints', async () => {
  const calls = [];
  const api = createApi(async (url, options) => {
    calls.push({ url, ...options });
    return new Response(null, { status: 204 });
  }).caseMembers(memberCaseId);
  assert.equal(await api.assign(memberId(2)), null);
  assert.equal(await api.remove(memberId(2)), null);
  assert.deepEqual(
    calls.map(({ url, method, body }) => ({ url, method, body })),
    ['PUT', 'DELETE'].map((method) => ({
      url: `/api/v1/cases/${memberCaseId}/members/${memberId(2)}`,
      method,
      body: undefined,
    })),
  );
  await assert.rejects(() => api.assign('../users'));
  assert.equal(calls.length, 2);
});

test('changing case scope discards late member rows and prevents stale assignment', async () => {
  let release,
    calls = 0;
  const api = createApi(() => {
    calls++;
    return new Promise((resolve) => {
      release = resolve;
    });
  }).caseMembers(memberCaseId);
  const pending = assert.rejects(api.list());
  api.dispose();
  release(Response.json(caseMemberPage([assignedMember()])));
  await pending;
  await assert.rejects(() => api.assign(memberId(2)));
  assert.equal(calls, 1);
});
