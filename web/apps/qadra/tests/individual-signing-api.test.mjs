import test from 'node:test';
import assert from 'node:assert/strict';
import { signingCertificatesApi } from '../src/lib/signing-certificates-api.mjs';
import { individualSigningApi } from '../src/lib/individual-signing-api.mjs';
import { checkedSigningPdf } from '../src/lib/individual-signing-files.mjs';
import {
  accountId,
  bindingId,
  operationId,
  signature,
  context,
  pem,
  pdf,
  binding,
  consent,
} from './individual-signing-fixture.mjs';

function transport(result) {
  const calls = [];
  return {
    calls,
    request: async (path, options) => {
      calls.push({ path, options });
      return structuredClone(typeof result === 'function' ? result() : result);
    },
  };
}

test('public binding sends only public certificate, canonical proof and exact withdrawal path', async () => {
  let response = binding();
  const { calls, request } = transport(() => response);
  const api = signingCertificatesApi(request, accountId);
  const prepared = await api.prepare(bindingId, pem);
  assert.deepEqual(calls[0], {
    path: '/identity/signing-certificates/' + bindingId + '/prepare',
    options: { method: 'POST', data: { certificate_base64: pem } },
  });
  response = binding(true);
  const input = {
    statement_base64: prepared.statement_base64,
    certificate_der_base64: prepared.certificate.der_base64,
    signature_base64: signature,
  };
  assert.deepEqual(await api.register(bindingId, input), response);
  assert.deepEqual(calls[1].options.data, input);
  response.withdrawn_at = '2026-10-10T01:00:00Z';
  await api.withdraw(bindingId);
  assert.deepEqual(calls[2].options, { method: 'POST' });
  response = { binding: null };
  assert.equal(await api.current(), null);
  await assert.rejects(() => api.register(bindingId, { ...input, private_key: 'secret' }));
  await assert.rejects(() => api.prepare(bindingId, btoa('-----BEGIN PRIVATE KEY-----')));
  assert.equal(calls.length, 4);
  api.dispose();
  await assert.rejects(() => api.current());
  assert.equal(calls.length, 4);
});

test('binding rejects identity substitution, DER fingerprint changes and inconsistent counters', async () => {
  for (const change of [
    (value) => {
      value.account_id = bindingId;
    },
    (value) => {
      value.certificate.der_base64 = btoa('other');
    },
    (value) => {
      value.account_revision = '10';
    },
    (value) => {
      value.root_fingerprint = '00'.repeat(32);
    },
  ]) {
    const value = binding();
    change(value);
    const { request } = transport(value);
    await assert.rejects(() => signingCertificatesApi(request, accountId).prepare(bindingId, pem));
  }
});

test('document API binds consent and receipts to exact PDF signature and context without retries', async () => {
  let response = consent();
  const { calls, request } = transport(() => response);
  const api = individualSigningApi(request, context);
  const prepared = await api.prepare(operationId, signature);
  assert.deepEqual(calls[0], {
    path: '/individual-signing/prepare',
    options: {
      method: 'POST',
      data: { operation_id: operationId, pdf_signature_base64: signature },
    },
  });
  const input = {
    consent_base64: prepared.consent_base64,
    pdf_signature_base64: signature,
    consent_signature_base64: signature,
  };
  response = consent(true);
  assert.deepEqual(await api.submit(input), response);
  assert.deepEqual(calls[1].options.data, input);
  assert.deepEqual(await api.receipt(operationId, prepared), response);
  assert.equal(calls[2].path, '/individual-signing/receipts/' + operationId);
  await assert.rejects(() => api.submit({ ...input, pdf_signature_base64: btoa('short') }));
  assert.equal(calls.length, 3);
  let attempts = 0;
  const lost = individualSigningApi(async () => {
    attempts++;
    throw new Error('Lost response');
  }, context);
  await assert.rejects(() => lost.submit(input));
  assert.equal(attempts, 1);
});

test('consent mismatches fail closed and PDF downloads require exact content bytes', async () => {
  for (const change of [
    (value) => {
      value.version++;
    },
    (value) => {
      value.operation_id = bindingId;
    },
    (value) => {
      value.account_id = bindingId;
    },
    (value) => {
      value.pdf_signature_digest = '00'.repeat(32);
    },
  ]) {
    const value = consent();
    change(value);
    await assert.rejects(() =>
      individualSigningApi(transport(value).request, context).prepare(operationId, signature),
    );
  }
  const checked = await checkedSigningPdf(new Blob([pdf]), context.digest);
  assert.equal(checked.type, 'application/pdf');
  assert.deepEqual(Buffer.from(await checked.arrayBuffer()), pdf);
  await assert.rejects(() => checkedSigningPdf(new Blob([pdf, 'modified']), context.digest));
  await assert.rejects(() => checkedSigningPdf(new Blob(['not a PDF']), context.digest));
});

test('historical individual verification preserves current rejection and withdrawal separately', async () => {
  const value = {
    kind: 'individual',
    historical: {
      document_digest: context.digest,
      verdict: 'valid',
    },
    signing: consent(true),
    certificate_current: {
      status: 'rejected',
      reason: 'certificate_revoked',
      checked_at_unix: 2000000000,
    },
    binding_withdrawn_at: '2026-10-10T01:00:00Z',
  };
  const api = individualSigningApi(transport(value).request, context);
  assert.deepEqual(await api.verification(), value);
  value.historical.document_digest = '0'.repeat(64);
  await assert.rejects(() =>
    individualSigningApi(transport(value).request, context).verification(),
  );
});

test('integrated API keeps personal operations inside authenticated account and selected version', async () => {
  const { createApi } = await import('../src/lib/api.mjs');
  let requests = 0;
  const api = createApi(async (url) => {
    requests++;
    if (url.includes('/mfa/'))
      return Response.json({ access_token: 'opaque', user: { id: accountId } });
    if (url.endsWith('/current')) return Response.json({ binding: null });
    return Response.json(consent());
  });
  await api.mfa('challenge', '123456', 'totp');
  const certificates = api.signingCertificates(accountId);
  assert.equal(await certificates.current(), null);
  const selected = api.caseDocuments(context.caseId).version(context.documentId, context.version);
  const personal = selected.individualSigning(accountId, context.digest);
  await personal.prepare(operationId, signature);
  selected.dispose();
  const count = requests;
  await assert.rejects(() => personal.prepare(operationId, signature));
  assert.equal(requests, count);
  api.invalidateSession();
  await assert.rejects(() => certificates.current());
  assert.equal(requests, count);
});
