import test from 'node:test';
import assert from 'node:assert/strict';
import { createDraftRegistry } from '../src/lib/draft-registry.mjs';
import {
  descriptor,
  registry,
  attach,
  suspended,
  deferred,
  allowed,
  attempt,
  recover,
} from './fixtures/draft-registry.mjs';

test('registration requires an active owner, explicit fields and a valid descriptor', () => {
  const store = createDraftRegistry();
  assert.throws(() => attach(store));
  store.activate('account-a');
  for (const change of [
    { principalId: 'account-b' },
    { contextId: '' },
    { schemaVersion: 0 },
    { fieldPath: 'supports/one' },
    { fieldPath: [''] },
    { rowId: 'root-without-parent' },
    { ownerDraftKey: 'missing-parent', fieldPath: ['supports'] },
    { ownerDraftKey: null, fieldPath: ['supports'] },
    { password: 'must-not-be-a-descriptor' },
  ])
    assert.throws(() => attach(store, descriptor(change)));
  for (const fields of [undefined, [], ['text', 'text']]) {
    assert.throws(() => store.register(descriptor(), { fields, capture: () => ({ text: 'x' }) }));
  }
});

test('duplicate active keys are rejected without replacing the first projection', async () => {
  const store = registry();
  const first = attach(store);
  assert.throws(() => attach(store, descriptor(), { text: 'replacement' }));
  suspended(store);
  assert.deepEqual(await recover(store, first.key), { text: 'unfinished' });
});

test('nested rows and fields have independent stable identities', async () => {
  const store = registry();
  const parent = attach(store);
  const children = [
    { fieldPath: ['supports'], rowId: 'row-a' },
    { fieldPath: ['supports'], rowId: 'row-b' },
    { fieldPath: ['evidence'], rowId: 'row-a' },
  ].map((part, index) =>
    attach(
      store,
      descriptor({
        editorKind: 'upload',
        resourceId: null,
        ownerDraftKey: parent.key,
        ...part,
      }),
      { text: `field-${index}` },
    ),
  );
  assert.equal(new Set([parent.key, ...children.map((child) => child.key)]).size, 4);
  assert.throws(() =>
    attach(
      store,
      descriptor({
        contextId: 'case-b',
        ownerDraftKey: parent.key,
        fieldPath: ['supports'],
      }),
    ),
  );
  suspended(store);
  for (const index of [2, 0, 1]) {
    assert.equal((await recover(store, children[index].key)).text, `field-${index}`);
  }
});

test('authorization receives only an isolated descriptor and gates all saved values', async () => {
  const store = registry(),
    gate = deferred(),
    original = descriptor();
  const handle = attach(store, original, { text: 'private draft' });
  suspended(store);
  let applications = 0;
  const authorize = (...args) => {
    assert.equal(args.length, 1);
    assert.deepEqual(args[0], original);
    assert.notEqual(args[0], original);
    args[0].fieldPath.push('external mutation');
    return gate.promise;
  };
  const pending = attempt(store, handle.key, authorize, (values) => {
    applications++;
    assert.equal(values.text, 'private draft');
  });
  assert.equal(applications, 0);
  assert.deepEqual(store.pending()[0].fieldPath, []);
  assert.equal(JSON.stringify(store.pending()).includes('private draft'), false);
  gate.resolve(true);
  assert.equal((await pending).status, 'restored');
  assert.equal(applications, 1);
});

test('only exact true authorizes; rejection and network errors retain the snapshot', async () => {
  for (const authorize of [
    async () => false,
    async () => undefined,
    async () => ({}),
    async () => 'true',
    async () => {
      throw new Error('offline');
    },
  ]) {
    const store = registry(),
      handle = attach(store);
    suspended(store);
    const applications = [];
    const result = await attempt(store, handle.key, authorize, (value) => applications.push(value));
    assert.ok(['denied', 'failed'].includes(result.status));
    assert.deepEqual(applications, []);
    assert.equal(store.pending().length, 1);
    assert.deepEqual(await recover(store, handle.key), { text: 'unfinished' });
  }
});

test('failed application retains an untouched snapshot for an explicit retry', async () => {
  const store = registry(),
    handle = attach(store, descriptor(), { draft: { text: 'original' } });
  suspended(store);
  const result = await attempt(store, handle.key, allowed, (saved) => {
    saved.draft.text = 'partial';
    throw new Error('cannot hydrate');
  });
  assert.equal(result.status, 'failed');
  assert.equal(store.pending().length, 1);
  assert.equal((await recover(store, handle.key)).draft.text, 'original');
});

test('an asynchronous apply cannot consume the pending snapshot', async () => {
  const store = registry(),
    handle = attach(store);
  suspended(store);
  const result = await attempt(store, handle.key, allowed, () => Promise.resolve());
  assert.equal(result.status, 'failed');
  assert.equal(store.pending().length, 1);
});

test('another account or logout discards drafts even if the old account returns', async () => {
  for (const leave of [(store) => store.activate('account-b'), (store) => store.logout()]) {
    const store = registry(),
      handle = attach(store);
    suspended(store);
    leave(store);
    store.activate('account-a');
    assert.deepEqual(store.pending(), []);
    const calls = [];
    const record = () => calls.push('called');
    assert.equal((await attempt(store, handle.key, record, record)).status, 'missing');
    assert.deepEqual(calls, []);
  }
});

test('account changes, logout and repeated expiry invalidate pending authorization', async () => {
  for (const change of [
    (store) => store.activate('account-b'),
    (store) => store.logout(),
    (store) => store.suspend(),
  ]) {
    const store = registry(),
      handle = attach(store),
      gate = deferred();
    suspended(store);
    const applied = [];
    const pending = attempt(
      store,
      handle.key,
      () => gate.promise,
      (value) => applied.push(value),
    );
    change(store);
    store.activate('account-a');
    gate.resolve(true);
    assert.equal((await pending).status, 'stale');
    assert.deepEqual(applied, []);
  }
});

test('two restore attempts cannot apply the same snapshot twice', async () => {
  const store = registry(),
    handle = attach(store),
    gate = deferred();
  suspended(store);
  let applied = 0;
  const first = attempt(
    store,
    handle.key,
    () => gate.promise,
    () => applied++,
  );
  const calls = [];
  const duplicate = await attempt(
    store,
    handle.key,
    () => calls.push('authorize'),
    () => calls.push('apply'),
  );
  assert.equal(duplicate.status, 'busy');
  assert.deepEqual(calls, []);
  gate.resolve(true);
  assert.equal((await first).status, 'restored');
  assert.equal(applied, 1);
});

test('context denial is exact and invalidates pending authorization', async () => {
  const store = registry(),
    a = attach(store),
    gate = deferred();
  const b = attach(store, descriptor({ contextId: 'case-a-extended' }), { text: 'other case' });
  suspended(store);
  const applied = [];
  const pending = attempt(
    store,
    a.key,
    () => gate.promise,
    (value) => applied.push(value),
  );
  store.denyContext('case-a');
  gate.resolve(true);
  assert.equal((await pending).status, 'stale');
  assert.deepEqual(applied, []);
  assert.deepEqual(await recover(store, b.key), { text: 'other case' });
  assert.deepEqual(store.pending(), []);
});

test('closing an editor cascades through owner relationships but preserves siblings', async () => {
  const store = registry(),
    parent = attach(store),
    gate = deferred();
  const child = attach(store, descriptor({ ownerDraftKey: parent.key, fieldPath: ['support'] }));
  const nested = attach(store, descriptor({ ownerDraftKey: child.key, fieldPath: ['upload'] }));
  const sibling = attach(store, descriptor({ resourceId: 'document-a-extended' }), {
    text: 'sibling',
  });
  suspended(store);
  const applied = [];
  const pending = attempt(
    store,
    nested.key,
    () => gate.promise,
    (value) => applied.push(value),
  );
  store.closeEditor(parent.key);
  gate.resolve(true);
  assert.equal((await pending).status, 'stale');
  assert.deepEqual(applied, []);
  assert.deepEqual(await recover(store, sibling.key), { text: 'sibling' });
  assert.deepEqual(store.pending(), []);
});

test('remounting does not replace a pending snapshot with an empty editor', async () => {
  const store = registry(),
    first = attach(store);
  store.suspend();
  first.dispose();
  store.activate('account-a');
  const replacement = attach(store, descriptor({ baseRevision: 4 }), { text: '' });
  suspended(store);
  assert.equal(first.key, replacement.key);
  assert.deepEqual(await recover(store, first.key), { text: 'unfinished' });
});

test('an old disposer cannot unregister a new adapter with the same key', async () => {
  const store = registry(),
    old = attach(store);
  old.dispose();
  const current = attach(store, descriptor(), { text: 'current' });
  old.dispose();
  suspended(store);
  assert.deepEqual(await recover(store, current.key), { text: 'current' });
  assert.deepEqual(store.pending(), []);
});
