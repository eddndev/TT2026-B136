import { ownerBase64, ownerCertificateInvalid as invalid } from './owner-certificate-binary.mjs';

export function signingUuid(value) {
  if (
    typeof value !== 'string' ||
    !/^[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12}$/.test(value) ||
    value === '00000000-0000-0000-0000-000000000000'
  )
    invalid();
  return value;
}

export function signingDigest(value) {
  if (typeof value !== 'string' || !/^[0-9a-f]{64}$/.test(value)) invalid();
  return value;
}

export function signingContext(accountId, caseId, documentId, version, digest) {
  [accountId, caseId, documentId].forEach(signingUuid);
  signingDigest(digest);
  if (!Number.isInteger(version) || version < 1 || version > 0xffffffff) invalid();
  return Object.freeze({ accountId, caseId, documentId, version, digest });
}

export function decodeSigningStatement(value, kind) {
  if (!['binding', 'consent'].includes(kind)) invalid();
  const bytes = ownerBase64(value, kind === 'binding' ? 142 : 258);
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const hex = (start, length) =>
    Array.from(bytes.subarray(start, start + length), (byte) =>
      byte.toString(16).padStart(2, '0'),
    ).join('');
  const uuid = (start) => {
    const raw = hex(start, 16);
    return signingUuid(
      [raw.slice(0, 8), raw.slice(8, 12), raw.slice(12, 16), raw.slice(16, 20), raw.slice(20)].join(
        '-',
      ),
    );
  };
  if (
    new TextDecoder().decode(bytes.subarray(0, 8)) !==
      (kind === 'binding' ? 'DOCBIND1' : 'DOCSIGN1') ||
    bytes[8] !== 1 ||
    bytes[9] !== 1
  )
    invalid();
  const accountRevision = view.getBigUint64(78),
    authGeneration = view.getBigUint64(86);
  const trustRevision = view.getUint32(58);
  if (!trustRevision || accountRevision > 9223372036854775807n || authGeneration > accountRevision)
    invalid();
  const result = {
    accountId: uuid(62),
    deploymentId: uuid(10),
    bindingId: uuid(94),
    rootFingerprint: hex(26, 32),
    leafFingerprint: hex(110, 32),
    trustRevision,
    accountRevision: accountRevision.toString(),
    authGeneration: authGeneration.toString(),
  };
  if (kind === 'consent') {
    const version = view.getUint32(190);
    if (!version) invalid();
    Object.assign(result, {
      operationId: uuid(142),
      caseId: uuid(158),
      documentId: uuid(174),
      version,
      digest: hex(194, 32),
      pdfSignatureDigest: hex(226, 32),
    });
  }
  return Object.freeze(result);
}

export function signingStatementDownload(value, kind) {
  decodeSigningStatement(value, kind);
  return new Blob([ownerBase64(value, kind === 'binding' ? 142 : 258)], {
    type: 'application/octet-stream',
  });
}
