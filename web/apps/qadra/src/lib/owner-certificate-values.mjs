import {
  OWNER_CERTIFICATE_LIMIT,
  OWNER_SIGNATURE_BYTES,
  ownerBase64,
  ownerCertificateInvalid as invalid,
  ownerStatementBytes,
} from './owner-certificate-binary.mjs';

const POLICY = 'internal_partner_binding_v1';
const I64_MAX = 9223372036854775807n;
const U64_MAX = 18446744073709551615n;

export function ownerObject(value, fields) {
  if (
    !value ||
    typeof value !== 'object' ||
    Array.isArray(value) ||
    Object.keys(value).length !== fields.length ||
    fields.some((name) => !Object.hasOwn(value, name))
  )
    invalid();
  return value;
}

export function ownerUuid(value) {
  if (
    typeof value !== 'string' ||
    !/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/.test(value) ||
    value === '00000000-0000-0000-0000-000000000000'
  )
    invalid();
  return value;
}

function decimal(value, maximum = I64_MAX) {
  if (
    typeof value !== 'string' ||
    value.length > 20 ||
    !/^(0|[1-9][0-9]*)$/.test(value) ||
    BigInt(value) > maximum
  )
    invalid();
  return BigInt(value);
}

function counters(value) {
  if (decimal(value.auth_generation) > decimal(value.account_revision)) invalid();
}

function text(value) {
  if (typeof value !== 'string') invalid();
}

function digest(value) {
  if (typeof value !== 'string' || !/^[a-f0-9]{64}$/.test(value)) invalid();
}

function integer(value, minimum, maximum) {
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) invalid();
}

function timestamp(value) {
  if (
    typeof value !== 'string' ||
    !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$/.test(value) ||
    !Number.isFinite(Date.parse(value))
  )
    invalid();
}

function interval(value, start, end) {
  integer(value[start], Number.MIN_SAFE_INTEGER, Number.MAX_SAFE_INTEGER);
  integer(value[end], Number.MIN_SAFE_INTEGER, Number.MAX_SAFE_INTEGER);
  if (value[start] > value[end]) invalid();
}

function certificate(value) {
  ownerObject(value, ['der_base64', 'fingerprint', 'summary']);
  ownerBase64(value.der_base64, 1, OWNER_CERTIFICATE_LIMIT);
  digest(value.fingerprint);
  const summary = ownerObject(value.summary, [
    'subject',
    'issuer',
    'serial_hex',
    'not_before_unix',
    'not_after_unix',
  ]);
  for (const name of ['subject', 'issuer', 'serial_hex']) text(summary[name]);
  interval(summary, 'not_before_unix', 'not_after_unix');
}

function trust(value) {
  ownerObject(value, [
    'deployment_id',
    'revision',
    'root_der_base64',
    'crl_der_base64',
    'root_fingerprint',
    'crl_digest',
    'crl_number',
    'crl_this_update_unix',
    'crl_next_update_unix',
    'valid_from_unix',
    'valid_until_unix',
    'published_at',
    'published_by',
  ]);
  ownerUuid(value.deployment_id);
  integer(value.revision, 1, 4294967295);
  ownerBase64(value.root_der_base64, 1, OWNER_CERTIFICATE_LIMIT);
  ownerBase64(value.crl_der_base64, 1, 1048576);
  digest(value.root_fingerprint);
  digest(value.crl_digest);
  decimal(value.crl_number, U64_MAX);
  interval(value, 'crl_this_update_unix', 'crl_next_update_unix');
  interval(value, 'valid_from_unix', 'valid_until_unix');
  timestamp(value.published_at);
  text(value.published_by);
}

function own(value, ownerId, bindingId) {
  ownerUuid(value.owner_id);
  ownerUuid(value.binding_id);
  if (
    value.policy !== POLICY ||
    (ownerId !== undefined && value.owner_id !== ownerId) ||
    (bindingId !== undefined && value.binding_id !== bindingId)
  )
    invalid();
}

export function ownerPreparation(value, ownerId, bindingId) {
  ownerObject(value, [
    'binding_id',
    'owner_id',
    'policy',
    'statement_base64',
    'account_revision',
    'auth_generation',
    'deployment_id',
    'trust_revision',
    'root_fingerprint',
    'certificate',
  ]);
  own(value, ownerId, bindingId);
  ownerStatementBytes(value.statement_base64);
  counters(value);
  ownerUuid(value.deployment_id);
  integer(value.trust_revision, 1, 4294967295);
  digest(value.root_fingerprint);
  certificate(value.certificate);
  return value;
}

export function ownerSubmission(value) {
  ownerObject(value, ['statement_base64', 'certificate_der_base64', 'signature_base64']);
  ownerStatementBytes(value.statement_base64);
  ownerBase64(value.certificate_der_base64, 1, OWNER_CERTIFICATE_LIMIT);
  ownerBase64(value.signature_base64, OWNER_SIGNATURE_BYTES);
  return Object.freeze({
    statement_base64: value.statement_base64,
    certificate_der_base64: value.certificate_der_base64,
    signature_base64: value.signature_base64,
  });
}

export function ownerReceipt(value, ownerId, bindingId, current = false) {
  ownerObject(value, [
    'binding_id',
    'owner_id',
    'revision',
    'policy',
    'registration',
    'withdrawal',
  ]);
  own(value, ownerId, bindingId);
  const registration = ownerObject(value.registration, [
    'statement_base64',
    'statement_digest',
    'certificate',
    'signature_base64',
    'account_revision',
    'auth_generation',
    'checked_at_unix',
    'valid_from_unix',
    'valid_until_unix',
    'registered_at',
    'trust',
  ]);
  ownerStatementBytes(registration.statement_base64);
  digest(registration.statement_digest);
  certificate(registration.certificate);
  ownerBase64(registration.signature_base64, OWNER_SIGNATURE_BYTES);
  counters(registration);
  interval(registration, 'valid_from_unix', 'valid_until_unix');
  integer(
    registration.checked_at_unix,
    registration.valid_from_unix,
    registration.valid_until_unix,
  );
  timestamp(registration.registered_at);
  trust(registration.trust);
  if (value.withdrawal === null) {
    if (value.revision !== 1) invalid();
  } else {
    if (current || value.revision !== 2) invalid();
    const withdrawal = ownerObject(value.withdrawal, [
      'statement_base64',
      'account_revision',
      'auth_generation',
      'withdrawn_at',
    ]);
    ownerStatementBytes(withdrawal.statement_base64, 2);
    counters(withdrawal);
    if (
      decimal(withdrawal.account_revision) < decimal(registration.account_revision) ||
      decimal(withdrawal.auth_generation) < decimal(registration.auth_generation)
    )
      invalid();
    timestamp(withdrawal.withdrawn_at);
  }
  return value;
}
