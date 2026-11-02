// Decoders of the two binary statements of bilateral contract signing: the
// CTRTREQ1 request (240 bytes) and the CTRTSIG1 statement each party signs
// (303 bytes). They apply every rule of the domain and validator decoders
// and must decode crates/domain/tests/fixtures/contract_statement_vectors.json
// to its values; they check no signature and recompute no digest. Signed and
// unsigned 64-bit values come back as decimal strings, so the request instants
// accept every value the Rust decoders accept.
// Rationale: docs/adr/0086-bilateral-contract-signing.md.

import { ownerBase64, ownerCertificateInvalid as invalid } from './owner-certificate-binary.mjs';

export const CONTRACT_REQUEST_BYTES = 240;
export const CONTRACT_STATEMENT_BYTES = 303;

const NIL = '00000000-0000-0000-0000-000000000000';
const ZERO_DIGEST = '00'.repeat(32);
const I64_MAX = 9223372036854775807n;

function reader(value, length, magic) {
  const bytes = ownerBase64(value, length);
  if (new TextDecoder().decode(bytes.subarray(0, 8)) !== magic || bytes[8] !== 1 || bytes[9] !== 1)
    invalid();
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const hex = (start, size) =>
    Array.from(bytes.subarray(start, start + size), (byte) =>
      byte.toString(16).padStart(2, '0'),
    ).join('');
  const uuid = (start) => {
    const raw = hex(start, 16);
    const text = [
      raw.slice(0, 8),
      raw.slice(8, 12),
      raw.slice(12, 16),
      raw.slice(16, 20),
      raw.slice(20),
    ].join('-');
    if (text === NIL) invalid();
    return text;
  };
  return { bytes, view, hex, uuid };
}

export function decodeContractRequest(value) {
  const { bytes, view, hex, uuid } = reader(value, CONTRACT_REQUEST_BYTES, 'CTRTREQ1');
  const deploymentId = uuid(10),
    requestId = uuid(26),
    caseId = uuid(42),
    documentId = uuid(58);
  const clientAccountId = uuid(110),
    firmAccountId = uuid(126),
    requesterAccountId = uuid(142);
  const documentVersion = view.getUint32(74);
  if (!documentVersion || bytes[238] !== 2 || bytes[239] !== 1) invalid();
  if (clientAccountId === firmAccountId || clientAccountId === requesterAccountId) invalid();
  const clientLeafFingerprint = hex(158, 32),
    firmLeafFingerprint = hex(190, 32);
  if (clientLeafFingerprint === firmLeafFingerprint) invalid();
  const createdAt = view.getBigInt64(222),
    expiresAt = view.getBigInt64(230);
  if (createdAt >= expiresAt) invalid();
  return Object.freeze({
    deploymentId,
    requestId,
    caseId,
    documentId,
    documentVersion,
    documentDigest: hex(78, 32),
    clientAccountId,
    firmAccountId,
    requesterAccountId,
    clientLeafFingerprint,
    firmLeafFingerprint,
    createdAt: createdAt.toString(),
    expiresAt: expiresAt.toString(),
  });
}

export function decodeContractStatement(value) {
  const { bytes, view, hex, uuid } = reader(value, CONTRACT_STATEMENT_BYTES, 'CTRTSIG1');
  const accountId = uuid(62);
  const accountRevision = view.getBigUint64(78),
    authGeneration = view.getBigUint64(86);
  if (accountRevision > I64_MAX || authGeneration > accountRevision) invalid();
  const deploymentId = uuid(10),
    bindingId = uuid(94);
  const trustRevision = view.getUint32(58);
  if (!trustRevision) invalid();
  const position = bytes[238];
  if (position !== 1 && position !== 2) invalid();
  const operationId = uuid(142),
    requestId = uuid(158);
  const priorSignature = hex(239, 32),
    priorToken = hex(271, 32);
  const chained = priorSignature !== ZERO_DIGEST && priorToken !== ZERO_DIGEST;
  const empty = priorSignature === ZERO_DIGEST && priorToken === ZERO_DIGEST;
  if (position === 1 ? !empty : !chained) invalid();
  return Object.freeze({
    deploymentId,
    rootFingerprint: hex(26, 32),
    trustRevision,
    accountId,
    accountRevision: accountRevision.toString(),
    authGeneration: authGeneration.toString(),
    bindingId,
    leafFingerprint: hex(110, 32),
    operationId,
    requestId,
    requestDigest: hex(174, 32),
    documentDigest: hex(206, 32),
    position,
    priorSignatureDigest: position === 2 ? priorSignature : null,
    priorTokenDigest: position === 2 ? priorToken : null,
  });
}
