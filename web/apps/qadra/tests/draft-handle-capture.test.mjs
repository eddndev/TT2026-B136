import test from 'node:test';
import assert from 'node:assert/strict';
import {
  descriptor,
  registry,
  attach,
  suspended,
  recover,
  attempt,
  deferred,
} from './fixtures/draft-registry.mjs';

test('one handle captures synchronously with an isolated projection and leaves other editors live', async () => {
  const store = registry();
  const source = { draft: { email: ' Partial@', role: 'litigator', rows: ['-'] }, uncertain: true };
  Object.defineProperty(source, 'password', {
    get() {
      assert.fail('Secret getter was read.');
    },
  });
  source.enrollment = { totp_secret_base32: 'excluded', recovery_codes: ['excluded'] };
  let calls = 0,
    otherCalls = 0;
  const handle = store.register(descriptor(), {
    fields: ['draft', 'uncertain'],
    capture: () => {
      calls++;
      return source;
    },
  });
  const other = store.register(descriptor({ resourceId: 'document-b' }), {
    fields: ['text'],
    capture: () => {
      otherCalls++;
      return { text: 'other editor' };
    },
  });
  assert.deepEqual(handle.capture(), { status: 'captured' });
  assert.equal(calls, 1);
  assert.equal(otherCalls, 0);
  assert.equal(store.active(), true);
  assert.deepEqual(
    store.pending().map((entry) => entry.key),
    [handle.key],
  );
  assert.equal(JSON.stringify(store.pending()).includes('Partial'), false);
  source.draft.email = 'changed';
  source.draft.rows[0] = 'changed';
  handle.dispose();
  assert.deepEqual(suspended(store).captured, [other.key]);
  assert.equal(otherCalls, 1);
  assert.deepEqual(await recover(store, handle.key), {
    draft: { email: ' Partial@', role: 'litigator', rows: ['-'] },
    uncertain: true,
  });
  assert.deepEqual(await recover(store, other.key), { text: 'other editor' });
});

test('capturing a handle does not invalidate another pending restore authorization', async () => {
  const store = registry(),
    gate = deferred(),
    applied = [];
  const pending = attach(store, descriptor({ resourceId: 'document-b' }), { text: 'saved' });
  suspended(store);
  const restoring = attempt(
    store,
    pending.key,
    () => gate.promise,
    (value) => applied.push(value),
  );
  const active = attach(store, descriptor(), { text: 'new capture' });
  assert.deepEqual(active.capture(), { status: 'captured' });
  assert.equal(store.active(), true);
  assert.deepEqual(applied, []);
  gate.resolve(true);
  assert.deepEqual(await restoring, { status: 'restored' });
  assert.deepEqual(applied, [{ text: 'saved' }]);
  assert.deepEqual(await recover(store, active.key), { text: 'new capture' });
});

test('capture never replaces an existing snapshot or calls a replacement projection', async () => {
  const store = registry();
  const original = attach(store, descriptor(), { text: 'original values' });
  assert.deepEqual(original.capture(), { status: 'captured' });
  original.dispose();
  const replacement = store.register(descriptor({ baseRevision: 4 }), {
    fields: ['text'],
    capture: () => assert.fail('Pending snapshot must win.'),
  });
  assert.deepEqual(replacement.capture(), { status: 'existing' });
  assert.equal(store.pending()[0].baseRevision, 3);
  replacement.dispose();
  assert.deepEqual(await recover(store, original.key), { text: 'original values' });
});

test('disposed, closed, denied and prior-session handles cannot recreate snapshots', async () => {
  const revoke = [
    (store, handle) => handle.dispose(),
    (store, handle) => store.closeEditor(handle.key),
    (store) => store.denyContext('case-a'),
    (store) => store.logout(),
    (store) => store.activate('account-a'),
    (store) => store.activate('account-b'),
    (store) => store.suspend(),
  ];
  for (const change of revoke) {
    const store = registry();
    let calls = 0;
    const handle = store.register(descriptor(), {
      fields: ['text'],
      capture: () => {
        calls++;
        return { text: 'original' };
      },
    });
    change(store, handle);
    const before = { calls, active: store.active(), pending: store.pending() };
    assert.deepEqual(handle.capture(), { status: 'stale' });
    assert.deepEqual({ calls, active: store.active(), pending: store.pending() }, before);
  }
  const store = registry(),
    old = attach(store);
  old.dispose();
  const current = attach(store, descriptor(), { text: 'current values' });
  assert.deepEqual(old.capture(), { status: 'stale' });
  assert.deepEqual(current.capture(), { status: 'captured' });
  assert.deepEqual(await recover(store, current.key), { text: 'current values' });
});

test('liveness, exact adapter and session generation are rechecked after projection', () => {
  for (const invalidate of [
    (store, handle) => handle.dispose(),
    (store, handle) => store.closeEditor(handle.key),
    (store) => store.denyContext('case-a'),
    (store) => store.logout(),
    (store) => store.activate('account-a'),
    (store) => store.activate('account-b'),
  ]) {
    const store = registry();
    let calls = 0,
      handle;
    handle = store.register(descriptor(), {
      fields: ['text'],
      capture: () => {
        calls++;
        invalidate(store, handle);
        return { text: 'must not revive' };
      },
    });
    assert.deepEqual(handle.capture(), { status: 'stale' });
    assert.equal(calls, 1);
    store.activate('account-a');
    assert.deepEqual(store.pending(), []);
  }
  const store = registry();
  let replacement, original;
  original = store.register(descriptor(), {
    fields: ['text'],
    capture: () => {
      original.dispose();
      replacement = attach(store, descriptor(), { text: 'replacement' });
      return { text: 'obsolete adapter' };
    },
  });
  assert.deepEqual(original.capture(), { status: 'stale' });
  assert.deepEqual(store.pending(), []);
  assert.deepEqual(replacement.capture(), { status: 'captured' });
});

test('unsafe or asynchronous projections fail atomically without retaining secrets or disabling siblings', async () => {
  let selectedReads = 0;
  const invalid = [
    () => {
      throw new Error('Unreported capture failure.');
    },
    () => Promise.resolve({ draft: 'late' }),
    () => ({ draft: { password: 'must not be copied' } }),
    () => ({ draft: { totp_secret_base32: 'must not be copied' } }),
    () => ({ draft: { otpauth_uri: 'must not be copied' } }),
    () => ({ draft: { recovery_codes: ['must not be copied'] } }),
    () => ({ draft: { access_token: 'must not be copied' } }),
    () => ({ draft: { callback() {} } }),
    () => ({
      get draft() {
        selectedReads++;
        return 'computed field';
      },
    }),
    () => ({}),
  ];
  for (const capture of invalid) {
    const store = registry();
    const bad = store.register(descriptor(), { fields: ['draft'], capture });
    const good = attach(store, descriptor({ resourceId: 'document-b' }));
    assert.deepEqual(bad.capture(), { status: 'failed' });
    assert.equal(selectedReads, 0);
    assert.equal(store.active(), true);
    assert.deepEqual(store.pending(), []);
    assert.deepEqual(good.capture(), { status: 'captured' });
    assert.deepEqual(await recover(store, good.key), { text: 'unfinished' });
  }
  for (const field of [
    'password',
    'totp_secret_base32',
    'otpauth_uri',
    'recovery_codes',
    'access_token',
  ])
    assert.throws(() =>
      registry().register(descriptor(), {
        fields: [field],
        capture: () => ({ [field]: 'must not be copied' }),
      }),
    );
});
