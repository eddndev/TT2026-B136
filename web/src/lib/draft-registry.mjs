import {
  copyDescriptor,
  descriptorKey,
  nonemptyString,
  validateDescriptor,
} from './draft-descriptor.mjs';
import {
  asynchronous,
  captureProjection,
  cloneDraftValue,
  discardAsyncResult,
  validateFields,
} from './draft-values.mjs';

export function createDraftRegistry() {
  let principalId = null;
  let enabled = false;
  let epoch = 0;
  const adapters = new Map();
  const snapshots = new Map();
  const restores = new Map();

  function invalidate() {
    epoch++;
    restores.clear();
  }

  function activate(principal) {
    if (!nonemptyString(principal)) throw new TypeError('A draft owner is required.');
    invalidate();
    adapters.clear();
    if (principal !== principalId) snapshots.clear();
    principalId = principal;
    enabled = true;
  }

  function logout() {
    invalidate();
    enabled = false;
    principalId = null;
    adapters.clear();
    snapshots.clear();
  }

  function register(value, { fields, capture }) {
    if (!enabled || typeof capture !== 'function')
      throw new TypeError('An active draft owner and capture are required.');
    const lookup = (key) => adapters.get(key)?.descriptor ?? snapshots.get(key)?.descriptor;
    const descriptor = validateDescriptor(value, principalId, lookup);
    const key = descriptorKey(descriptor);
    if (adapters.has(key)) throw new TypeError('The draft editor is already registered.');
    const adapter = { descriptor, fields: validateFields(fields), capture };
    adapters.set(key, adapter);
    return {
      key,
      dispose() {
        if (adapters.get(key) === adapter) adapters.delete(key);
      },
    };
  }

  function suspend() {
    invalidate();
    enabled = false;
    const expected = epoch;
    const report = { captured: [], failed: [] };
    for (const [key, adapter] of [...adapters]) {
      if (epoch !== expected || adapters.get(key) !== adapter || snapshots.has(key)) continue;
      try {
        const values = captureProjection(adapter.capture, adapter.fields);
        if (epoch !== expected || adapters.get(key) !== adapter) continue;
        snapshots.set(key, { descriptor: adapter.descriptor, values });
        report.captured.push(key);
      } catch {
        report.failed.push({ key, code: 'capture_failed' });
      }
    }
    if (epoch === expected) adapters.clear();
    return report;
  }

  function pending() {
    if (!enabled) return [];
    return [...snapshots].map(([key, snapshot]) => ({
      key,
      ...copyDescriptor(snapshot.descriptor),
    }));
  }

  async function restore(key, { authorize, apply }) {
    const snapshot = snapshots.get(key);
    if (!enabled || !snapshot || snapshot.descriptor.principalId !== principalId)
      return { status: 'missing' };
    if (restores.has(key)) return { status: 'busy' };
    if (typeof authorize !== 'function' || typeof apply !== 'function')
      return { status: 'failed', code: 'invalid_adapter' };
    const expected = epoch;
    const lease = {};
    restores.set(key, lease);
    const current = () =>
      enabled &&
      epoch === expected &&
      snapshots.get(key) === snapshot &&
      restores.get(key) === lease;
    let stage = 'authorization';
    try {
      const allowed = await authorize(copyDescriptor(snapshot.descriptor));
      if (!current()) return { status: 'stale' };
      if (allowed !== true) return { status: 'denied' };
      stage = 'application';
      const result = apply(cloneDraftValue(snapshot.values));
      discardAsyncResult(result);
      if (!current()) return { status: 'stale' };
      if (asynchronous(result)) return { status: 'failed', code: 'asynchronous_apply' };
      snapshots.delete(key);
      return { status: 'restored' };
    } catch {
      return current() ? { status: 'failed', code: `${stage}_failed` } : { status: 'stale' };
    } finally {
      if (restores.get(key) === lease) restores.delete(key);
    }
  }

  function remove(key) {
    adapters.delete(key);
    snapshots.delete(key);
    restores.delete(key);
  }

  function entries() {
    return new Map([...snapshots, ...adapters]);
  }

  function closeEditor(key) {
    const closed = new Set([key]);
    const all = entries();
    let added = true;
    while (added) {
      added = false;
      for (const [child, entry] of all) {
        if (!closed.has(child) && closed.has(entry.descriptor.ownerDraftKey)) {
          closed.add(child);
          added = true;
        }
      }
    }
    closed.forEach(remove);
  }

  function denyContext(contextId) {
    for (const [key, entry] of entries()) {
      if (entry.descriptor.contextId === contextId) remove(key);
    }
  }

  return {
    activate,
    active: () => enabled,
    register,
    suspend,
    pending,
    restore,
    denyContext,
    closeEditor,
    logout,
  };
}
