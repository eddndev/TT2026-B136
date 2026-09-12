import test from 'node:test';
import assert from 'node:assert/strict';
import { createApi } from '../src/lib/api.mjs';
import {
  can,
  safeFilename,
  validateUpload,
  validId,
  upsertDocument,
} from '../src/lib/documents.mjs';

test('login has JSON credentials and no bearer; MFA establishes memory-only session', async () => {
  const requests = [];
  const api = createApi(async (url, options) => {
    requests.push({ url, ...options });
    return Response.json(url.endsWith('/totp') ? { access_token: 'opaque' } : {});
  });
  await api.login('a@example.com', 'secret');
  await api.mfa('challenge', '123456', 'totp');
  await api.me();
  assert.equal(requests[0].headers.Authorization, undefined);
  assert.deepEqual(JSON.parse(requests[0].body), { email: 'a@example.com', password: 'secret' });
  assert.equal(requests[2].headers.Authorization, 'Bearer opaque');
  assert.equal(requests[2].cache, 'no-store');
});

test('upload sends bytes and an ASCII name header without multipart encoding', async () => {
  const file = new Blob(['evidence']);
  const api = createApi(async (url, options) => {
    assert.equal(url, '/api/v1/documents');
    assert.equal(options.body, file);
    assert.equal(options.headers['X-Document-Name'], 'evidence.pdf');
    assert.equal(options.headers['Content-Type'], 'application/octet-stream');
    return Response.json({ id: 'document' }, { status: 201 });
  });
  await api.upload(file, 'evidence.pdf');
});

test('protected 401 clears session and notifies application', async () => {
  let expired = 0;
  const api = createApi(
    async () => Response.json({ error: { code: 'invalid_session' } }, { status: 401 }),
    () => expired++,
  );
  await assert.rejects(api.me(), /sesion/i);
  assert.equal(expired, 1);
});

test('logout accepts an empty 204 and removes the bearer', async () => {
  const requests = [];
  const api = createApi(async (url, options) => {
    requests.push(options);
    if (url.endsWith('/totp')) return Response.json({ access_token: 'opaque' });
    return new Response(null, { status: 204 });
  });
  await api.mfa('challenge', '123456', 'totp');
  await api.logout();
  await api.me();
  assert.equal(requests[1].headers.Authorization, 'Bearer opaque');
  assert.equal(requests[2].headers.Authorization, undefined);
});

test('API translates permission errors and handles non-JSON gateway failures', async () => {
  const denied = createApi(async () =>
    Response.json({ error: { code: 'permission_denied' } }, { status: 403 }),
  );
  await assert.rejects(denied.audit(), /permiso/i);
  const offline = createApi(async () => new Response('Bad gateway', { status: 502 }));
  await assert.rejects(offline.audit(), /servidor/i);
});

test('API preserves stable error codes and explains pending documents and duplicate users', async () => {
  const pending = createApi(async () =>
    Response.json({ error: { code: 'document_not_sealed' } }, { status: 409 }),
  );
  await assert.rejects(pending.verify('document'), (error) => {
    assert.equal(error.code, 'document_not_sealed');
    assert.equal(error.status, 409);
    assert.match(error.message, /verificar|verificacion/);
    assert.match(error.message, /evidencia/);
    return true;
  });
  const duplicate = createApi(async () =>
    Response.json({ error: { code: 'user_already_exists' } }, { status: 409 }),
  );
  await assert.rejects(duplicate.createUser('a@example.com', 'password', 'client'), (error) => {
    assert.equal(error.code, 'user_already_exists');
    assert.match(error.message, /cuenta.*correo/);
    return true;
  });
});

test('evidence returns a ZIP blob and the digest header', async () => {
  const api = createApi(
    async () => new Response('zip', { headers: { 'X-Document-Digest': 'abc' } }),
  );
  const result = await api.evidence('123');
  assert.equal(await result.blob.text(), 'zip');
  assert.equal(result.digest, 'abc');
});

test('roles mirror backend permissions, including denied unknown roles', () => {
  assert.equal(can('owner', 'users'), true);
  assert.equal(can('litigator', 'seal'), true);
  assert.equal(can('paralegal', 'seal'), false);
  assert.equal(can('paralegal', 'documents'), true);
  assert.equal(can('client', 'documents'), false);
  assert.equal(can('unknown', 'documents'), false);
});

test('reject oversized uploads and names incompatible with evidence archive entries', () => {
  assert.equal(validateUpload({ size: 16 * 1024 * 1024 }, 'file.pdf'), '');
  assert.match(validateUpload({ size: 16 * 1024 * 1024 + 1 }, 'file.pdf'), /16 MiB/);
  for (const name of [
    '',
    '../file',
    'a\\b',
    'a\nheader',
    'caf\u00e9.pdf',
    'file name.pdf',
    'file(1).pdf',
    '.hidden',
    '-flag.txt',
    '_document.pdf',
    'a'.repeat(125),
  ]) {
    assert.ok(validateUpload({ size: 1 }, name));
  }
  assert.equal(validateUpload({ size: 1 }, 'a'.repeat(124)), '');
  for (const name of [
    'INSTRUCCIONES.md',
    'certificado.pem',
    'ca.pem',
    'crl.pem',
    'tsa-chain.pem',
  ]) {
    assert.match(validateUpload({ size: 1 }, name), /reservado/);
  }
});

test('filename suggestions preserve useful names and normalize accents and paths', () => {
  assert.equal(safeFilename('contrato_01.pdf'), 'contrato_01.pdf');
  assert.equal(safeFilename('Resoluci\u00f3n final (2).pdf'), 'Resolucion-final-2.pdf');
  assert.equal(safeFilename('C:\\fakepath\\acta.pdf'), 'acta.pdf');
  assert.equal(safeFilename('../informe final.txt'), 'informe-final.txt');
  assert.equal(safeFilename('.hidden'), 'hidden');
  assert.equal(safeFilename(''), 'documento');
  assert.equal(safeFilename('\u6587\u6863'), 'documento');
  assert.equal(safeFilename('\u6587\u6863.pdf'), 'documento.pdf');
});

test('filename suggestions leave room for evidence suffixes and avoid reserved archive names', () => {
  const long = safeFilename(`${'a'.repeat(160)}.pdf`);
  assert.equal(long.length, 124);
  assert.ok(long.endsWith('.pdf'));
  for (const input of [
    'INSTRUCCIONES.md',
    'certificado.pem',
    'ca.pem',
    'crl.pem',
    'tsa-chain.pem',
    'CA.PEM',
    'a'.repeat(180),
    '--.txt',
  ]) {
    const name = safeFilename(input);
    assert.equal(validateUpload({ size: 1 }, name), '', name);
    assert.notEqual(name.toLowerCase(), input.toLowerCase());
  }
});

test('document references require UUIDs and repeated upload responses replace metadata', () => {
  assert.equal(validId('78ac67b1-ab36-49ea-9b08-f951f341f081'), true);
  assert.equal(validId('../auth/me'), false);
  assert.deepEqual(upsertDocument([{ id: 'a', sealed: false }], { id: 'a', sealed: true }), [
    { id: 'a', sealed: true },
  ]);
});
