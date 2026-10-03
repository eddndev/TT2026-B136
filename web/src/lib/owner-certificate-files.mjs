import {
  OWNER_CERTIFICATE_LIMIT,
  ownerCertificateInvalid,
  ownerPublicPem,
  ownerStatementBytes,
} from './owner-certificate-binary.mjs';

export async function readOwnerPublicCertificate(file) {
  const size = file?.size;
  if (
    !Number.isSafeInteger(size) ||
    size < 1 ||
    size > OWNER_CERTIFICATE_LIMIT ||
    typeof file.arrayBuffer !== 'function'
  )
    ownerCertificateInvalid();
  const buffer = await file.arrayBuffer();
  if (!(buffer instanceof ArrayBuffer) || buffer.byteLength !== size) ownerCertificateInvalid();
  const bytes = ownerPublicPem(new Uint8Array(buffer));
  return new Blob([bytes], { type: 'application/octet-stream' });
}

export function ownerStatementDownload(statementBase64) {
  return new Blob([ownerStatementBytes(statementBase64)], { type: 'application/octet-stream' });
}
