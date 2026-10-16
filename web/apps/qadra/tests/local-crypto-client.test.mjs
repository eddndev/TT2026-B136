import test from 'node:test';
import assert from 'node:assert/strict';
import { createLocalCrypto } from '../src/lib/local-crypto-client.mjs';

function harness() {
  const workers = [];
  const client = createLocalCrypto(() => {
    const worker = {
      terminated: false,
      postMessage(value) {
        this.request = value;
      },
      terminate() {
        this.terminated = true;
      },
    };
    workers.push(worker);
    return worker;
  });
  return { client, workers };
}
test('one operation returns only its result and disposes the worker', async () => {
  const { client, workers } = harness();
  const pending = client.run('sign', { password: 'local-only', bytes: new Uint8Array([1]) });
  workers[0].onmessage({ data: { ok: true, result: 'public-signature' } });
  assert.equal(await pending, 'public-signature');
  assert.equal(workers[0].terminated, true);
});
test('cancellation rejects and ignores a late reply from a previous context', async () => {
  const { client, workers } = harness();
  const first = client.run('sign', {});
  const lateReply = workers[0].onmessage;
  const rejected = assert.rejects(first, { name: 'AbortError' });
  client.cancel();
  await rejected;
  const second = client.run('generate', {});
  lateReply({ data: { ok: true, result: 'old-context' } });
  workers[1].onmessage({ data: { ok: true, result: 'new-context' } });
  assert.equal(await second, 'new-context');
  assert.ok(workers.every((worker) => worker.terminated));
});
test('worker failure is bounded and does not expose arbitrary error details', async () => {
  const { client, workers } = harness();
  const pending = client.run('sign', {});
  workers[0].onmessage({ data: { ok: false, error: 'secret-key-value' } });
  await assert.rejects(pending, (error) => !error.message.includes('secret-key-value'));
  assert.equal(workers[0].terminated, true);
});
test('unsupported runtime fails without starting an operation', async () => {
  const client = createLocalCrypto(() => {
    throw new Error('unavailable');
  });
  await assert.rejects(client.run('sign', {}), /navegador|local/);
});
