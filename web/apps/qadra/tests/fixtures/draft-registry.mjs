import assert from 'node:assert/strict';
import { createDraftRegistry } from '../../src/lib/draft-registry.mjs';

export const descriptor = (change = {}) => ({
  principalId: 'account-a',
  contextId: 'case-a',
  editorKind: 'metadata',
  resourceId: 'document-a',
  action: 'edit',
  instanceId: null,
  ownerDraftKey: null,
  fieldPath: [],
  rowId: null,
  schemaVersion: 1,
  baseRevision: 3,
  ...change,
});

export function registry() {
  const result = createDraftRegistry();
  result.activate('account-a');
  return result;
}

export function attach(store, description = descriptor(), values = { text: 'unfinished' }) {
  return store.register(description, { fields: Object.keys(values), capture: () => values });
}

export function suspended(store) {
  const report = store.suspend();
  store.activate('account-a');
  return report;
}

export function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

export const allowed = async () => true;
export const attempt = (store, key, authorize, apply) => store.restore(key, { authorize, apply });

export async function recover(store, key) {
  let values;
  const result = await attempt(store, key, allowed, (saved) => {
    values = saved;
  });
  assert.equal(result.status, 'restored');
  return values;
}
