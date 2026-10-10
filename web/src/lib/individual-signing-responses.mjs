import { ownerBase64, ownerCertificateInvalid as invalid } from './owner-certificate-binary.mjs';
import { ownerObject } from './owner-certificate-values.mjs';
import { decodeSigningStatement, signingDigest } from './individual-signing-values.mjs';
import { signingHash } from './individual-signing-files.mjs';

export function signingTime(value) {
  if (
    typeof value !== 'string' ||
    !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$/.test(value) ||
    !Number.isFinite(Date.parse(value))
  )
    invalid();
}

function match(value, canonical, mapping) {
  for (const [wire, key] of Object.entries(mapping)) if (value[wire] !== canonical[key]) invalid();
  if (value.policy !== 'internal_individual_signing_v1') invalid();
}
const common = {
  account_id: 'accountId',
  binding_id: 'bindingId',
  trust_revision: 'trustRevision',
  account_revision: 'accountRevision',
  auth_generation: 'authGeneration',
};

export async function signingBinding(value, accountId, bindingId, receipt = false) {
  const fields = [
    'binding_id',
    'account_id',
    'policy',
    'statement_base64',
    'account_revision',
    'auth_generation',
    'deployment_id',
    'trust_revision',
    'root_fingerprint',
    'certificate',
  ];
  ownerObject(value, [
    ...fields,
    ...(receipt ? ['signature_base64', 'registered_at', 'withdrawn_at'] : ['account_label']),
  ]);
  const canonical = decodeSigningStatement(value.statement_base64, 'binding');
  match(value, canonical, {
    ...common,
    deployment_id: 'deploymentId',
    root_fingerprint: 'rootFingerprint',
  });
  if (value.account_id !== accountId || (bindingId !== undefined && value.binding_id !== bindingId))
    invalid();
  const certificate = ownerObject(value.certificate, ['der_base64', 'fingerprint', 'summary']);
  const bytes = ownerBase64(certificate.der_base64, 1, 16384);
  if (
    certificate.fingerprint !== canonical.leafFingerprint ||
    (await signingHash(bytes)) !== certificate.fingerprint
  )
    invalid();
  const summary = ownerObject(certificate.summary, [
    'subject',
    'issuer',
    'serial_hex',
    'not_before_unix',
    'not_after_unix',
  ]);
  if (
    ['subject', 'issuer', 'serial_hex'].some(
      (key) => typeof summary[key] !== 'string' || summary[key].length > 4096,
    ) ||
    !Number.isSafeInteger(summary.not_before_unix) ||
    !Number.isSafeInteger(summary.not_after_unix) ||
    summary.not_before_unix > summary.not_after_unix
  )
    invalid();
  if (receipt) {
    ownerBase64(value.signature_base64, 384);
    signingTime(value.registered_at);
    if (value.withdrawn_at !== null) {
      signingTime(value.withdrawn_at);
      if (Date.parse(value.withdrawn_at) < Date.parse(value.registered_at)) invalid();
    }
  } else if (typeof value.account_label !== 'string' || value.account_label.length > 1024)
    invalid();
  return value;
}

export function signingConsent(value, context, operationId, receipt = false) {
  const fields = [
    'operation_id',
    'case_id',
    'document_id',
    'version',
    'document_digest',
    'pdf_signature_digest',
    'account_id',
    'binding_id',
    'certificate_fingerprint',
    'trust_revision',
    'account_revision',
    'auth_generation',
    'consent_base64',
    'policy',
  ];
  ownerObject(value, [...fields, ...(receipt ? ['status', 'accepted_at'] : ['name'])]);
  const canonical = decodeSigningStatement(value.consent_base64, 'consent');
  match(value, canonical, {
    ...common,
    operation_id: 'operationId',
    case_id: 'caseId',
    document_id: 'documentId',
    version: 'version',
    document_digest: 'digest',
    pdf_signature_digest: 'pdfSignatureDigest',
    certificate_fingerprint: 'leafFingerprint',
  });
  for (const key of ['accountId', 'caseId', 'documentId', 'version', 'digest'])
    if (canonical[key] !== context[key]) invalid();
  if (operationId !== undefined && canonical.operationId !== operationId) invalid();
  if (receipt) {
    if (value.status !== 'accepted') invalid();
    signingTime(value.accepted_at);
  } else if (typeof value.name !== 'string' || value.name.length > 124) invalid();
  return value;
}

export async function signingSubmission(input, context) {
  ownerObject(input, ['consent_base64', 'pdf_signature_base64', 'consent_signature_base64']);
  const canonical = decodeSigningStatement(input.consent_base64, 'consent');
  for (const key of ['accountId', 'caseId', 'documentId', 'version', 'digest'])
    if (canonical[key] !== context[key]) invalid();
  ownerBase64(input.consent_signature_base64, 384);
  if (
    (await signingHash(ownerBase64(input.pdf_signature_base64, 384))) !==
    canonical.pdfSignatureDigest
  )
    invalid();
  return Object.freeze({ ...input });
}

export function signingVerification(value, context) {
  ownerObject(value, [
    'kind',
    'historical',
    'certificate_current',
    'binding_withdrawn_at',
    'signing',
  ]);
  if (
    !['individual', 'legacy_server'].includes(value.kind) ||
    value.historical?.document_digest !== context.digest ||
    !['valid', 'not_valid'].includes(value.historical?.verdict)
  )
    invalid();
  const current = ownerObject(value.certificate_current, ['status', 'reason', 'checked_at_unix']);
  if (
    !['valid', 'rejected', 'trust_unavailable', 'not_individual'].includes(current.status) ||
    !Number.isSafeInteger(current.checked_at_unix) ||
    (current.reason !== null && (typeof current.reason !== 'string' || current.reason.length > 128))
  )
    invalid();
  if (value.kind === 'individual') {
    if (current.status === 'not_individual') invalid();
    signingConsent(
      value.signing,
      { ...context, accountId: value.signing?.account_id },
      undefined,
      true,
    );
    if (value.binding_withdrawn_at !== null) signingTime(value.binding_withdrawn_at);
  } else if (
    current.status !== 'not_individual' ||
    value.signing !== null ||
    value.binding_withdrawn_at !== null
  )
    invalid();
  signingDigest(context.digest);
  return value;
}
