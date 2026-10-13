import { createHash } from 'node:crypto';
export const accountId = '11111111-1111-4111-8111-111111111111';
export const caseId = '22222222-2222-4222-8222-222222222222';
export const documentId = '33333333-3333-4333-8333-333333333333';
export const bindingId = '77777777-7777-4777-8777-777777777777';
export const deploymentId = '88888888-8888-4888-8888-888888888888';
export const operationId = '99999999-9999-4999-8999-999999999999';
export const certificate = Buffer.from('public certificate DER fixture');
export const fingerprint = createHash('sha256').update(certificate).digest('hex');
export const pdf = Buffer.from('%PDF-1.7\nexact version\n%%EOF\n');
export const digest = createHash('sha256').update(pdf).digest('hex');
export const signature = Buffer.alloc(384, 23).toString('base64');
export const signatureDigest = createHash('sha256')
  .update(Buffer.from(signature, 'base64'))
  .digest('hex');
export const context = { accountId, caseId, documentId, version: 7, digest };
export const pem = Buffer.from(
  '-----BEGIN CERTIFICATE-----\n' +
    certificate.toString('base64') +
    '\n-----END CERTIFICATE-----\n',
).toString('base64');
export function statement(consent = false) {
  const bytes = Buffer.alloc(consent ? 258 : 142);
  bytes.write(consent ? 'DOCSIGN1' : 'DOCBIND1');
  bytes[8] = bytes[9] = 1;
  const uuid = (value, offset) => Buffer.from(value.replaceAll('-', ''), 'hex').copy(bytes, offset);
  uuid(deploymentId, 10);
  bytes.fill(44, 26, 58);
  bytes.writeUInt32BE(3, 58);
  uuid(accountId, 62);
  bytes.writeBigUInt64BE(9n, 78);
  bytes.writeBigUInt64BE(4n, 86);
  uuid(bindingId, 94);
  Buffer.from(fingerprint, 'hex').copy(bytes, 110);
  if (consent) {
    uuid(operationId, 142);
    uuid(caseId, 158);
    uuid(documentId, 174);
    bytes.writeUInt32BE(7, 190);
    Buffer.from(digest, 'hex').copy(bytes, 194);
    Buffer.from(signatureDigest, 'hex').copy(bytes, 226);
  }
  return bytes.toString('base64');
}
export function binding(receipt = false) {
  return {
    binding_id: bindingId,
    account_id: accountId,
    policy: 'internal_individual_signing_v1',
    statement_base64: statement(),
    account_revision: '9',
    auth_generation: '4',
    deployment_id: deploymentId,
    trust_revision: 3,
    root_fingerprint: '2c'.repeat(32),
    certificate: {
      der_base64: certificate.toString('base64'),
      fingerprint,
      summary: {
        subject: 'CN=Example',
        issuer: 'CN=Root',
        serial_hex: '01',
        not_before_unix: 0,
        not_after_unix: 2000000000,
      },
    },
    ...(receipt
      ? { signature_base64: signature, registered_at: '2026-10-10T00:00:00Z', withdrawn_at: null }
      : { account_label: 'litigator@example.test' }),
  };
}
export function consent(receipt = false) {
  return {
    operation_id: operationId,
    case_id: caseId,
    document_id: documentId,
    version: 7,
    document_digest: digest,
    pdf_signature_digest: signatureDigest,
    account_id: accountId,
    binding_id: bindingId,
    certificate_fingerprint: fingerprint,
    trust_revision: 3,
    account_revision: '9',
    auth_generation: '4',
    consent_base64: statement(true),
    policy: 'internal_individual_signing_v1',
    ...(receipt
      ? {
          status: 'accepted',
          accepted_at: '2026-10-10T00:00:00Z',
        }
      : { name: 'exact.pdf' }),
  };
}
