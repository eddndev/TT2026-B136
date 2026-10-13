import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  readPublicCertificate,
  readDetachedSignature,
  declarationDownloads,
} from '../src/lib/participant-credential-files.mjs';
import { credentialSelection } from '../src/lib/participant-credential-selection.mjs';

const pem = readFileSync(new URL('./fixtures/participant-public-certificate.pem', import.meta.url));
const der = Buffer.from(pem.toString().replace(/-----[^-]+-----|\s/g, ''), 'base64');
const certificate = () => new File([pem], 'public.pem');
const signature = () => new File([new Uint8Array(384).fill(7)], 'statement.sig');
const statement = () => {
  const bytes = new Uint8Array(218);
  bytes.set(new TextEncoder().encode('PCRED1'));
  for (let n = 8; n < bytes.length; n++) bytes[n] = n;
  return bytes;
};
const receipt = new TextEncoder().encode('{ "description": "Exact receipt" }\n');
const bytes = async (blob) => new Uint8Array(await blob.arrayBuffer());
async function prepared(selection) {
  await selection.selectCertificate(certificate());
  const ticket = selection.beginPreparation();
  assert.equal(selection.acceptPreparation(ticket, statement(), receipt), true);
}
function delayedFile(file) {
  let release;
  const pending = new Promise((resolve) => (release = resolve));
  return {
    file: { name: file.name, size: file.size, arrayBuffer: () => pending },
    release: async () => release(await file.arrayBuffer()),
  };
}

test('public certificate preflight accepts exact PEM and DER without certifying trust', async () => {
  assert.deepEqual(await bytes(await readPublicCertificate(certificate())), new Uint8Array(pem));
  assert.deepEqual(
    await bytes(await readPublicCertificate(new File([der], 'certificate.der'))),
    new Uint8Array(der),
  );
});

test('certificate limit is enforced before reading and after receiving bytes', async () => {
  let reads = 0;
  await assert.rejects(
    readPublicCertificate({ size: 16385, arrayBuffer: () => reads++ }),
    /16 KiB/,
  );
  assert.equal(reads, 0);
  await assert.rejects(
    readPublicCertificate({ size: 20, arrayBuffer: async () => new ArrayBuffer(16385) }),
    /16 KiB/,
  );
  const padded = new Uint8Array(16384).fill(32);
  padded.set(pem);
  assert.equal((await readPublicCertificate(new File([padded], 'limit.pem'))).size, 16384);
});

test('certificate preflight rejects private envelopes, extra material and non-certificate DER', async () => {
  for (const material of [
    '',
    pem.toString().replaceAll('CERTIFICATE', 'PRIVATE KEY'),
    pem.toString().replaceAll('CERTIFICATE', 'CERTIFICATE REQUEST'),
    Buffer.concat([pem, pem]),
    Buffer.concat([pem, Buffer.from('trailing')]),
    Buffer.concat([der, Buffer.from([0])]),
    der.subarray(0, der.length - 1),
    Uint8Array.of(0x30, 3, 2, 1, 0),
    Uint8Array.of(0x30, 0x80, 0, 0),
  ])
    await assert.rejects(readPublicCertificate(new File([material], 'public.cer')), /certificado/i);
});

test('detached signature is binary with exactly 384 bytes, bounded before reading', async () => {
  assert.equal((await readDetachedSignature(signature())).size, 384);
  for (const size of [0, 383, 385]) {
    let read = false;
    await assert.rejects(
      readDetachedSignature({ size, arrayBuffer: () => (read = true) }),
      /384 bytes/,
    );
    assert.equal(read, false);
  }
  await assert.rejects(
    readDetachedSignature({ size: 384, arrayBuffer: async () => new ArrayBuffer(383) }),
    /384 bytes/,
  );
});

test('declaration and receipt downloads preserve bytes without JSON reserialization', async () => {
  const canonical = statement();
  const expected = canonical.slice();
  const downloads = declarationDownloads(canonical, receipt);
  canonical.fill(0);
  assert.deepEqual(await bytes(downloads.statement), expected);
  assert.deepEqual(await bytes(downloads.receipt), receipt);
  assert.equal(downloads.statement.type, 'application/octet-stream');
  assert.equal(downloads.receipt.type, 'application/json');
  for (const invalid of [new Uint8Array(217), new Uint8Array(219), new Uint8Array(218)])
    assert.throws(() => declarationDownloads(invalid, receipt), /declaraci/);
  for (const index of [6, 7]) {
    const invalid = statement();
    invalid[index] = 1;
    assert.throws(() => declarationDownloads(invalid, receipt), /declaraci/);
  }
});

test('a new base or values invalidates prepared bytes and signature while retaining certificate', async () => {
  const selection = credentialSelection();
  selection.updateContext('case/session', 'base and values 1');
  await prepared(selection);
  await selection.selectSignature(signature());
  assert.equal(selection.snapshot().ready, true);
  selection.updateContext('case/session', 'base and values 2');
  assert.equal(selection.snapshot().certificate.name, 'public.pem');
  assert.equal(selection.snapshot().prepared, null);
  assert.equal(selection.snapshot().signature, null);
  assert.equal(selection.snapshot().ready, false);
});

test('certificate changes invalidate the old preparation and signature even if selection fails', async () => {
  const selection = credentialSelection();
  selection.updateContext('scope', 1);
  await prepared(selection);
  await selection.selectSignature(signature());
  await selection.selectCertificate(new File(['PRIVATE KEY'], 'private.pem'));
  assert.equal(selection.snapshot().certificate, null);
  assert.equal(selection.snapshot().prepared, null);
  assert.equal(selection.snapshot().signature, null);
  assert.match(selection.snapshot().error, /certificado/i);
});

test('late preparation cannot bind to changed values or a different certificate', async () => {
  const selection = credentialSelection();
  selection.updateContext('scope', 1);
  await selection.selectCertificate(certificate());
  const first = selection.beginPreparation();
  selection.updateContext('scope', 2);
  assert.equal(selection.acceptPreparation(first, statement(), receipt), false);
  const second = selection.beginPreparation();
  await selection.selectCertificate(certificate());
  assert.equal(selection.acceptPreparation(second, statement(), receipt), false);
  assert.equal(selection.snapshot().prepared, null);
  assert.equal(selection.snapshot().busy, false);
});

test('scope change clears materials and ignores a delayed certificate read', async () => {
  const selection = credentialSelection();
  selection.updateContext('first case/session', 1);
  const delayed = delayedFile(certificate());
  const pending = selection.selectCertificate(delayed.file);
  selection.updateContext('second case/session', 1);
  await delayed.release();
  await pending;
  assert.equal(selection.snapshot().certificate, null);
  assert.equal(selection.snapshot().busy, false);
});

test('late signature read and failed preparation cannot restore invalidated materials', async () => {
  const selection = credentialSelection();
  selection.updateContext('scope', 1);
  await prepared(selection);
  const delayed = delayedFile(signature());
  const pending = selection.selectSignature(delayed.file);
  selection.updateContext('scope', 2);
  await delayed.release();
  await pending;
  assert.equal(selection.snapshot().signature, null);
  const ticket = selection.beginPreparation();
  selection.updateContext('another scope', 1);
  assert.equal(selection.failPreparation(ticket, 'Old failure'), false);
  assert.equal(selection.snapshot().error, '');
  assert.equal(selection.snapshot().certificate, null);
});

test('dispose clears held materials and subscriptions do not receive later async results', async () => {
  const selection = credentialSelection();
  selection.updateContext('scope', 1);
  await prepared(selection);
  const snapshots = [];
  const unsubscribe = selection.subscribe((value) => snapshots.push(value));
  const delayed = delayedFile(signature());
  const pending = selection.selectSignature(delayed.file);
  selection.dispose();
  const count = snapshots.length;
  await delayed.release();
  await pending;
  assert.equal(snapshots.length, count);
  assert.equal(selection.snapshot().certificate, null);
  assert.equal(selection.snapshot().prepared, null);
  assert.equal(selection.snapshot().signature, null);
  unsubscribe();
});

test('preparation completion can settle its ticket only once', async () => {
  const selection = credentialSelection();
  selection.updateContext('scope', 1);
  await selection.selectCertificate(certificate());
  const ticket = selection.beginPreparation();
  assert.equal(selection.acceptPreparation(ticket, statement(), receipt), true);
  const replacement = statement();
  replacement[8]++;
  assert.equal(selection.acceptPreparation(ticket, replacement, receipt), false);
  assert.equal(selection.failPreparation(ticket, 'A late failure'), false);
  assert.deepEqual(await bytes(selection.snapshot().prepared.statement), statement());
  assert.equal(selection.snapshot().error, '');
});

test('signature selection during preparation does not invalidate or strand the pending request', async () => {
  const selection = credentialSelection();
  selection.updateContext('scope', 1);
  await selection.selectCertificate(certificate());
  const ticket = selection.beginPreparation();
  assert.equal(await selection.selectSignature(signature()), false);
  assert.equal(selection.snapshot().busy, true);
  assert.equal(selection.acceptPreparation(ticket, statement(), receipt), true);
  assert.equal(selection.snapshot().busy, false);
});

test('an unchanged context retains ready materials and a new scope clears all of them', async () => {
  const selection = credentialSelection();
  selection.updateContext('scope', 1);
  await prepared(selection);
  await selection.selectSignature(signature());
  selection.updateContext('scope', 1);
  assert.equal(selection.snapshot().ready, true);
  selection.updateContext('new scope', 1);
  assert.deepEqual(
    [
      selection.snapshot().certificate,
      selection.snapshot().prepared,
      selection.snapshot().signature,
    ],
    [null, null, null],
  );
  assert.equal(selection.snapshot().ready, false);
});

test('no files are read before a scope is set or after the scope is cleared', async () => {
  const selection = credentialSelection();
  let reads = 0;
  const file = { name: 'public.pem', size: pem.length, arrayBuffer: () => reads++ };
  assert.equal(await selection.selectCertificate(file), false);
  assert.equal(reads, 0);
  selection.updateContext('scope', 1);
  await prepared(selection);
  selection.updateContext(null, null);
  assert.equal(await selection.selectCertificate(file), false);
  assert.equal(reads, 0);
  assert.equal(selection.snapshot().certificate, null);
});
