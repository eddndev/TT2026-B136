import test from 'node:test';
import assert from 'node:assert/strict';
import { createSessionMonitor } from '../src/lib/session-monitor.mjs';

function session(change = {}) {
  return {
    user: { id: 'account-a', role: 'owner' },
    policy: { absolute_ttl_seconds: 300, idle_ttl_seconds: 60 },
    server_now_unix_ms: 100000,
    absolute_expires_at_unix_ms: 400000,
    idle_expires_at_unix_ms: 160000,
    ...change,
  };
}

function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

function harness() {
  let time = 1000,
    nextId = 0;
  const timers = new Map(),
    calls = [],
    expired = [],
    errors = [];
  const monitor = createSessionMonitor({
    api: {
      sessionStatus() {
        const reply = deferred();
        calls.push({ kind: 'read', reply });
        return reply.promise;
      },
      recordActivity() {
        const reply = deferred();
        calls.push({ kind: 'activity', reply });
        return reply.promise;
      },
    },
    now: () => time,
    setTimer(fn, delay) {
      const id = ++nextId;
      timers.set(id, { at: time + delay, fn });
      return id;
    },
    clearTimer(id) {
      timers.delete(id);
    },
    onExpired: (reason) => expired.push(reason),
    onError: (error) => errors.push(error),
    activityIntervalMs: 10000,
  });
  function advance(ms) {
    time += ms;
    for (const [id, timer] of [...timers]) {
      if (timer.at <= time) {
        timers.delete(id);
        timer.fn();
      }
    }
  }
  const begin = (value = session()) => monitor.start(value, { startedAt: time, receivedAt: time });
  return { monitor, begin, advance, calls, expired, errors, timers };
}

test('an idle session expires once without making autonomous activity requests', () => {
  const h = harness();
  h.begin();
  h.advance(59999);
  assert.equal(h.monitor.active(), true);
  assert.equal(h.calls.length, 0);
  h.advance(1);
  h.advance(60000);
  assert.equal(h.monitor.active(), false);
  assert.deepEqual(h.expired, ['deadline']);
  assert.equal(h.calls.length, 0);
  assert.equal(h.timers.size, 0);
});

test('explicit activity waits for confirmation before changing the local deadline', async () => {
  const h = harness();
  h.begin();
  h.advance(20000);
  const pending = h.monitor.activity();
  assert.equal(h.calls[0].kind, 'activity');
  assert.equal(h.monitor.remaining(), 40000);
  h.advance(200);
  h.calls[0].reply.resolve(
    session({ server_now_unix_ms: 120100, idle_expires_at_unix_ms: 180100 }),
  );
  assert.equal(await pending, true);
  assert.equal(h.monitor.remaining(), 59800);
});

test('bursts are bounded and never schedule a delayed renewal without another event', async () => {
  const h = harness();
  h.begin();
  const first = h.monitor.activity();
  assert.equal(await h.monitor.activity(), false);
  h.calls[0].reply.resolve(session());
  await first;
  h.advance(9999);
  assert.equal(await h.monitor.activity(), false);
  h.advance(1);
  assert.equal(h.calls.length, 1);
  const second = h.monitor.activity();
  assert.equal(h.calls.length, 2);
  h.calls[1].reply.resolve(
    session({ server_now_unix_ms: 110000, idle_expires_at_unix_ms: 170000 }),
  );
  await second;
});

test('a visibility recheck reads session state without declaring activity', async () => {
  const h = harness();
  h.begin();
  h.advance(10000);
  const pending = h.monitor.refresh();
  assert.equal(h.calls[0].kind, 'read');
  h.calls[0].reply.resolve(session({ server_now_unix_ms: 110000 }));
  await pending;
  assert.equal(h.monitor.remaining(), 50000);
});

test('a faster status response cannot extend a confirmed absolute deadline', async () => {
  const h = harness();
  const absolute = {
    policy: { absolute_ttl_seconds: 300, idle_ttl_seconds: null },
    idle_expires_at_unix_ms: null,
  };
  h.monitor.start(session(absolute), { startedAt: 0, receivedAt: 1000 });
  h.advance(10000);
  const pending = h.monitor.refresh();
  h.calls[0].reply.resolve(session({ ...absolute, server_now_unix_ms: 110000 }));
  assert.equal(await pending, true);
  assert.equal(h.monitor.remaining(), 289000);
});

test('reads and activity preserve the conservative clock offset established at login', async () => {
  const h = harness();
  h.monitor.start(session(), { startedAt: 0, receivedAt: 1000 });
  h.advance(10000);
  const read = h.monitor.refresh();
  h.calls[0].reply.resolve(session({ server_now_unix_ms: 110000 }));
  assert.equal(await read, true);
  assert.equal(h.monitor.remaining(), 49000);
  const activity = h.monitor.activity();
  h.calls[1].reply.resolve(
    session({ server_now_unix_ms: 110000, idle_expires_at_unix_ms: 170000 }),
  );
  assert.equal(await activity, true);
  assert.equal(h.monitor.remaining(), 59000);
});

test('network failure retains the last confirmed deadline and performs no retry', async () => {
  const h = harness();
  h.begin();
  h.advance(10000);
  const pending = h.monitor.activity();
  h.calls[0].reply.reject(new Error('offline'));
  assert.equal(await pending, false);
  assert.equal(h.errors.length, 1);
  assert.equal(h.monitor.remaining(), 50000);
  h.advance(50000);
  assert.equal(h.calls.length, 1);
  assert.deepEqual(h.expired, ['deadline']);
});

test('a late success cannot resurrect an expired session', async () => {
  const h = harness();
  h.begin();
  h.advance(59000);
  const pending = h.monitor.activity();
  h.advance(1000);
  h.calls[0].reply.resolve(
    session({ server_now_unix_ms: 159000, idle_expires_at_unix_ms: 219000 }),
  );
  assert.equal(await pending, false);
  assert.equal(h.monitor.active(), false);
  assert.deepEqual(h.expired, ['deadline']);
});

test('a stopped monitor ignores late success and failure from its old requests', async () => {
  for (const failure of [false, true]) {
    const h = harness();
    h.begin();
    const pending = h.monitor.refresh();
    h.monitor.stop();
    if (failure) h.calls[0].reply.reject(Object.assign(new Error('old'), { status: 401 }));
    else h.calls[0].reply.resolve(session());
    assert.equal(await pending, false);
    assert.deepEqual(h.expired, []);
    assert.equal(h.timers.size, 0);
  }
});

test('responses for an old login cannot expire or replace a new principal', async () => {
  const h = harness();
  h.begin();
  const old = h.monitor.refresh();
  h.begin(session({ user: { id: 'account-b', role: 'client' } }));
  h.calls[0].reply.reject(Object.assign(new Error('old'), { status: 401 }));
  await old;
  assert.equal(h.monitor.active(), true);
  assert.deepEqual(h.expired, []);
});

test('confirmed rejection expires once and blocks subsequent activity', async () => {
  const h = harness();
  h.begin();
  const pending = h.monitor.refresh();
  h.calls[0].reply.reject(Object.assign(new Error('rejected'), { status: 401 }));
  await pending;
  assert.deepEqual(h.expired, ['rejected']);
  assert.equal(await h.monitor.activity(), false);
  assert.equal(h.calls.length, 1);
});

test('identity, absolute deadline or policy changes never extend the current session', async () => {
  for (const change of [
    { user: { id: 'account-b', role: 'owner' } },
    { absolute_expires_at_unix_ms: 401000, server_now_unix_ms: 101000 },
    { policy: { absolute_ttl_seconds: 300, idle_ttl_seconds: 120 } },
  ]) {
    const h = harness();
    h.begin();
    const pending = h.monitor.refresh();
    h.calls[0].reply.resolve(session(change));
    assert.equal(await pending, false);
    assert.deepEqual(h.expired, ['invalid-state']);
  }
});

test('a response exhausted in transport is not a new usable session', () => {
  const h = harness();
  h.monitor.start(session({ idle_expires_at_unix_ms: 100500 }), { startedAt: 0, receivedAt: 1000 });
  assert.deepEqual(h.expired, ['deadline']);
  assert.equal(h.monitor.active(), false);
});

test('malformed successful metadata closes the local session instead of retaining access', async () => {
  for (const reply of [null, {}, session({ idle_expires_at_unix_ms: null })]) {
    const h = harness();
    h.begin();
    const pending = h.monitor.refresh();
    h.calls[0].reply.resolve(reply);
    assert.equal(await pending, false);
    assert.equal(h.monitor.active(), false);
    assert.deepEqual(h.expired, ['invalid-state']);
  }
});

test('an absolute-only session never emits activity and still expires on schedule', async () => {
  const h = harness();
  h.begin(
    session({
      policy: { absolute_ttl_seconds: 300, idle_ttl_seconds: null },
      idle_expires_at_unix_ms: null,
    }),
  );
  assert.equal(await h.monitor.activity(), false);
  assert.equal(h.calls.length, 0);
  h.advance(300000);
  assert.deepEqual(h.expired, ['deadline']);
});
