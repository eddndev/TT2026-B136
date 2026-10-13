import test from 'node:test';
import assert from 'node:assert/strict';
import {
  readOwnerLoginReceipt,
  ownerLoginChallenge,
  ownerLoginStatementDownload,
  ownerLoginDeadline,
} from '../src/lib/owner-login-values.mjs';
import {
  publicReceipt,
  maximumReceipt,
  selection,
  statement,
  challenge,
  token,
  withdrawn,
  receiptByteLimit,
} from './fixtures/owner-login.mjs';

const file = (value) => new File([JSON.stringify(value, null, 2)], 'public-receipt.json');

test('receipt intake projects only public selection and rejects terminal or private containers', async () => {
  const selected = await readOwnerLoginReceipt(file(publicReceipt()));
  assert.deepEqual(selected, selection);
  assert.ok(Object.isFrozen(selected));
  for (const value of [
    null,
    [],
    {},
    withdrawn(publicReceipt()),
    { ...publicReceipt(), private_key: 'private-intake-sentinel' },
    { ...publicReceipt(), policy: 'different-policy' },
    { ...publicReceipt(), owner_id: '00000000-0000-0000-0000-000000000000' },
  ])
    await assert.rejects(readOwnerLoginReceipt(file(value)));
  for (const bytes of [
    '-----BEGIN PRIVATE KEY-----\nprivate-intake-sentinel\n-----END PRIVATE KEY-----',
    '-----BEGIN ENCRYPTED PRIVATE KEY-----\nprivate-intake-sentinel\n-----END ENCRYPTED PRIVATE KEY-----',
    '{"private_key":"private-intake-sentinel"}',
    Buffer.from([0xff, 0xfe, 0x7b, 0x7d]),
    JSON.stringify(publicReceipt()) + '{}',
  ])
    await assert.rejects(readOwnerLoginReceipt(new File([bytes], 'selected.json')), (error) => {
      assert.ok(!error.message.includes('private-intake-sentinel'));
      return true;
    });
});

test('receipt byte budget covers existing public maxima and rejects before or after file read', async () => {
  const maximum = maximumReceipt();
  const bytes = Buffer.from(JSON.stringify(maximum, null, 2));
  assert.ok(bytes.length <= receiptByteLimit);
  assert.ok(receiptByteLimit - bytes.length < 4096);
  const selected = await readOwnerLoginReceipt(new File([bytes], 'maximum-export.json'));
  assert.equal(selected.ownerId, selection.ownerId);
  assert.equal(selected.subject, maximum.registration.certificate.summary.subject);
  const padded = Buffer.concat([bytes, Buffer.alloc(receiptByteLimit - bytes.length, 32)]);
  assert.equal(
    (await readOwnerLoginReceipt(new File([padded], 'boundary.json'))).bindingId,
    selection.bindingId,
  );
  let reads = 0;
  for (const size of [0, -1, 1.5, receiptByteLimit + 1]) {
    await assert.rejects(
      readOwnerLoginReceipt({
        size,
        arrayBuffer() {
          reads++;
        },
      }),
    );
  }
  assert.equal(reads, 0);
  const data = new Uint8Array(bytes).buffer;
  await assert.rejects(
    readOwnerLoginReceipt({ size: bytes.length - 1, arrayBuffer: async () => data }),
  );
  await assert.rejects(
    readOwnerLoginReceipt({
      size: 10,
      arrayBuffer: async () => new ArrayBuffer(receiptByteLimit + 1),
    }),
  );
  await assert.rejects(
    readOwnerLoginReceipt({
      size: 10,
      arrayBuffer: async () => {
        throw new Error('private-file-read-sentinel');
      },
    }),
    (error) => {
      assert.ok(!error.message.includes('private-file-read-sentinel'));
      return true;
    },
  );
});

test('login downloads preserve the canonical182 vector and admit current facts after historical capture', async () => {
  const original = challenge();
  assert.deepEqual(ownerLoginChallenge(original, selection), original);
  const blob = ownerLoginStatementDownload(original, selection);
  assert.equal(blob.type, 'application/octet-stream');
  assert.deepEqual(Buffer.from(await blob.arrayBuffer()), statement);
  assert.equal(blob.size, 182);
  const current = Buffer.from(statement);
  current.fill(0x73, 10, 26);
  current.fill(0x74, 26, 58);
  current.writeUInt32BE(0xffffffff, 58);
  current.writeBigUInt64BE(9223372036854775807n, 78);
  const updated = challenge(current);
  const selected = await readOwnerLoginReceipt(file(publicReceipt()));
  assert.deepEqual(ownerLoginChallenge(updated, selected), updated);
  assert.deepEqual(
    Buffer.from(await ownerLoginStatementDownload(updated, selected).arrayBuffer()),
    current,
  );
  assert.equal(
    Buffer.from(updated.statement_base64, 'base64').readBigUInt64BE(78),
    9223372036854775807n,
  );
});

test('challenge shape identity purpose exact integers and local monotonic lifetime fail closed', () => {
  const invalid = [
    null,
    [],
    {},
    { ...challenge(), access_token: 'unexpected-session' },
    { ...challenge(), expires_in_seconds: 0 },
    { ...challenge(), expires_in_seconds: 301 },
    { ...challenge(), expires_in_seconds: 1.5 },
    { ...challenge(), expires_in_seconds: '299' },
    ...['', 'a'.repeat(42), `${token}=`, Buffer.alloc(32, 0xfb).toString('base64')].map(
      (value) => ({ ...challenge(), challenge_token: value }),
    ),
    ...[Buffer.alloc(150), Buffer.alloc(181), Buffer.alloc(183)].map((bytes) => challenge(bytes)),
    { ...challenge(), statement_base64: ` ${statement.toString('base64')}` },
    { ...challenge(), statement_base64: statement.toString('base64').slice(0, -1) },
  ];
  for (const mutate of [
    (bytes) => bytes.write('OWNCERT1'),
    (bytes) => (bytes[8] = 2),
    (bytes) => (bytes[9] = 2),
    (bytes) => bytes.fill(0, 10, 26),
    (bytes) => bytes.writeUInt32BE(0, 58),
    (bytes) => bytes.fill(0x22, 62, 78),
    (bytes) => bytes.writeBigUInt64BE(9223372036854775808n, 78),
    (bytes) => bytes.fill(0x33, 86, 102),
    (bytes) => bytes.fill(0x66, 102, 134),
    (bytes) => bytes.writeBigInt64BE(-1n, 166),
    (bytes) => bytes.writeBigInt64BE(1700000000n, 174),
    (bytes) => bytes.writeBigInt64BE(1700000301n, 174),
    (bytes) => bytes.writeBigInt64BE(1700000001n, 174),
  ]) {
    const bytes = Buffer.from(statement);
    mutate(bytes);
    invalid.push(challenge(bytes));
  }
  for (const value of invalid) assert.throws(() => ownerLoginChallenge(value, selection));
  assert.equal(
    ownerLoginDeadline(challenge(), { startedAt: 1000.5, receivedAt: 1800.5 }),
    300000.5,
  );
  for (const timing of [
    { startedAt: 1000, receivedAt: 300000 },
    { startedAt: 1000, receivedAt: 999 },
    { startedAt: NaN, receivedAt: 1000 },
    { startedAt: 0, receivedAt: Infinity },
    { startedAt: -1, receivedAt: 1000 },
  ])
    assert.throws(() => ownerLoginDeadline(challenge(), timing));
});
