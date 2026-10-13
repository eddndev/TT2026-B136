export const OWNER_CERTIFICATE_LIMIT = 16384;
export const OWNER_SIGNATURE_BYTES = 384;
export const OWNER_STATEMENT_BYTES = 150;

export function ownerCertificateInvalid() {
  throw new Error('La evidencia del certificado no corresponde a la operacion solicitada.');
}

export function ownerBase64(value, minimum, maximum = minimum) {
  if (
    typeof value !== 'string' ||
    value.length > Math.ceil(maximum / 3) * 4 ||
    value.length % 4 !== 0 ||
    !/^[A-Za-z0-9+/]*={0,2}$/.test(value)
  )
    ownerCertificateInvalid();
  let binary;
  try {
    binary = atob(value);
  } catch {
    ownerCertificateInvalid();
  }
  if (binary.length < minimum || binary.length > maximum || btoa(binary) !== value)
    ownerCertificateInvalid();
  return Uint8Array.from(binary, (character) => character.charCodeAt(0));
}

export function ownerStatementBytes(value, purpose = 1) {
  const bytes = ownerBase64(value, OWNER_STATEMENT_BYTES);
  if (
    new TextDecoder().decode(bytes.subarray(0, 8)) !== 'OWNCERT1' ||
    bytes[8] !== purpose ||
    bytes[9] !== 1
  )
    ownerCertificateInvalid();
  return bytes;
}

export function ownerPublicPem(bytes) {
  if (
    !(bytes instanceof Uint8Array) ||
    bytes.length < 1 ||
    bytes.length > OWNER_CERTIFICATE_LIMIT ||
    bytes.some((byte) => byte > 127)
  )
    ownerCertificateInvalid();
  const match =
    /^[\t\n\v\f\r ]*-----BEGIN CERTIFICATE-----([A-Za-z0-9+/=\t\n\v\f\r ]+)-----END CERTIFICATE-----[\t\n\v\f\r ]*$/.exec(
      new TextDecoder().decode(bytes),
    );
  if (!match) ownerCertificateInvalid();
  ownerBase64(match[1].replace(/[\t\n\v\f\r ]/g, ''), 1, OWNER_CERTIFICATE_LIMIT);
  return bytes;
}
