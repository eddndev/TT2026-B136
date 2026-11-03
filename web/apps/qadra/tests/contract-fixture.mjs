// Consistent contract request and statement responses built from the shared
// vectors of crates/domain/tests/fixtures/contract_statement_vectors.json.
// Overrides patch the exact byte offsets of the CTRTREQ1 and CTRTSIG1
// layouts, so a test can bind a locally generated certificate or a fresh
// operation without breaking the decoders.
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

export const vectors = JSON.parse(
  readFileSync(
    new URL(
      '../../../../crates/domain/tests/fixtures/contract_statement_vectors.json',
      import.meta.url,
    ),
  ),
);
export const inputs = vectors.request_inputs;
export const pdf = Buffer.from(inputs.document_hex, 'hex');
const sha = (data) => createHash('sha256').update(data).digest('hex');
const uuid = (value) => Buffer.from(value.replaceAll('-', ''), 'hex');

export function requestBytes({ clientFingerprint, firmFingerprint, expiresAt } = {}) {
  const data = Buffer.from(vectors.request.hex, 'hex');
  if (clientFingerprint) Buffer.from(clientFingerprint, 'hex').copy(data, 158);
  if (firmFingerprint) Buffer.from(firmFingerprint, 'hex').copy(data, 190);
  if (expiresAt !== undefined) data.writeBigInt64BE(BigInt(expiresAt), 230);
  return data;
}

export function statementBytes(position, request, { fingerprint, operationId } = {}) {
  const data = Buffer.from(
    vectors[position === 1 ? 'client_statement' : 'firm_statement'].hex,
    'hex',
  );
  if (fingerprint) Buffer.from(fingerprint, 'hex').copy(data, 110);
  if (operationId) uuid(operationId).copy(data, 142);
  Buffer.from(sha(request), 'hex').copy(data, 174);
  return data;
}

const certificate = (fingerprint, subject) => ({
  der_base64: Buffer.from('certificate').toString('base64'),
  fingerprint,
  summary: {
    subject,
    issuer: 'CN=Qadra Demo Root',
    serial_hex: '01',
    not_before_unix: 1790000000,
    not_after_unix: 1890000000,
  },
});

export function requestDetail(overrides = {}) {
  const clientFingerprint = overrides.clientFingerprint ?? inputs.client_certificate_sha256;
  const firmFingerprint = overrides.firmFingerprint ?? inputs.firm_certificate_sha256;
  const expiresAt = overrides.expiresAt ?? inputs.expires_at_seconds;
  const request = requestBytes({ clientFingerprint, firmFingerprint, expiresAt });
  return {
    request_id: inputs.request_id,
    status: overrides.status ?? 'awaiting_client',
    contract_id: '2d7c1a90-5b8e-4f61-9a3c-7e2b4d6f8a10',
    document_id: inputs.document_id,
    version: inputs.document_version,
    document_digest: inputs.document_sha256,
    client_id: inputs.client_account_id,
    firm_signer_id: inputs.firm_account_id,
    requester_id: inputs.requester_account_id,
    requester_email: 'litigante@example.test',
    firm_signer_role: 'litigator',
    client_certificate_fingerprint: clientFingerprint,
    firm_certificate_fingerprint: firmFingerprint,
    created_at_unix: inputs.created_at_seconds,
    expires_at_unix: expiresAt,
    created_at: '2026-11-02T00:00:00Z',
    ceremony_reference: 'HF-0001',
    subject: null,
    client_signed_at: overrides.clientSignedAt ?? null,
    firm_signed_at: null,
    completed_at: overrides.completedAt ?? null,
    statement_base64: request.toString('base64'),
    revision: overrides.revision ?? 2,
    title: 'Contrato de prestacion de servicios',
    case_id: inputs.case_id,
    contract_state: overrides.contractState ?? 'en_firma',
    client_certificate: certificate(clientFingerprint, 'CN=Cliente Demo,O=Despacho,C=MX'),
    firm_certificate: certificate(firmFingerprint, 'CN=Litigante Demo,O=Despacho,C=MX'),
    signatures: [],
  };
}

export function preparedStatement(position, detail, operationId) {
  const request = Buffer.from(detail.statement_base64, 'base64');
  const fingerprint =
    position === 1 ? detail.client_certificate_fingerprint : detail.firm_certificate_fingerprint;
  const data = statementBytes(position, request, { fingerprint, operationId });
  const party = position === 1 ? vectors.client_party : vectors.firm_party;
  return {
    position,
    account_id: party.account_id,
    account_revision: String(party.account_revision),
    auth_generation: String(party.authentication_generation),
    binding_id: party.binding_id,
    deployment_id: inputs.deployment_id,
    root_fingerprint: party.root_sha256,
    trust_revision: party.trust_revision,
    certificate_fingerprint: fingerprint,
    operation_id: operationId,
    request_id: detail.request_id,
    request_sha256: sha(request),
    document_digest: detail.document_digest,
    client_signature_sha256: position === 1 ? null : party.prior_signature_sha256,
    client_timestamp_sha256: position === 1 ? null : party.prior_token_sha256,
    statement_base64: data.toString('base64'),
    revision: detail.revision,
  };
}

export function signatureReceipt(detail, prepared, signatureBase64, kind) {
  return {
    contract_id: detail.contract_id,
    case_id: detail.case_id,
    event: {
      revision: detail.revision + 1,
      kind,
      request_id: detail.request_id,
      actor_id: prepared.account_id,
      actor_label: 'parte',
      reason: null,
      at: '2026-11-02T01:00:00Z',
      operation_id: prepared.operation_id,
    },
    request_statement_base64: null,
    signature: {
      ...prepared,
      signature_base64: signatureBase64,
      accepted_at: '2026-11-02T01:00:00Z',
      timestamp_base64: prepared.position === 1 ? 'AAAA' : null,
    },
    manifest_sha256: kind === 'completed' ? 'ab'.repeat(32) : null,
  };
}
