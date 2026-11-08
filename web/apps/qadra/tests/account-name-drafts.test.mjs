import test from 'node:test';
import assert from 'node:assert/strict';
import { createDraftRegistry } from '../src/lib/draft-registry.mjs';
import { createMemberEnrollmentDraft } from '../src/lib/member-enrollment-draft.mjs';
import { invitationOwnerDraft } from '../src/lib/invitation-owner-draft.mjs';

const ownerId = '11111111-1111-4111-8111-111111111111';
const descriptor = (contextId, action, schemaVersion) => ({
  principalId: ownerId,
  contextId,
  editorKind: contextId,
  resourceId: null,
  action,
  instanceId: null,
  ownerDraftKey: null,
  fieldPath: [],
  rowId: null,
  schemaVersion,
  baseRevision: null,
});

function session() {
  const registry = createDraftRegistry();
  registry.activate(ownerId);
  return {
    registry,
    principal: () => ({ id: ownerId, role: 'owner' }),
    canAdmit: () => true,
    reopen() {
      const report = registry.suspend();
      registry.activate(ownerId);
      return report;
    },
  };
}
const api = { me: async () => ({ id: ownerId, email: 'o@example.test', role: 'owner' }) };

const legacyEnrollment = (s) =>
  s.registry.register(descriptor('member-enrollment', 'create', 1), {
    fields: ['email', 'role', 'uncertain'],
    capture: () => ({ email: 'a@example.test', role: 'paralegal', uncertain: true }),
  });
const legacyInvitation = (s) =>
  s.registry.register(descriptor('member-invitations', 'issue-or-resend', 1), {
    fields: ['intent'],
    capture: () => ({ intent: { kind: 'issue', id: ownerId, data: {} } }),
  });

test('an enrollment draft saved before account names existed is refused and discarded', async () => {
  const s = session();
  legacyEnrollment(s);
  s.reopen();
  const draft = createMemberEnrollmentDraft({ session: s, principalId: ownerId, capture: null });
  let applied = false;
  const result = await draft.restore(
    async () => true,
    () => {
      applied = true;
    },
  );
  assert.equal(result.status, 'denied');
  assert.equal(applied, false);
  assert.equal(s.registry.pending().length, 1);
  assert.equal(draft.pending(), false);
  assert.deepEqual(s.registry.pending(), []);
  const named = createMemberEnrollmentDraft({
    session: s,
    principalId: ownerId,
    capture: () => ({ name: 'Ana', email: 'a@example.test', role: 'client', uncertain: true }),
  });
  named.register();
  assert.deepEqual(s.reopen().captured.length, 1);
  assert.equal(named.pending(), true);
});

test('an enrollment draft keeps the typed account name and refuses a non-string one', async () => {
  const s = session();
  let saved = { name: 'Ana', email: 'a@example.test', role: 'client', uncertain: true };
  const first = createMemberEnrollmentDraft({
    session: s,
    principalId: ownerId,
    capture: () => saved,
  });
  first.register();
  assert.deepEqual(s.reopen().failed, []);
  const second = createMemberEnrollmentDraft({ session: s, principalId: ownerId, capture: null });
  assert.equal(second.pending(), true);
  let applied;
  const result = await second.restore(
    async () => true,
    (value) => {
      applied = value;
    },
  );
  assert.equal(result.status, 'restored');
  assert.deepEqual(applied, saved);
  second.close();
  for (const name of ['', 'Ana Garc\u00eda']) {
    saved = { ...saved, name };
    const next = createMemberEnrollmentDraft({
      session: s,
      principalId: ownerId,
      capture: () => saved,
    });
    next.register();
    assert.equal(s.reopen().captured.length, 1, name);
    next.close();
  }
  for (const value of [
    { ...saved, name: 7 },
    { email: saved.email, role: 'client', uncertain: true },
  ]) {
    saved = value;
    const third = createMemberEnrollmentDraft({
      session: s,
      principalId: ownerId,
      capture: () => saved,
    });
    third.register();
    assert.equal(s.reopen().failed.length, 1);
  }
});

test('an invitation intent saved before account names existed is refused and discarded', async () => {
  const s = session();
  legacyInvitation(s);
  s.reopen();
  const legacy = invitationOwnerDraft(s, api, ownerId, () => ({}));
  let applied = false;
  const result = await legacy.restore(() => {
    applied = true;
  });
  assert.equal(result.status, 'denied');
  assert.equal(applied, false);
  assert.equal(legacy.pending(), false);
  assert.deepEqual(s.registry.pending(), []);
  const current = invitationOwnerDraft(s, api, ownerId, () => ({ intent: null }));
  current.register();
  s.reopen();
  assert.equal(invitationOwnerDraft(s, api, ownerId, () => ({})).pending(), true);
});
