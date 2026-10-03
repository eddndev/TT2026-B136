import test from 'node:test';
import assert from 'node:assert/strict';
import {
  readOwnerPublicCertificate,
  ownerStatementDownload,
} from '../src/lib/owner-certificate-files.mjs';
import { publicPem, publicDer, canonical } from './fixtures/owner-certificates.mjs';

test('Owner certificate intake admits only one bounded public PEM container and preserves its bytes', async () => {
  const result = await readOwnerPublicCertificate(new File([publicPem], 'public.pem'));
  assert.ok(result instanceof Blob);
  assert.deepEqual(Buffer.from(await result.arrayBuffer()), publicPem);
  for (const material of [
    publicDer,
    Buffer.concat([publicPem, publicPem]),
    Buffer.concat([publicPem, Buffer.from('trailing')]),
    publicPem.toString().replaceAll('CERTIFICATE', 'PRIVATE KEY'),
    publicPem.toString().replaceAll('CERTIFICATE', 'ENCRYPTED PRIVATE KEY'),
    publicPem.toString().replaceAll('CERTIFICATE', 'CERTIFICATE REQUEST'),
    '-----BEGIN CERTIFICATE-----\n_w==\n-----END CERTIFICATE-----',
    '-----BEGIN CERTIFICATE-----\nZh==\n-----END CERTIFICATE-----',
  ])
    await assert.rejects(readOwnerPublicCertificate(new File([material], 'selected.pem')));
});

test('Owner public file limits reject before read and recheck actual bytes without retaining the File', async () => {
  let reads = 0;
  await assert.rejects(readOwnerPublicCertificate({ size: 16385, arrayBuffer: () => reads++ }));
  assert.equal(reads, 0);
  await assert.rejects(
    readOwnerPublicCertificate({ size: 12, arrayBuffer: async () => new ArrayBuffer(16385) }),
  );
  const padded = Buffer.alloc(16384, 32);
  publicPem.copy(padded);
  const file = new File([padded], 'maximum-public.pem');
  const result = await readOwnerPublicCertificate(file);
  assert.equal(result.size, 16384);
  assert.notEqual(result, file);
  assert.equal(result instanceof File, false);
});

test('download retains the exact 150-byte registration vector and rejects other purposes and encodings', async () => {
  const blob = ownerStatementDownload(canonical.toString('base64'));
  assert.equal(blob.type, 'application/octet-stream');
  assert.equal(blob.size, 150);
  assert.deepEqual(Buffer.from(await blob.arrayBuffer()), canonical);
  const withdrawn = Buffer.from(canonical);
  withdrawn[8] = 2;
  const declaration = Buffer.alloc(150);
  declaration.write('PCRED1');
  for (const input of [
    Buffer.alloc(149).toString('base64'),
    Buffer.alloc(151).toString('base64'),
    declaration.toString('base64'),
    withdrawn.toString('base64'),
    ` ${canonical.toString('base64')}`,
    `${canonical.toString('base64')}=`,
  ])
    assert.throws(() => ownerStatementDownload(input));
});
