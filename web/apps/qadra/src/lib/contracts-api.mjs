// Client of the bilateral contract routes. Every response that a party
// signs over is checked against the exact CTRTREQ1 and CTRTSIG1 bytes before
// the interface shows or signs it: parties, version, digest, expiry,
// position, account, certificate and operation must match, and a firm
// statement must chain to the stored Client signature and its token. No private key,
// password or certificate container ever passes through this module.
// Rationale: docs/adr/0086-bilateral-contract-signing.md.

import { ownerBase64, ownerCertificateInvalid as invalid } from './owner-certificate-binary.mjs';
import { signingDigest, signingUuid } from './individual-signing-values.mjs';
import { checkedSigningPdf, signingHash } from './individual-signing-files.mjs';
import { decodeContractRequest, decodeContractStatement } from './contract-signing-values.mjs';
import { contractReason } from './contract-request-draft.mjs';

const OUTCOMES = ['reject', 'decline', 'cancel'];
const STAGES = ['partial', 'final'];
const TOKEN_LIMIT = 65536;

function revision(value) {
  if (!Number.isSafeInteger(value) || value < 1) invalid();
  return value;
}

export function checkedRequestDetail(value) {
  if (!value || typeof value !== 'object') invalid();
  const request = decodeContractRequest(value.statement_base64);
  if (
    request.requestId !== value.request_id ||
    request.documentId !== value.document_id ||
    request.caseId !== value.case_id ||
    request.documentVersion !== value.version ||
    request.documentDigest !== value.document_digest ||
    request.clientAccountId !== value.client_id ||
    request.firmAccountId !== value.firm_signer_id ||
    request.clientLeafFingerprint !== value.client_certificate_fingerprint ||
    request.firmLeafFingerprint !== value.firm_certificate_fingerprint ||
    request.expiresAt !== String(value.expires_at_unix) ||
    value.client_certificate?.fingerprint !== value.client_certificate_fingerprint ||
    value.firm_certificate?.fingerprint !== value.firm_certificate_fingerprint
  )
    invalid();
  revision(value.revision);
  return { ...value, request };
}

// The firm statement must chain to the Client signature and T1 that the
// request detail serves, under the deployment and root of that signature.
async function checkedChain(statement, detail) {
  if (statement.position !== 2) return;
  const rows = Array.isArray(detail.signatures) ? detail.signatures : [];
  const client = rows.find((row) => row?.position === 1);
  if (!client) invalid();
  const signed = decodeContractStatement(client.statement_base64);
  const signature = await signingHash(ownerBase64(client.signature_base64, 384));
  const token = await signingHash(ownerBase64(client.timestamp_base64, 1, TOKEN_LIMIT));
  if (
    signed.position !== 1 ||
    signed.requestId !== statement.requestId ||
    signed.deploymentId !== statement.deploymentId ||
    signed.rootFingerprint !== statement.rootFingerprint ||
    statement.priorSignatureDigest !== signature ||
    statement.priorTokenDigest !== token
  )
    invalid();
}

async function checkedPrepared(value, operationId, expected) {
  const statement = decodeContractStatement(value?.statement_base64);
  const requestDigest = await signingHash(ownerBase64(expected.detail.statement_base64, 240));
  if (
    statement.position !== expected.position ||
    statement.accountId !== expected.accountId ||
    statement.leafFingerprint !== expected.fingerprint ||
    statement.operationId !== operationId ||
    statement.requestId !== expected.detail.request_id ||
    statement.requestDigest !== requestDigest ||
    statement.documentDigest !== expected.detail.document_digest ||
    value.operation_id !== operationId
  )
    invalid();
  await checkedChain(statement, expected.detail);
  revision(value.revision);
  return { ...value, statement };
}

function caseContracts(request, caseId) {
  signingUuid(caseId);
  const base = `/cases/${caseId}/contracts`;
  const post = (path, data) => request(base + path, { method: 'POST', data });
  return {
    list: () => request(base),
    get: (contractId) => request(`${base}/${signingUuid(contractId)}`),
    create({ operationId, documentId, title }) {
      return post('', {
        operation_id: signingUuid(operationId),
        document_id: signingUuid(documentId),
        title,
      });
    },
    request(contractId, input) {
      signingUuid(contractId);
      return post(`/${contractId}/requests`, {
        request_id: signingUuid(input.requestId),
        operation_id: signingUuid(input.operationId),
        expected_revision: revision(input.expectedRevision),
        version: revision(input.version),
        client_email: input.clientEmail,
        firm_signer_id: signingUuid(input.firmSignerId),
        ttl_days: input.ttlDays,
        ceremony_reference: input.ceremonyReference,
      });
    },
    abandon(contractId, { operationId, expectedRevision, reason }) {
      signingUuid(contractId);
      return post(`/${contractId}/abandon`, {
        operation_id: signingUuid(operationId),
        expected_revision: revision(expectedRevision),
        reason: contractReason(reason, true),
      });
    },
    operation: (operationId) => request(`${base}/operations/${signingUuid(operationId)}`),
  };
}

function contractRequests(request) {
  const base = (requestId) => `/contract-requests/${signingUuid(requestId)}`;
  const post = (requestId, path, data) => request(base(requestId) + path, { method: 'POST', data });
  return {
    async list() {
      const value = await request('/contract-requests');
      if (!Array.isArray(value?.requests)) invalid();
      return value.requests;
    },
    async get(requestId) {
      const value = checkedRequestDetail(await request(base(requestId)));
      if (value.request_id !== requestId) invalid();
      return value;
    },
    async content(requestId, digest) {
      signingDigest(digest);
      const value = await request(`${base(requestId)}/content`, { binary: 'content' });
      return checkedSigningPdf(value.blob, digest);
    },
    async prepare(operationId, expected) {
      signingUuid(operationId);
      const requestId = expected.detail.request_id;
      return checkedPrepared(
        await post(requestId, '/prepare', { operation_id: operationId }),
        operationId,
        expected,
      );
    },
    async sign(requestId, { prepared, signatureBase64 }) {
      ownerBase64(signatureBase64, 384);
      const data = {
        operation_id: signingUuid(prepared.operation_id),
        expected_revision: revision(prepared.revision),
        statement_base64: prepared.statement_base64,
        signature_base64: signatureBase64,
      };
      const value = await post(requestId, '/sign', data);
      if (
        value?.signature?.statement_base64 !== data.statement_base64 ||
        value.signature.signature_base64 !== signatureBase64
      )
        invalid();
      return value;
    },
    close(requestId, { operationId, expectedRevision, outcome, reason }) {
      if (!OUTCOMES.includes(outcome)) return Promise.reject(invalidError());
      let data;
      try {
        data = {
          operation_id: signingUuid(operationId),
          expected_revision: revision(expectedRevision),
          outcome,
        };
        const text = contractReason(reason, outcome === 'cancel');
        if (text) data.reason = text;
      } catch (failure) {
        return Promise.reject(failure);
      }
      return post(requestId, '/close', data);
    },
    operation: (requestId, operationId) =>
      request(`${base(requestId)}/operations/${signingUuid(operationId)}`),
    async evidence(requestId, stage) {
      if (!STAGES.includes(stage)) throw invalidError();
      const value = await request(`${base(requestId)}/evidence/${stage}`, { binary: true });
      if (!(value?.blob instanceof Blob)) invalid();
      return value.blob;
    },
  };
}

function invalidError() {
  try {
    invalid();
  } catch (failure) {
    return failure;
  }
}

export function contractsApi(request) {
  return {
    caseContracts: (caseId) => caseContracts(request, caseId),
    requests: contractRequests(request),
  };
}
