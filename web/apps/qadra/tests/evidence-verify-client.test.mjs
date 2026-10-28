import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createEvidenceVerifier } from '../src/lib/evidence-verify-client.mjs';

function harness() {
  const workers = [];
  const verifier = createEvidenceVerifier(() => {
    const worker = {
      terminated: false,
      posted: [],
      postMessage(message, transfer) {
        this.posted.push({ message, transfer });
      },
      terminate() {
        this.terminated = true;
      },
    };
    workers.push(worker);
    return worker;
  });
  const reply = (worker, data) =>
    worker.onmessage?.({ data: { op: worker.posted[0].message.op, ...data } });
  return { verifier, workers, reply };
}

test('one run transfers the ZIP, forwards stages and resolves with its context', async () => {
  const { verifier, workers, reply } = harness();
  const stages = [];
  const zip = new ArrayBuffer(8);
  const policy = { pinned: [], served: null, current_instant: 1 };
  const pending = verifier.run({ context: 'doc:2', zip, policy }, (stage) => stages.push(stage));
  const [worker] = workers;
  const { message, transfer } = worker.posted[0];
  assert.equal(message.zip, zip);
  assert.deepEqual(transfer, [zip]);
  assert.equal(message.policy, policy);
  assert.ok(message.op !== undefined);
  reply(worker, { stage: 'module' });
  reply(worker, { stage: 'validate' });
  reply(worker, { stage: 'unexpected' });
  reply(worker, { ok: true, report: { overall: 'checked' } });
  assert.deepEqual(await pending, { context: 'doc:2', report: { overall: 'checked' } });
  assert.deepEqual(stages, ['module', 'validate']);
  assert.equal(worker.terminated, true);
});

test('a reply carrying another operation identifier is dropped', async () => {
  const { verifier, workers } = harness();
  const pending = verifier.run({ context: 'c', zip: new ArrayBuffer(1), policy: {} });
  const [worker] = workers;
  const op = worker.posted[0].message.op;
  worker.onmessage({ data: { op: `${op}-other`, ok: true, report: 'stale' } });
  worker.onmessage({ data: { ok: true, report: 'no-op' } });
  assert.equal(worker.terminated, false);
  worker.onmessage({ data: { op, ok: true, report: 'current' } });
  assert.equal((await pending).report, 'current');
});

test('cancel rejects with AbortError and ignores late stages and replies', async () => {
  const { verifier, workers } = harness();
  const stages = [];
  const pending = verifier.run({ context: 'c', zip: new ArrayBuffer(1), policy: {} }, (s) =>
    stages.push(s),
  );
  const [worker] = workers;
  const late = worker.onmessage;
  const rejected = assert.rejects(pending, { name: 'AbortError' });
  verifier.cancel();
  await rejected;
  assert.equal(worker.terminated, true);
  late({ data: { op: worker.posted[0].message.op, stage: 'validate' } });
  late({ data: { op: worker.posted[0].message.op, ok: true, report: 'late' } });
  assert.deepEqual(stages, []);
  verifier.cancel();
});

test('a newer run cancels the previous one and its late result is ignored', async () => {
  const { verifier, workers, reply } = harness();
  const first = verifier.run({ context: 'doc:1', zip: new ArrayBuffer(1), policy: {} });
  const firstRejected = assert.rejects(first, { name: 'AbortError' });
  const late = workers[0].onmessage;
  const second = verifier.run({ context: 'doc:2', zip: new ArrayBuffer(1), policy: {} });
  await firstRejected;
  assert.notEqual(workers[0].posted[0].message.op, workers[1].posted[0].message.op);
  late({ data: { op: workers[0].posted[0].message.op, ok: true, report: 'old' } });
  reply(workers[1], { ok: true, report: 'new' });
  assert.deepEqual(await second, { context: 'doc:2', report: 'new' });
  assert.ok(workers.every((worker) => worker.terminated));
});

test('worker error codes map to bounded messages with a stable code', async () => {
  const cases = [
    ['module_unavailable', 'module_unavailable'],
    ['policy_invalid', 'policy_invalid'],
    ['engine_failed', 'engine_failed'],
    ['secret-detail', 'engine_failed'],
  ];
  for (const [sent, code] of cases) {
    const { verifier, workers, reply } = harness();
    const pending = verifier.run({ context: 'c', zip: new ArrayBuffer(1), policy: {} });
    reply(workers[0], { ok: false, error: sent });
    await assert.rejects(pending, (error) => {
      assert.equal(error.code, code);
      assert.ok(!error.message.includes(sent));
      assert.match(error.message, /^[^\x00-\x1f\u2013\u2014]+$/);
      return true;
    });
    assert.equal(workers[0].terminated, true);
  }
});

test('runtime failures are reported without leaving a worker running', async () => {
  const unavailable = createEvidenceVerifier(() => {
    throw new Error('no workers');
  });
  await assert.rejects(unavailable.run({ context: 'c', zip: new ArrayBuffer(1), policy: {} }), {
    code: 'module_unavailable',
  });
  for (const [handler, code] of [
    ['onerror', 'module_unavailable'],
    ['onmessageerror', 'engine_failed'],
  ]) {
    const { verifier, workers } = harness();
    const pending = verifier.run({ context: 'c', zip: new ArrayBuffer(1), policy: {} });
    workers[0][handler]({});
    await assert.rejects(pending, { code });
    assert.equal(workers[0].terminated, true);
  }
  const refusing = createEvidenceVerifier(() => ({
    postMessage() {
      throw new DOMException('detached', 'DataCloneError');
    },
    terminate() {},
  }));
  await assert.rejects(refusing.run({ context: 'c', zip: new ArrayBuffer(1), policy: {} }), {
    code: 'engine_failed',
  });
  await assert.rejects(
    harness().verifier.run({ context: 'c', zip: new Uint8Array(1), policy: {} }),
    TypeError,
  );
});

test('the worker code never sends data over the network', () => {
  for (const file of ['evidence-verify-worker.mjs', 'evidence-verify-run.mjs']) {
    const source = readFileSync(new URL(`../src/lib/${file}`, import.meta.url), 'utf8');
    for (const forbidden of ['fetch(', 'XMLHttpRequest', 'sendBeacon', 'WebSocket', 'EventSource'])
      assert.ok(!source.includes(forbidden), `${file} uses ${forbidden}`);
  }
});

test('the client and worker sources are ASCII without typographic dashes', () => {
  for (const file of [
    'evidence-verify-client.mjs',
    'evidence-verify-worker.mjs',
    'evidence-verify-run.mjs',
  ]) {
    const source = readFileSync(new URL(`../src/lib/${file}`, import.meta.url), 'utf8');
    assert.match(source, /^[\x09\x0a\x0d\x20-\x7e]*$/, file);
  }
});
