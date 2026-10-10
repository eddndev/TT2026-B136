import test from 'node:test';
import assert from 'node:assert/strict';
import { createDraftRegistry } from '../src/lib/draft-registry.mjs';
import { createSigningDraft } from '../src/lib/individual-signing-draft.mjs';
import { accountId, context, operationId } from './individual-signing-fixture.mjs';

function session() {
  const registry = createDraftRegistry();
  registry.activate(accountId);
  return {
    registry,
    principal: () => ({ id: accountId, role: 'litigator' }),
    canAdmit: () => true,
  };
}
function draft(current, selected = context) {
  return createSigningDraft(current, selected, 'document', () => ({
    state: { operationId, mode: 'uncertain' },
    digest: selected.digest,
  }));
}

test('uncertain signing survives navigation only for exact account, case and version without sending', async () => {
  const current = session(),
    first = draft(current);
  first.register();
  first.dispose(true);
  assert.equal(current.registry.pending().length, 1);
  const wrongVersion = draft(current, { ...context, version: 8 });
  assert.equal(wrongVersion.pending(), null);
  let authorize = 0,
    restored;
  const resumed = draft(current);
  assert.ok(resumed.pending());
  const result = await resumed.restore(
    async () => {
      authorize++;
      return true;
    },
    (value) => {
      restored = value;
    },
  );
  assert.equal(result.status, 'restored');
  assert.equal(authorize, 1);
  assert.deepEqual(restored.state, { operationId, mode: 'uncertain' });
  assert.equal(current.registry.pending().length, 0);
  resumed.close();
  resumed.dispose();
});

test('changed digest or denied fresh authorization never restores signed proof', async () => {
  const current = session(),
    first = draft(current);
  first.register();
  first.dispose(true);
  let applied = false;
  const changed = draft(current, { ...context, digest: '0'.repeat(64) });
  await changed.restore(
    async () => true,
    () => {
      applied = true;
    },
  );
  assert.equal(applied, false);
  const denied = draft(current);
  await denied.restore(
    async () => false,
    () => {
      applied = true;
    },
  );
  assert.equal(applied, false);
  assert.equal(current.registry.pending().length, 1);
  current.registry.activate('22222222-2222-4222-8222-222222222222');
  assert.equal(current.registry.pending().length, 0);
});

test('explicit discard clears exact operation and ordinary unfinished editor is not preserved on navigation', () => {
  const current = session(),
    first = draft(current);
  first.register();
  first.dispose(false);
  assert.equal(current.registry.pending().length, 0);
  const second = draft(current);
  second.register();
  second.dispose(true);
  const third = draft(current);
  third.close();
  assert.equal(current.registry.pending().length, 0);
});
