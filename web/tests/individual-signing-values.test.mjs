import test from 'node:test';
import assert from 'node:assert/strict';
import {
  decodeSigningStatement,
  signingStatementDownload,
  signingContext,
} from '../src/lib/individual-signing-values.mjs';

const account = '11111111-1111-4111-8111-111111111111';
const caseId = '22222222-2222-4222-8222-222222222222';
const documentId = '33333333-3333-4333-8333-333333333333';
const uuid = (value) => Buffer.from(value.replaceAll('-', ''), 'hex');
const root = '44'.repeat(32),
  leaf = '55'.repeat(32),
  digest = '66'.repeat(32);
const bindingId = '77777777-7777-4777-8777-777777777777';
const deployment = '88888888-8888-4888-8888-888888888888';
const operation = '99999999-9999-4999-8999-999999999999';

function canonical(consent = false) {
  const bytes = Buffer.alloc(consent ? 258 : 142);
  bytes.write(consent ? 'DOCSIGN1' : 'DOCBIND1');
  bytes[8] = bytes[9] = 1;
  uuid(deployment).copy(bytes, 10);
  Buffer.from(root, 'hex').copy(bytes, 26);
  bytes.writeUInt32BE(3, 58);
  uuid(account).copy(bytes, 62);
  bytes.writeBigUInt64BE(9007199254740995n, 78);
  bytes.writeBigUInt64BE(9007199254740993n, 86);
  uuid(bindingId).copy(bytes, 94);
  Buffer.from(leaf, 'hex').copy(bytes, 110);
  if (consent) {
    uuid(operation).copy(bytes, 142);
    uuid(caseId).copy(bytes, 158);
    uuid(documentId).copy(bytes, 174);
    bytes.writeUInt32BE(7, 190);
    Buffer.from(digest, 'hex').copy(bytes, 194);
    Buffer.from('aa'.repeat(32), 'hex').copy(bytes, 226);
  }
  return bytes;
}

test('personal statements preserve exact accounts, references, purposes and full-width counters', () => {
  const binding = decodeSigningStatement(canonical().toString('base64'), 'binding');
  assert.deepEqual(binding, {
    accountId: account,
    deploymentId: deployment,
    bindingId,
    rootFingerprint: root,
    leafFingerprint: leaf,
    trustRevision: 3,
    accountRevision: '9007199254740995',
    authGeneration: '9007199254740993',
  });
  assert.deepEqual(decodeSigningStatement(canonical(true).toString('base64'), 'consent'), {
    ...binding,
    operationId: operation,
    caseId,
    documentId,
    version: 7,
    digest,
    pdfSignatureDigest: 'aa'.repeat(32),
  });
});

test('canonical statement downloads preserve public bytes and reject another purpose or version', async () => {
  for (const [kind, consent] of [
    ['binding', false],
    ['consent', true],
  ]) {
    const original = canonical(consent);
    const blob = signingStatementDownload(original.toString('base64'), kind);
    assert.equal(blob.type, 'application/octet-stream');
    assert.deepEqual(Buffer.from(await blob.arrayBuffer()), original);
    for (const index of [0, 8, 9]) {
      const changed = Buffer.from(original);
      changed[index] ^= 1;
      assert.throws(() => decodeSigningStatement(changed.toString('base64'), kind));
    }
    for (const changed of [original.subarray(1), Buffer.concat([original, Buffer.from([0])])])
      assert.throws(() => decodeSigningStatement(changed.toString('base64'), kind));
    assert.throws(() =>
      decodeSigningStatement(original.toString('base64'), consent ? 'binding' : 'consent'),
    );
    assert.throws(() => decodeSigningStatement(' ' + original.toString('base64'), kind));
  }
});

test('decoding rejects nil identity, zero trust and counter regression without accepting private material', () => {
  for (const [offset, length] of [
    [10, 16],
    [62, 16],
    [94, 16],
    [58, 4],
  ]) {
    const changed = canonical();
    changed.fill(0, offset, offset + length);
    assert.throws(() => decodeSigningStatement(changed.toString('base64'), 'binding'));
  }
  const futureGeneration = canonical();
  futureGeneration.writeBigUInt64BE(9007199254740996n, 86);
  assert.throws(() => decodeSigningStatement(futureGeneration.toString('base64'), 'binding'));
  const oversizedRevision = canonical();
  oversizedRevision.writeBigUInt64BE(9223372036854775808n, 78);
  assert.throws(() => decodeSigningStatement(oversizedRevision.toString('base64'), 'binding'));
  for (const [offset, length] of [
    [142, 16],
    [158, 16],
    [174, 16],
    [190, 4],
  ]) {
    const changed = canonical(true);
    changed.fill(0, offset, offset + length);
    assert.throws(() => decodeSigningStatement(changed.toString('base64'), 'consent'));
  }
  assert.throws(() =>
    decodeSigningStatement(
      Buffer.from('-----BEGIN PRIVATE KEY-----').toString('base64'),
      'binding',
    ),
  );
});

test('document intent context binds one account, case, version and content digest', () => {
  const context = signingContext(account, caseId, documentId, 7, digest);
  assert.deepEqual(context, { accountId: account, caseId, documentId, version: 7, digest });
  assert.equal(Object.isFrozen(context), true);
  for (const values of [
    ['', caseId, documentId, 7, digest],
    [account, '00000000-0000-0000-0000-000000000000', documentId, 7, digest],
    [account, caseId, documentId, 0, digest],
    [account, caseId, documentId, 7, 'wrong'],
  ])
    assert.throws(() => signingContext(...values));
});
