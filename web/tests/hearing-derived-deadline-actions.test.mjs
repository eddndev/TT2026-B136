import test from 'node:test';
import assert from 'node:assert/strict';
import { createHearingDerivedDeadlineActions } from '../src/lib/hearing-derived-deadline-actions.mjs';
import {
  derivedReady,
  derivedRecord,
  principal,
  digest,
  clone,
} from './fixtures/hearing-derived-deadline-unit.mjs';

function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
function setup(mode = 'draft') {
  const ready = derivedReady(),
    record = derivedRecord(ready),
    calls = [],
    updates = [],
    completed = [],
    failed = [];
  let allowed = true;
  const state = {
    caseId: ready.command.case_id,
    hearingId: ready.command.result.hearing_id,
    user: principal(),
    mode,
    busy: false,
    pending: false,
    disabled: false,
    blocked: false,
    closed: false,
    prepared: mode === 'review' ? clone(ready) : null,
    last: ['uncertain', 'conflict'].includes(mode) ? clone(ready) : null,
    acknowledged: false,
    retryAvailable: false,
    error: '',
  };
  const scoped = {
    async prepare(command, actor) {
      calls.push(['prepare', clone(command), clone(actor)]);
      return clone(ready);
    },
    async submit(value, actor) {
      calls.push(['submit', clone(value), clone(actor)]);
      return clone(record);
    },
  };
  const actions = createHearingDerivedDeadlineActions({
    read: () => state,
    update: (patch) => {
      updates.push(clone(patch));
      Object.assign(state, patch);
    },
    admitted: () => allowed,
    scoped,
    buildCommand: () => clone(ready.command),
    finish: async (value) => {
      completed.push(clone(value));
    },
    fail: (error, uncertain) => {
      failed.push({ error, uncertain });
    },
  });
  return {
    state,
    ready,
    record,
    scoped,
    actions,
    calls,
    updates,
    completed,
    failed,
    deny: () => {
      allowed = false;
    },
  };
}

test('preparation reviews one compound command and edits discard its approval', async () => {
  const s = setup();
  await s.actions.prepare();
  assert.deepEqual(s.calls, [['prepare', s.ready.command, principal()]]);
  assert.equal(s.state.mode, 'review');
  assert.deepEqual(s.state.prepared, s.ready);
  assert.equal(s.state.acknowledged, false);
  s.state.acknowledged = true;
  s.actions.edit();
  assert.equal(s.state.mode, 'draft');
  assert.equal(s.state.prepared, null);
  assert.equal(s.state.acknowledged, false);
  assert.equal(s.state.last, null);
});

test('fresh preparation Replay confirms the stored record without another write', async () => {
  const s = setup();
  s.scoped.prepare = async (command, actor) => {
    s.calls.push(['prepare', clone(command), clone(actor)]);
    return { state: 'replay', record: clone(s.record) };
  };
  await s.actions.prepare();
  assert.deepEqual(s.completed, [s.record]);
  assert.equal(s.calls.length, 1);
  assert.equal(s.state.acknowledged, false);
});

test('submission requires approval and freezes the exact attempt before awaiting transport', async () => {
  const s = setup('review'),
    pending = deferred();
  await s.actions.submit();
  assert.equal(s.calls.length, 0);
  s.state.acknowledged = true;
  s.scoped.submit = async (ready, actor) => {
    s.calls.push(['submit', clone(ready), clone(actor)]);
    assert.equal(s.state.mode, 'uncertain');
    assert.deepEqual(s.state.last, s.ready);
    return pending.promise;
  };
  const saving = s.actions.submit();
  s.state.prepared.command.result.change.values.summary = 'Changed after send';
  await s.actions.submit();
  assert.equal(s.calls.length, 1);
  assert.deepEqual(s.state.last, s.ready);
  pending.reject(new Error('Lost response'));
  await saving;
  assert.equal(s.state.mode, 'uncertain');
  assert.deepEqual(s.state.last, s.ready);
  assert.equal(s.state.retryAvailable, false);
  assert.equal(s.completed.length, 0);
  assert.equal(s.failed.length, 1);
  assert.equal(s.failed[0].uncertain, true);
});

test('explicit reconciliation permits historical Replay in a closed case after identity metadata changes', async () => {
  const s = setup('uncertain');
  s.state.closed = true;
  s.state.user = { ...principal(), email: 'current@example.test', role: 'litigator' };
  s.scoped.prepare = async (command, actor) => {
    s.calls.push(['prepare', clone(command), clone(actor)]);
    return { state: 'replay', record: clone(s.record) };
  };
  await s.actions.check();
  assert.deepEqual(s.calls, [['prepare', s.ready.command, s.state.user]]);
  assert.deepEqual(s.completed, [s.record]);
  assert.deepEqual(s.completed[0].origin.recorded_by, principal());
});

test('Ready during reconciliation is not success and only permits an explicit same-envelope retry', async () => {
  const s = setup('uncertain');
  await s.actions.retry();
  assert.equal(s.calls.length, 0);
  await s.actions.check();
  assert.equal(s.state.mode, 'uncertain');
  assert.equal(s.state.retryAvailable, true);
  assert.equal(s.completed.length, 0);
  assert.equal(s.calls.length, 1);
  assert.deepEqual(s.state.last, s.ready);
  await s.actions.retry();
  assert.deepEqual(s.calls, [
    ['prepare', s.ready.command, principal()],
    ['submit', s.ready, principal()],
  ]);
  assert.deepEqual(s.completed, [s.record]);
});

test('changed review blocks replacement attempts and retains the original command', async () => {
  const s = setup('uncertain'),
    newer = clone(s.ready);
  newer.review_digest = digest('2');
  s.scoped.prepare = async () => newer;
  await s.actions.check();
  assert.equal(s.state.mode, 'conflict');
  assert.equal(s.state.retryAvailable, false);
  assert.deepEqual(s.state.last, s.ready);
  const before = clone(s.state);
  await s.actions.retry();
  await s.actions.prepare();
  s.actions.edit();
  assert.deepEqual(s.state, before);
  assert.equal(s.completed.length, 0);
});

test('a mismatched Replay digest never confirms an uncertain attempt', async () => {
  const s = setup('uncertain'),
    changed = clone(s.record);
  changed.review_digest = changed.origin.review_digest = digest('2');
  s.scoped.prepare = async () => ({ state: 'replay', record: changed });
  await s.actions.check();
  assert.equal(s.state.mode, 'conflict');
  assert.equal(s.state.retryAvailable, false);
  assert.deepEqual(s.state.last, s.ready);
  assert.equal(s.completed.length, 0);
});

test('closed cases allow checking but prevent fresh writes and explicit retries', async () => {
  for (const mode of ['draft', 'review', 'uncertain']) {
    const s = setup(mode);
    s.state.closed = true;
    s.state.acknowledged = true;
    s.state.retryAvailable = true;
    await s.actions.prepare();
    await s.actions.submit();
    await s.actions.retry();
    assert.equal(s.calls.length, 0, mode);
    if (mode === 'uncertain') {
      await s.actions.check();
      assert.equal(s.calls.length, 1);
      await s.actions.retry();
      assert.equal(s.calls.length, 1);
    }
  }
});

test('a check failure keeps the attempt uncertain and cannot enable retry', async () => {
  for (const error of [new Error('Offline'), Object.assign(new Error('Denied'), { status: 403 })]) {
    const s = setup('uncertain');
    s.state.retryAvailable = true;
    s.scoped.prepare = async () => {
      throw error;
    };
    await s.actions.check();
    assert.equal(s.state.mode, 'uncertain');
    assert.equal(s.state.retryAvailable, false);
    assert.deepEqual(s.state.last, s.ready);
    assert.equal(s.failed[0].error, error);
    assert.equal(s.completed.length, 0);
  }
});

test('stale prepare success and failure cannot update a newer or disposed context', async () => {
  for (const invalidate of ['edit', 'dispose', 'deny']) {
    for (const response of ['success', 'failure']) {
      const s = setup(),
        pending = deferred();
      s.scoped.prepare = () => pending.promise;
      const operation = s.actions.prepare();
      if (invalidate === 'deny') s.deny();
      else s.actions[invalidate]();
      const count = s.updates.length;
      if (response === 'success') pending.resolve(clone(s.ready));
      else pending.reject(new Error('Late error'));
      await operation;
      assert.equal(s.updates.length, count, `${invalidate}/${response}`);
      assert.equal(s.failed.length, 0);
      assert.equal(s.completed.length, 0);
    }
  }
});

test('late submit or reconciliation responses never reopen a disposed editor', async () => {
  for (const method of ['submit', 'check', 'retry']) {
    const s = setup(method === 'submit' ? 'review' : 'uncertain'),
      pending = deferred();
    s.state.acknowledged = true;
    s.state.retryAvailable = true;
    if (method === 'check') s.scoped.prepare = () => pending.promise;
    else s.scoped.submit = () => pending.promise;
    const operation = s.actions[method]();
    s.actions.dispose();
    const count = s.updates.length;
    pending.resolve(method === 'check' ? { state: 'replay', record: s.record } : s.record);
    await operation;
    assert.equal(s.updates.length, count);
    assert.equal(s.completed.length, 0);
  }
});

test('pending fields, denied admission and busy state prevent duplicate operations', async () => {
  for (const field of ['pending', 'disabled', 'blocked', 'busy', 'denied']) {
    const s = setup();
    if (field === 'denied') s.deny();
    else s.state[field] = true;
    await s.actions.prepare();
    assert.equal(s.calls.length, 0, field);
  }
});
