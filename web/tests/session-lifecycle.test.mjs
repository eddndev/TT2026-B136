import test from 'node:test';
import assert from 'node:assert/strict';
import { session, otherSession, harness } from './fixtures/session-lifecycle.mjs';

test('signed out admission is closed and a valid MFA starts only primitive public state', () => {
  const h = harness();
  assert.equal(h.admitted(), false);
  assert.equal(h.begin(), true);
  assert.equal(h.admitted(), true);
  assert.equal(h.latest().phase, 'active');
  assert.equal(h.latest().principalId, 'account-a');
  for (const state of h.states)
    assert.ok(
      Object.values(state).every(
        (value) => value === null || ['string', 'number', 'boolean'].includes(typeof value),
      ),
    );
  assert.equal(JSON.stringify(h.states).includes('credential-not-for-state'), false);
  assert.equal(h.calls.length, 0);
});

test('invalid or exhausted MFA metadata cannot mount an admitted application', () => {
  for (const value of [
    null,
    {},
    session({ idle_expires_at_unix_ms: null }),
    session({ idle_expires_at_unix_ms: 100500 }),
  ]) {
    const h = harness();
    assert.equal(h.begin(value, { startedAt: 0, receivedAt: 1000 }), false);
    assert.equal(h.admitted(), false);
    assert.equal(h.drafts.active(), false);
    assert.equal(h.latest().phase, 'expired');
    assert.ok(h.events.includes('invalidate'));
    assert.equal(h.timers.size, 0);
  }
});

test('expiry closes admission, captures before unmount and invalidates exactly once', () => {
  const h = harness();
  h.begin();
  h.draft();
  h.events.length = 0;
  h.lifecycle.expire('deadline');
  h.lifecycle.expire('rejected');
  assert.deepEqual(h.events, ['capture', 'invalidate', 'state:expired']);
  assert.equal(h.latest().principalId, null);
  assert.equal(h.admitted(), false);
  assert.equal(h.timers.size, 0);
});

test('failed capture reports a count without preventing local expiry or leaking values', () => {
  const h = harness();
  h.begin();
  h.draft(true);
  h.lifecycle.expire('rejected');
  assert.equal(h.latest().captureFailures, 1);
  assert.equal(h.latest().phase, 'expired');
  assert.equal(h.admitted(), false);
  assert.equal(JSON.stringify(h.states).includes('private draft'), false);
});

test('timer expiry suspends without making autonomous reads or activity requests', () => {
  const h = harness();
  h.begin();
  h.draft();
  h.advance(60000);
  h.advance(60000);
  assert.equal(h.latest().phase, 'expired');
  assert.equal(h.events.filter((event) => event === 'capture').length, 1);
  assert.deepEqual(h.calls, []);
});

test('same-account MFA offers drafts without applying them and another principal discards', () => {
  const h = harness();
  h.begin();
  const handle = h.draft();
  h.lifecycle.expire('deadline');
  h.begin();
  assert.deepEqual(
    h.drafts.pending().map((item) => item.key),
    [handle.key],
  );
  assert.equal(h.calls.length, 0);
  h.lifecycle.expire('deadline');
  h.begin(otherSession());
  assert.deepEqual(h.drafts.pending(), []);
  assert.equal(h.latest().principalId, 'account-b');
});

test('offline logout discards immediately and communicates uncertain remote revocation', async () => {
  const h = harness();
  h.begin();
  h.draft();
  const pending = h.lifecycle.logout();
  assert.equal(h.admitted(), false);
  assert.equal(h.latest().phase, 'signed-out');
  assert.ok(h.events.includes('invalidate'));
  assert.equal(h.events.includes('capture'), false);
  h.calls[0].reject(new Error('offline'));
  assert.equal(await pending, false);
  assert.equal(h.latest().logoutUncertain, true);
  h.begin();
  assert.deepEqual(h.drafts.pending(), []);
});

test('late logout failure never replaces a new principal or publishes an obsolete notice', async () => {
  const h = harness();
  h.begin();
  const pending = h.lifecycle.logout();
  h.begin(otherSession());
  const count = h.states.length;
  h.calls[0].reject(new Error('old logout'));
  await pending;
  assert.equal(h.states.length, count);
  assert.equal(h.latest().principalId, 'account-b');
  assert.equal(h.admitted(), true);
});

test('visibility return blocks business admission and coalesces a read without activity', async () => {
  const h = harness();
  h.begin();
  await h.lifecycle.visibilityChanged('hidden');
  assert.equal(h.admitted(), false);
  const first = h.lifecycle.visibilityChanged('visible');
  const second = h.lifecycle.visibilityChanged('visible');
  assert.equal(h.latest().phase, 'checking');
  assert.equal(h.admitted(), false);
  assert.deepEqual(
    h.calls.map((call) => call.kind),
    ['read'],
  );
  h.calls[0].resolve(session());
  assert.equal(await first, true);
  assert.equal(await second, true);
  assert.equal(h.admitted(), true);
});

test('an unconfirmed visibility check stays closed without retries until deadline', async () => {
  const h = harness();
  h.begin();
  h.draft();
  const pending = h.lifecycle.visibilityChanged('visible');
  h.calls[0].reject(new Error('offline'));
  assert.equal(await pending, false);
  assert.equal(h.latest().phase, 'checking');
  assert.equal(h.admitted(), false);
  h.advance(60000);
  assert.equal(h.latest().phase, 'expired');
  assert.equal(h.calls.length, 1);
  assert.equal(h.events.filter((event) => event === 'capture').length, 1);
});

test('a rejected visibility check captures once and closes the current session', async () => {
  const h = harness();
  h.begin();
  h.draft();
  const pending = h.lifecycle.visibilityChanged('visible');
  h.calls[0].reject(Object.assign(new Error('expired'), { status: 401 }));
  await pending;
  assert.equal(h.latest().phase, 'expired');
  assert.equal(h.events.filter((event) => event === 'capture').length, 1);
  assert.equal(h.admitted(), false);
});

test('hiding during a read prevents its reply from reopening admission', async () => {
  const h = harness();
  h.begin();
  const pending = h.lifecycle.visibilityChanged('visible');
  await h.lifecycle.visibilityChanged('hidden');
  h.calls[0].resolve(session());
  assert.equal(await pending, false);
  assert.equal(h.admitted(), false);
  assert.equal(h.latest().phase, 'hidden');
});

test('old status replies cannot reopen expiry or alter a replacement MFA', async () => {
  for (const replace of [false, true]) {
    const h = harness();
    h.begin();
    const pending = h.lifecycle.visibilityChanged('visible');
    h.lifecycle.expire('deadline');
    if (replace) h.begin(otherSession());
    const count = h.states.length;
    h.calls[0].resolve(session());
    assert.equal(await pending, false);
    assert.equal(h.states.length, count);
    assert.equal(h.admitted(), replace);
  }
});

test('only explicit trusted input can request activity and bursts never queue renewals', async () => {
  const h = harness();
  h.begin();
  for (const event of [
    {},
    { type: 'keydown', isTrusted: false },
    { type: 'focus', isTrusted: true },
    { type: 'visibilitychange', isTrusted: true },
  ])
    assert.equal(await h.lifecycle.activity(event), false);
  const first = h.lifecycle.activity({ type: 'keydown', isTrusted: true });
  assert.equal(await h.lifecycle.activity({ type: 'input', isTrusted: true }), false);
  assert.deepEqual(
    h.calls.map((call) => call.kind),
    ['activity'],
  );
  h.calls[0].resolve(session());
  assert.equal(await first, true);
  h.advance(10000);
  assert.equal(h.calls.length, 1);
});

test('hidden and checking phases do not accept activity even for trusted events', async () => {
  const h = harness();
  h.begin();
  await h.lifecycle.visibilityChanged('hidden');
  assert.equal(await h.lifecycle.activity({ type: 'pointerdown', isTrusted: true }), false);
  const pending = h.lifecycle.visibilityChanged('visible');
  assert.equal(await h.lifecycle.activity({ type: 'input', isTrusted: true }), false);
  assert.deepEqual(
    h.calls.map((call) => call.kind),
    ['read'],
  );
  h.calls[0].resolve(session());
  await pending;
});

test(
  'a visibility return after activity in flight still performs a distinct status read',
  { timeout: 1000 },
  async () => {
    const h = harness();
    h.begin();
    const activity = h.lifecycle.activity({ type: 'input', isTrusted: true });
    const checking = h.lifecycle.visibilityChanged('visible');
    h.calls[0].resolve(session());
    await activity;
    const read = await h.nextCall(1);
    assert.equal(read.kind, 'read');
    assert.equal(h.admitted(), false);
    read.resolve(session());
    assert.equal(await checking, true);
  },
);

test('disposing removes timers and the API guard and ignores outstanding checks', async () => {
  const h = harness();
  h.begin();
  const pending = h.lifecycle.visibilityChanged('visible');
  h.lifecycle.dispose();
  const count = h.states.length;
  h.calls[0].resolve(session());
  assert.equal(await pending, false);
  assert.equal(h.lifecycle.canAdmit(), false);
  assert.equal(h.hasGuard(), false);
  assert.equal(h.localOpen(), false);
  assert.equal(h.timers.size, 0);
  assert.equal(h.drafts.active(), false);
  assert.equal(h.states.length, count);
});
