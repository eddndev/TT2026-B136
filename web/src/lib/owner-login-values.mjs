import { ownerObject, ownerReceipt, ownerUuid } from './owner-certificate-values.mjs';
import { ownerBase64 } from './owner-certificate-binary.mjs';

const RECEIPT_BYTES =
  4 * Math.ceil(1048576 / 3) + 8 * Math.ceil(16384 / 3) + 6 * (2 * 65536 + 1024) + 4096;
const I64_MAX = 9223372036854775807n;
const invalid = () => {
  throw new Error(
    'El material de acceso con certificado no es valido. Selecciona el recibo publico o prepara un nuevo intento.',
  );
};
const hex = (bytes) => Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
function uuid(bytes) {
  const value = hex(bytes);
  return ownerUuid(value.replace(/^(........)(....)(....)(....)(............)$/, '$1-$2-$3-$4-$5'));
}
function seconds(value) {
  if (!Number.isSafeInteger(value) || value < 1 || value > 300) invalid();
  return value;
}
function token(value) {
  if (typeof value !== 'string' || !/^[A-Za-z0-9_-]{43}$/.test(value)) invalid();
  ownerBase64(value.replaceAll('-', '+').replaceAll('_', '/') + '=', 32);
  return value;
}
function boundedText(value, limit) {
  if (typeof value !== 'string' || new TextEncoder().encode(value).length > limit) invalid();
  return value;
}

export function ownerLoginSelection(value) {
  ownerObject(value, ['ownerId', 'bindingId', 'leafFingerprint', 'subject']);
  ownerUuid(value.ownerId);
  ownerUuid(value.bindingId);
  if (typeof value.leafFingerprint !== 'string' || !/^[0-9a-f]{64}$/.test(value.leafFingerprint))
    invalid();
  boundedText(value.subject, 65536);
  return Object.freeze({ ...value });
}

export async function readOwnerLoginReceipt(file) {
  try {
    const size = file?.size;
    if (
      !Number.isSafeInteger(size) ||
      size < 1 ||
      size > RECEIPT_BYTES ||
      typeof file.arrayBuffer !== 'function'
    )
      invalid();
    const buffer = await file.arrayBuffer();
    if (!(buffer instanceof ArrayBuffer) || buffer.byteLength !== size) invalid();
    const parsed = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(buffer));
    const receipt = ownerReceipt(parsed, undefined, undefined, true);
    const certificate = receipt.registration.certificate;
    boundedText(certificate.summary.issuer, 65536);
    boundedText(certificate.summary.serial_hex, 40);
    boundedText(receipt.registration.trust.published_by, 1024);
    return ownerLoginSelection({
      ownerId: receipt.owner_id,
      bindingId: receipt.binding_id,
      leafFingerprint: certificate.fingerprint,
      subject: certificate.summary.subject,
    });
  } catch {
    invalid();
  }
}

export function ownerLoginChallenge(value, selected) {
  const selection = ownerLoginSelection(selected);
  ownerObject(value, ['challenge_token', 'statement_base64', 'expires_in_seconds']);
  token(value.challenge_token);
  const remaining = seconds(value.expires_in_seconds);
  const bytes = ownerBase64(value.statement_base64, 182);
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const issued = view.getBigInt64(166),
    expires = view.getBigInt64(174);
  if (
    new TextDecoder().decode(bytes.subarray(0, 8)) !== 'OWNAUTH1' ||
    bytes[8] !== 1 ||
    bytes[9] !== 1 ||
    view.getUint32(58) < 1 ||
    uuid(bytes.subarray(62, 78)) !== selection.ownerId ||
    view.getBigUint64(78) > I64_MAX ||
    uuid(bytes.subarray(86, 102)) !== selection.bindingId ||
    hex(bytes.subarray(102, 134)) !== selection.leafFingerprint ||
    issued < 0n ||
    expires <= issued ||
    expires - issued > 300n ||
    BigInt(remaining) > expires - issued
  )
    invalid();
  uuid(bytes.subarray(10, 26));
  return Object.freeze({ ...value });
}

export function ownerLoginStatementDownload(value, selection) {
  const checked = ownerLoginChallenge(value, selection);
  return new Blob([ownerBase64(checked.statement_base64, 182)], {
    type: 'application/octet-stream',
  });
}

export function ownerLoginDeadline(value, { startedAt, receivedAt }) {
  const deadline = startedAt + seconds(value.expires_in_seconds) * 1000;
  if (
    !Number.isFinite(startedAt) ||
    !Number.isFinite(receivedAt) ||
    !Number.isFinite(deadline) ||
    startedAt < 0 ||
    receivedAt < startedAt ||
    receivedAt >= deadline
  )
    invalid();
  return deadline;
}

export function ownerLoginAvailability(value) {
  ownerObject(value, ['enabled']);
  if (typeof value.enabled !== 'boolean') invalid();
  return value.enabled;
}

export function ownerLoginMfa(value) {
  ownerObject(value, ['challenge_token', 'expires_in_seconds']);
  token(value.challenge_token);
  seconds(value.expires_in_seconds);
  return Object.freeze({ ...value });
}
