import assert from 'node:assert/strict';
import { createDraftRegistry } from '../../src/lib/draft-registry.mjs';
import { createSessionLifecycle } from '../../src/lib/session-lifecycle.mjs';

export const session = (change = {}) => ({
  user: { id: 'account-a', email: 'owner@example.test', role: 'owner' },
  policy: { absolute_ttl_seconds: 300, idle_ttl_seconds: 60 },
  server_now_unix_ms: 100000,
  absolute_expires_at_unix_ms: 400000,
  idle_expires_at_unix_ms: 160000,
  access_token: 'credential-not-for-state',
  ...change,
});
export const otherSession = () =>
  session({
    user: { id: 'account-b', email: 'client@example.test', role: 'client' },
  });

function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

export function harness() {
  let time = 1000,
    nextTimer = 0,
    guard = null,
    localOpen = false;
  const calls = [],
    events = [],
    states = [],
    waiting = new Map(),
    timers = new Map();
  const drafts = createDraftRegistry();
  function request(kind) {
    const call = { kind, ...deferred() };
    calls.push(call);
    waiting.get(calls.length - 1)?.resolve(call);
    events.push(kind);
    return call.promise;
  }
  const api = {
    setSessionGuard(callback) {
      guard = callback;
      return () => {
        if (guard === callback) guard = null;
      };
    },
    invalidateSession() {
      if (localOpen) events.push('invalidate');
      localOpen = false;
    },
    sessionStatus() {
      assert.equal(localOpen, true);
      return request('read');
    },
    recordActivity() {
      assert.equal(localOpen, true);
      assert.equal(guard(), true);
      return request('activity');
    },
    logout() {
      assert.equal(localOpen, true);
      const pending = request('logout');
      api.invalidateSession();
      return pending;
    },
  };
  const lifecycle = createSessionLifecycle({
    api,
    drafts,
    now: () => time,
    setTimer(fn, delay) {
      const id = ++nextTimer;
      timers.set(id, { fn, at: time + delay });
      return id;
    },
    clearTimer: (id) => timers.delete(id),
    onState(state) {
      states.push(state);
      events.push(`state:${state.phase}`);
    },
  });
  function draft(fail = false) {
    return drafts.register(
      {
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
      },
      {
        fields: ['text'],
        capture() {
          assert.equal(guard(), false);
          events.push('capture');
          if (fail) throw new Error('private draft must not reach notices');
          return { text: 'unfinished draft' };
        },
      },
    );
  }
  function begin(value = session(), receipt = { startedAt: time, receivedAt: time }) {
    localOpen = true;
    return lifecycle.acceptSession(value, receipt);
  }
  function advance(ms) {
    time += ms;
    for (const [id, timer] of [...timers]) {
      if (timer.at <= time) {
        timers.delete(id);
        timer.fn();
      }
    }
  }
  function nextCall(index) {
    if (calls[index]) return Promise.resolve(calls[index]);
    const next = deferred();
    waiting.set(index, next);
    return next.promise;
  }
  return {
    lifecycle,
    drafts,
    calls,
    events,
    states,
    timers,
    begin,
    draft,
    advance,
    nextCall,
    admitted: () => guard?.() === true,
    hasGuard: () => guard !== null,
    localOpen: () => localOpen,
    latest: () => states.at(-1),
  };
}
