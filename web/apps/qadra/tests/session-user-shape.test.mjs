import test from 'node:test';
import assert from 'node:assert/strict';
import { freshMemberAccess } from '../src/lib/member-access-draft.mjs';
import { freshReportRequest } from '../src/lib/case-report-request-draft.mjs';

const ownerId = '11111111-1111-4111-8111-111111111111';
const memberId = '22222222-2222-4222-8222-222222222222';
const owner = { id: ownerId, email: 'owner@example.test', name: 'Ana Garc\u00eda', role: 'owner' };
const summary = {
  id: memberId,
  email: 'member@example.test',
  role: 'paralegal',
  active: true,
  revision: '1',
};

function memberAccess(principal) {
  return freshMemberAccess({
    api: { me: async () => structuredClone(principal) },
    members: { get: async () => structuredClone(summary) },
    principalId: ownerId,
    id: memberId,
    admitted: () => true,
  });
}

function reportRequest(principal) {
  const scoped = {
    litigators: async () => ({
      scope: 'office',
      litigators: [],
      has_more: false,
      next_after_id: null,
    }),
  };
  return freshReportRequest(
    { me: async () => structuredClone(principal) },
    scoped,
    { id: ownerId, role: 'owner' },
    () => true,
  );
}

test('the current account read is accepted with or without its display name', async () => {
  assert.deepEqual(await memberAccess(owner), summary);
  const { name: _name, ...nameless } = owner;
  assert.deepEqual(await memberAccess(nameless), summary);
  assert.deepEqual(await reportRequest(owner), { case_state: [] });
  assert.deepEqual(await reportRequest(nameless), { case_state: [] });
});

test('a malformed display name or an unknown key in the current account is refused', async () => {
  for (const principal of [
    { ...owner, name: 7 },
    { ...owner, name: null },
    { ...owner, name: '' },
    { ...owner, name: ' Ana' },
    { ...owner, name: 'Ana\u202e' },
    { ...owner, nickname: 'Ana' },
  ]) {
    await assert.rejects(memberAccess(principal), JSON.stringify(principal));
    await assert.rejects(reportRequest(principal), JSON.stringify(principal));
  }
});
