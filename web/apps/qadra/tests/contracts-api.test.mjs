import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createApi } from '../src/lib/api.mjs';
import { contractsApi } from '../src/lib/contracts-api.mjs';
import { messages } from '../src/lib/api-messages.mjs';
import {
  inputs,
  pdf,
  requestDetail,
  preparedStatement,
  signatureReceipt,
} from './contract-fixture.mjs';

const operation = '0e1d2c3b-4a59-4687-9a6b-5c4d3e2f1a0b';
function recorder(responses) {
  const calls = [];
  const request = async (path, options = {}) => {
    calls.push({ path, ...options });
    const value = responses[path];
    if (value instanceof Error) throw value;
    return typeof value === 'function' ? value(options) : value;
  };
  return { calls, request };
}

test('the request detail must decode to the same parties, version, digest and expiry', async () => {
  const detail = requestDetail();
  const { request } = recorder({ [`/contract-requests/${inputs.request_id}`]: detail });
  const value = await contractsApi(request).requests.get(inputs.request_id);
  assert.equal(value.request.documentDigest, inputs.document_sha256);
  for (const [field, changed] of [
    ['document_digest', '00'.repeat(32)],
    ['client_id', inputs.firm_account_id],
    ['version', 8],
    ['expires_at_unix', inputs.expires_at_seconds + 1],
    ['firm_certificate_fingerprint', inputs.client_certificate_sha256],
  ]) {
    const tampered = recorder({
      [`/contract-requests/${inputs.request_id}`]: { ...detail, [field]: changed },
    });
    await assert.rejects(contractsApi(tampered.request).requests.get(inputs.request_id));
  }
});

test('the exact PDF is accepted only when its recomputed digest matches the request', async () => {
  const path = `/contract-requests/${inputs.request_id}/content`;
  const good = recorder({ [path]: { blob: new Blob([pdf]) } });
  const blob = await contractsApi(good.request).requests.content(
    inputs.request_id,
    inputs.document_sha256,
  );
  assert.equal(blob.type, 'application/pdf');
  assert.equal(good.calls[0].binary, 'content');
  const bad = recorder({ [path]: { blob: new Blob([Buffer.concat([pdf, Buffer.from('x')])]) } });
  await assert.rejects(
    contractsApi(bad.request).requests.content(inputs.request_id, inputs.document_sha256),
  );
});

test('prepare checks position, account, certificate, request digest and operation', async () => {
  const detail = requestDetail();
  const path = `/contract-requests/${inputs.request_id}/prepare`;
  const prepared = preparedStatement(1, detail, operation);
  const { request, calls } = recorder({ [path]: prepared });
  const expected = {
    detail,
    position: 1,
    accountId: inputs.client_account_id,
    fingerprint: detail.client_certificate_fingerprint,
  };
  const value = await contractsApi(request).requests.prepare(operation, expected);
  assert.equal(value.statement.position, 1);
  assert.deepEqual(calls[0].data, { operation_id: operation });
  for (const changed of [
    { ...expected, position: 2 },
    { ...expected, accountId: inputs.firm_account_id },
    { ...expected, fingerprint: detail.firm_certificate_fingerprint },
  ])
    await assert.rejects(contractsApi(request).requests.prepare(operation, changed));
  const other = recorder({ [path]: preparedStatement(1, detail, inputs.request_id) });
  await assert.rejects(contractsApi(other.request).requests.prepare(operation, expected));
});

test('sign sends the exact statement and accepts only the matching receipt', async () => {
  const detail = requestDetail();
  const prepared = preparedStatement(1, detail, operation);
  const signature = Buffer.alloc(384, 7).toString('base64');
  const path = `/contract-requests/${inputs.request_id}/sign`;
  const receipt = signatureReceipt(detail, prepared, signature, 'client_signed');
  const { request, calls } = recorder({ [path]: receipt });
  const input = { prepared, signatureBase64: signature };
  assert.deepEqual(await contractsApi(request).requests.sign(inputs.request_id, input), receipt);
  assert.deepEqual(calls[0].data, {
    operation_id: operation,
    expected_revision: detail.revision,
    statement_base64: prepared.statement_base64,
    signature_base64: signature,
  });
  const other = recorder({
    [path]: signatureReceipt(detail, prepared, Buffer.alloc(384, 8).toString('base64'), 'x'),
  });
  await assert.rejects(contractsApi(other.request).requests.sign(inputs.request_id, input));
  await assert.rejects(
    contractsApi(request).requests.sign(inputs.request_id, { prepared, signatureBase64: 'AA==' }),
  );
});

test('close admits only the three outcomes and requires a reason to cancel', async () => {
  const path = `/contract-requests/${inputs.request_id}/close`;
  const { request, calls } = recorder({ [path]: { event: { kind: 'rejected' } } });
  const api = contractsApi(request).requests;
  await api.close(inputs.request_id, {
    operationId: operation,
    expectedRevision: 2,
    outcome: 'reject',
  });
  assert.deepEqual(calls[0].data, {
    operation_id: operation,
    expected_revision: 2,
    outcome: 'reject',
  });
  await assert.rejects(
    api.close(inputs.request_id, {
      operationId: operation,
      expectedRevision: 2,
      outcome: 'cancel',
    }),
  );
  await assert.rejects(
    api.close(inputs.request_id, { operationId: operation, expectedRevision: 2, outcome: 'void' }),
  );
  await api.close(inputs.request_id, {
    operationId: operation,
    expectedRevision: 2,
    outcome: 'cancel',
    reason: 'Cambio de version',
  });
  assert.equal(calls[1].data.reason, 'Cambio de version');
});

test('case routes create, request, abandon and read receipts with exact members', async () => {
  const caseId = inputs.case_id;
  const contract = '2d7c1a90-5b8e-4f61-9a3c-7e2b4d6f8a10';
  const { request, calls } = recorder({});
  const scoped = contractsApi(request).caseContracts(caseId);
  await scoped.create({
    operationId: operation,
    documentId: inputs.document_id,
    title: 'Contrato',
  });
  await scoped.request(contract, {
    requestId: inputs.request_id,
    operationId: operation,
    expectedRevision: 1,
    version: 3,
    clientEmail: 'cliente@example.test',
    firmSignerId: inputs.firm_account_id,
    ttlDays: 7,
    ceremonyReference: 'HF-0001',
  });
  await scoped.abandon(contract, {
    operationId: operation,
    expectedRevision: 1,
    reason: 'Sin uso',
  });
  await scoped.operation(operation);
  assert.deepEqual(
    calls.map((call) => [call.method ?? 'GET', call.path]),
    [
      ['POST', `/cases/${caseId}/contracts`],
      ['POST', `/cases/${caseId}/contracts/${contract}/requests`],
      ['POST', `/cases/${caseId}/contracts/${contract}/abandon`],
      ['GET', `/cases/${caseId}/contracts/operations/${operation}`],
    ],
  );
  assert.deepEqual(Object.keys(calls[1].data).sort(), [
    'ceremony_reference',
    'client_email',
    'expected_revision',
    'firm_signer_id',
    'operation_id',
    'request_id',
    'ttl_days',
    'version',
  ]);
  assert.throws(() => contractsApi(request).caseContracts('not-a-uuid'));
});

test('evidence downloads only the partial or final stage', async () => {
  const path = (stage) => `/contract-requests/${inputs.request_id}/evidence/${stage}`;
  const { request, calls } = recorder({ [path('partial')]: { blob: new Blob(['zip']) } });
  const blob = await contractsApi(request).requests.evidence(inputs.request_id, 'partial');
  assert.ok(blob instanceof Blob);
  assert.equal(calls[0].binary, true);
  await assert.rejects(contractsApi(request).requests.evidence(inputs.request_id, 'other'));
});

test('every contract error code of the HTTP contract has a plain message', () => {
  const contract = readFileSync(
    new URL('../../../../docs/http-contract-signing.md', import.meta.url),
    'utf8',
  );
  const codes = [...contract.matchAll(/^\| \d{3} \| `([a-z_]+)` \|/gm)].map((match) => match[1]);
  codes.push(
    'signing_account_changed',
    'signing_trust_changed',
    'signing_conflict',
    'signing_binding_inactive',
    'signing_credential_rejected',
    'signing_trust_unavailable',
  );
  assert.ok(codes.length >= 20);
  for (const code of codes) assert.equal(typeof messages[code], 'string', code);
});

test('a refused request keeps the readiness blockers of the error envelope', async () => {
  const api = createApi(async () =>
    Response.json(
      {
        error: {
          code: 'contract_party_ineligible',
          message: 'not ready',
          blockers: ['client_not_ready', 'document_not_ready'],
        },
      },
      { status: 422 },
    ),
  );
  await assert.rejects(api.me(), (error) => {
    assert.deepEqual(error.blockers, ['client_not_ready', 'document_not_ready']);
    assert.equal(error.code, 'contract_party_ineligible');
    return true;
  });
});

test('the document version view reaches the contract routes through the account request', async () => {
  const { versionApi } = await import('../src/lib/version-api.mjs');
  const paths = [];
  const version = versionApi(
    async () => assert.fail('the document scope must not carry contract routes'),
    inputs.case_id,
    async (path) => {
      paths.push(path);
      return { contracts: [], firm_signers: [] };
    },
  ).version(inputs.document_id, 1);
  await version.contracts().caseContracts(inputs.case_id).list();
  assert.deepEqual(paths, [`/cases/${inputs.case_id}/contracts`]);
  version.dispose();
  await assert.rejects(version.contracts().caseContracts(inputs.case_id).list());
});
