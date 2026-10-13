import test from 'node:test';
import assert from 'node:assert/strict';
import { discardDocumentDrafts } from '../src/lib/draft-document-access.mjs';
import { descriptor, registry, attach, suspended, deferred } from './fixtures/draft-registry.mjs';

function documents() {
  const store = registry();
  const first = attach(store, descriptor(), { text: 'first document draft' });
  const child = attach(
    store,
    descriptor({
      ownerDraftKey: first.key,
      fieldPath: ['support'],
      editorKind: 'upload',
    }),
    { text: 'nested draft' },
  );
  const second = attach(store, descriptor({ resourceId: 'document-b' }), { text: 'second draft' });
  const otherCase = attach(store, descriptor({ contextId: 'case-a-extended' }));
  suspended(store);
  return { store, first, child, second, otherCase };
}

const access = (handle) => ({ contextId: 'case-a', editorKey: handle.key });
const keys = (store) =>
  store
    .pending()
    .map((item) => item.key)
    .sort();

for (const [status, code] of [
  [404, 'document_not_found'],
  [403, 'permission_denied'],
]) {
  test(`${code} discards only the affected editor and its descendants`, () => {
    const h = documents();
    assert.equal(discardDocumentDrafts(h.store, access(h.first), { status, code }), 'editor');
    assert.deepEqual(keys(h.store), [h.second.key, h.otherCase.key].sort());
    assert.equal(
      h.store.pending().some((item) => item.key === h.child.key),
      false,
    );
  });
}

test('case_not_found discards both document drafts only in the exact case context', () => {
  const h = documents();
  const result = discardDocumentDrafts(h.store, access(h.first), {
    status: 404,
    code: 'case_not_found',
  });
  assert.equal(result, 'context');
  assert.deepEqual(keys(h.store), [h.otherCase.key]);
});

test('an unclassified document access denial never broadens to the whole case', () => {
  for (const failure of [{ status: 403 }, { status: 404, code: 'unknown_access_error' }]) {
    const h = documents();
    assert.equal(discardDocumentDrafts(h.store, access(h.first), failure), 'editor');
    assert.deepEqual(keys(h.store), [h.second.key, h.otherCase.key].sort());
  }
});

test('session expiry, network errors, case closure and revision conflicts retain drafts', () => {
  for (const failure of [
    new Error('offline'),
    { status: 401, code: 'invalid_session' },
    { status: 409, code: 'case_closed' },
    { status: 409, code: 'document_metadata_conflict' },
    { status: 500, code: 'case_not_found' },
  ]) {
    const h = documents();
    const before = keys(h.store);
    assert.equal(discardDocumentDrafts(h.store, access(h.first), failure), null);
    assert.deepEqual(keys(h.store), before);
  }
});

test('document denial invalidates its pending restore without canceling another document', async () => {
  const h = documents();
  const firstGate = deferred();
  const secondGate = deferred();
  const applied = [];
  const first = h.store.restore(h.first.key, {
    authorize: () => firstGate.promise,
    apply: () => applied.push('first'),
  });
  const second = h.store.restore(h.second.key, {
    authorize: () => secondGate.promise,
    apply: () => applied.push('second'),
  });
  discardDocumentDrafts(h.store, access(h.first), { status: 404, code: 'document_not_found' });
  firstGate.resolve(true);
  secondGate.resolve(true);
  assert.equal((await first).status, 'stale');
  assert.equal((await second).status, 'restored');
  assert.deepEqual(applied, ['second']);
});

test('denial unregisters only the affected live adapter before the next suspension', () => {
  const store = registry();
  const first = attach(store);
  const second = attach(store, descriptor({ resourceId: 'document-b' }));
  discardDocumentDrafts(store, access(first), { status: 403, code: 'permission_denied' });
  assert.deepEqual(suspended(store).captured, [second.key]);
  assert.deepEqual(keys(store), [second.key]);
});
