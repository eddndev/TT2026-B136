import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import {
  CONTRACT_REQUEST_BYTES,
  CONTRACT_STATEMENT_BYTES,
  decodeContractRequest,
  decodeContractStatement,
} from '../src/lib/contract-signing-values.mjs';

const vectors = JSON.parse(
  readFileSync(
    new URL(
      '../../../../crates/domain/tests/fixtures/contract_statement_vectors.json',
      import.meta.url,
    ),
  ),
);
const inputs = vectors.request_inputs;
const bytes = (name) => Buffer.from(vectors[name].hex, 'hex');
const base64 = (buffer) => buffer.toString('base64');

test('the shared vector has the lengths and digests the codec writes', () => {
  assert.equal(CONTRACT_REQUEST_BYTES, 240);
  assert.equal(CONTRACT_STATEMENT_BYTES, 303);
  for (const name of ['request', 'client_statement', 'firm_statement']) {
    const data = bytes(name);
    assert.equal(data.length, vectors[name].length);
    assert.equal(createHash('sha256').update(data).digest('hex'), vectors[name].sha256);
  }
});

test('the request decodes to the values of the shared vector', () => {
  assert.deepEqual(decodeContractRequest(base64(bytes('request'))), {
    deploymentId: inputs.deployment_id,
    requestId: inputs.request_id,
    caseId: inputs.case_id,
    documentId: inputs.document_id,
    documentVersion: inputs.document_version,
    documentDigest: inputs.document_sha256,
    clientAccountId: inputs.client_account_id,
    firmAccountId: inputs.firm_account_id,
    requesterAccountId: inputs.requester_account_id,
    clientLeafFingerprint: inputs.client_certificate_sha256,
    firmLeafFingerprint: inputs.firm_certificate_sha256,
    createdAt: inputs.created_at_seconds,
    expiresAt: inputs.expires_at_seconds,
  });
  assert.ok(Object.isFrozen(decodeContractRequest(base64(bytes('request')))));
});

function expectedStatement(party, priorSignature, priorToken) {
  return {
    deploymentId: inputs.deployment_id,
    rootFingerprint: party.root_sha256,
    trustRevision: party.trust_revision,
    accountId: party.account_id,
    accountRevision: String(party.account_revision),
    authGeneration: String(party.authentication_generation),
    bindingId: party.binding_id,
    leafFingerprint: party.certificate_sha256,
    operationId: party.operation_id,
    requestId: inputs.request_id,
    requestDigest: vectors.request.sha256,
    documentDigest: inputs.document_sha256,
    position: party.position,
    priorSignatureDigest: priorSignature,
    priorTokenDigest: priorToken,
  };
}

test('the Client statement decodes in position 1 without a chain', () => {
  assert.deepEqual(
    decodeContractStatement(base64(bytes('client_statement'))),
    expectedStatement(vectors.client_party, null, null),
  );
});

test('the firm statement decodes in position 2 with the chain to the Client', () => {
  const firm = vectors.firm_party;
  assert.deepEqual(
    decodeContractStatement(base64(bytes('firm_statement'))),
    expectedStatement(firm, firm.prior_signature_sha256, firm.prior_token_sha256),
  );
});

function rejected(decode, data, label) {
  assert.throws(() => decode(base64(data)), Error, label);
}

function changed(name, edit) {
  const data = bytes(name);
  edit(data);
  return data;
}

test('a request is rejected for every rule of the codec', () => {
  const cases = {
    short: bytes('request').subarray(0, 239),
    long: Buffer.concat([bytes('request'), Buffer.alloc(1)]),
    magic: changed('request', (data) => data.write('CTRTSIG1')),
    format: changed('request', (data) => (data[9] = 2)),
    nil_deployment: changed('request', (data) => data.fill(0, 10, 26)),
    nil_request: changed('request', (data) => data.fill(0, 26, 42)),
    nil_case: changed('request', (data) => data.fill(0, 42, 58)),
    nil_document: changed('request', (data) => data.fill(0, 58, 74)),
    version: changed('request', (data) => data.fill(0, 74, 78)),
    nil_client: changed('request', (data) => data.fill(0, 110, 126)),
    nil_firm: changed('request', (data) => data.fill(0, 126, 142)),
    nil_requester: changed('request', (data) => data.fill(0, 142, 158)),
    parties: changed('request', (data) => (data[238] = 3)),
    rule: changed('request', (data) => (data[239] = 2)),
    client_is_firm: changed('request', (data) => data.copy(data, 126, 110, 126)),
    client_is_requester: changed('request', (data) => data.copy(data, 142, 110, 126)),
    same_leaf: changed('request', (data) => data.copy(data, 190, 158, 190)),
    validity: changed('request', (data) => data.copy(data, 230, 222, 230)),
  };
  for (const [label, data] of Object.entries(cases)) {
    rejected(decodeContractRequest, data, label);
  }
});

test('an Owner may both request and sign for the firm', () => {
  const data = changed('request', (data) => data.copy(data, 142, 126, 142));
  assert.equal(decodeContractRequest(base64(data)).requesterAccountId, inputs.firm_account_id);
});

test('a party statement is rejected for every rule of the codec', () => {
  const cases = {
    short: bytes('client_statement').subarray(0, 302),
    magic: changed('client_statement', (data) => data.write('DOCSIGN1')),
    format: changed('client_statement', (data) => (data[8] = 2)),
    nil_deployment: changed('client_statement', (data) => data.fill(0, 10, 26)),
    trust_revision: changed('client_statement', (data) => data.fill(0, 58, 62)),
    nil_account: changed('client_statement', (data) => data.fill(0, 62, 78)),
    account_revision: changed('client_statement', (data) => data.fill(0xff, 78, 86)),
    authentication: changed('client_statement', (data) => data.writeBigUInt64BE(6n, 86)),
    nil_binding: changed('client_statement', (data) => data.fill(0, 94, 110)),
    nil_operation: changed('client_statement', (data) => data.fill(0, 142, 158)),
    nil_request: changed('client_statement', (data) => data.fill(0, 158, 174)),
    position_zero: changed('client_statement', (data) => (data[238] = 0)),
    position_three: changed('client_statement', (data) => (data[238] = 3)),
    client_with_chain: changed('client_statement', (data) => (data[302] = 1)),
    firm_without_signature: changed('firm_statement', (data) => data.fill(0, 239, 271)),
    firm_without_token: changed('firm_statement', (data) => data.fill(0, 271, 303)),
  };
  for (const [label, data] of Object.entries(cases)) {
    rejected(decodeContractStatement, data, label);
  }
});

test('values that are not canonical base64 are rejected', () => {
  for (const value of [null, 7, '', `${base64(bytes('request'))}=`, '@'.repeat(320)]) {
    assert.throws(() => decodeContractRequest(value), Error);
  }
});
