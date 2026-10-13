import test from 'node:test';
import assert from 'node:assert/strict';
import { webcrypto } from 'node:crypto';
import { invitationOperations } from '../src/lib/invitation-operation.mjs';

test('reopening the same link restores only operation identifiers and phase', async () => {
  const values = new Map();
  const storage = {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
    removeItem: (key) => values.delete(key),
  };
  const token = Buffer.alloc(32, 9).toString('base64url');
  const first = await invitationOperations(token, storage, webcrypto);
  first.mark('finishing');
  const reopened = await invitationOperations(token, storage, webcrypto);
  assert.equal(reopened.prepare, first.prepare);
  assert.equal(reopened.finish, first.finish);
  assert.equal(reopened.phase, 'finishing');
  assert.ok(!JSON.stringify([...values]).includes(token));
  assert.deepEqual(Object.keys(JSON.parse([...values.values()][0])).sort(), [
    'finish',
    'phase',
    'prepare',
  ]);
  reopened.clear();
  assert.equal(values.size, 0);
});
test('unavailable storage remains explicit and does not block a fresh enrollment', async () => {
  const storage = {
    getItem() {
      throw new Error('blocked');
    },
    setItem() {
      throw new Error('blocked');
    },
  };
  const result = await invitationOperations('token', storage, webcrypto);
  assert.equal(result.durable, false);
  assert.ok(result.prepare);
  assert.ok(result.finish);
  result.mark('prepared');
  assert.equal(result.phase, 'prepared');
});
