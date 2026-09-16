export const CERTIFICATE_BYTE_LIMIT = 16 * 1024;
export const DETACHED_SIGNATURE_BYTES = 384;
export const DECLARATION_BYTES = 218;
const invalidCertificate = () =>
  new Error('Selecciona un certificado p\u00fablico PEM o DER, sin claves ni material adicional.');

async function readBounded(file, validSize, sizeMessage) {
  if (!file || !Number.isSafeInteger(file.size) || !validSize(file.size))
    throw new Error(sizeMessage);
  let buffer;
  try {
    buffer = await file.arrayBuffer();
  } catch {
    throw new Error('No se pudo leer el archivo seleccionado. Vuelve a seleccionarlo.');
  }
  if (!(buffer instanceof ArrayBuffer) || !validSize(buffer.byteLength))
    throw new Error(sizeMessage);
  return new Uint8Array(buffer);
}

function element(bytes, offset, tag) {
  if (bytes[offset] !== tag || offset + 1 >= bytes.length) throw invalidCertificate();
  let length = bytes[offset + 1];
  let body = offset + 2;
  if (length >= 128) {
    const count = length & 127;
    if (!count || count > 4 || body + count > bytes.length || bytes[body] === 0)
      throw invalidCertificate();
    length = 0;
    for (let index = 0; index < count; index++) length = length * 256 + bytes[body++];
    if (length < 128) throw invalidCertificate();
  }
  const end = body + length;
  if (end > bytes.length) throw invalidCertificate();
  return { body, end };
}

function certificateEnvelope(der) {
  // This envelope gate excludes key/CSR containers; X.509 admission belongs to the server.
  const outer = element(der, 0, 0x30);
  if (outer.end !== der.length) throw invalidCertificate();
  const tbs = element(der, outer.body, 0x30);
  const algorithm = element(der, tbs.end, 0x30);
  const signature = element(der, algorithm.end, 0x03);
  if (signature.end !== outer.end) throw invalidCertificate();
  const version = element(der, tbs.body, 0xa0);
  if (
    version.end - version.body !== 3 ||
    der[version.body] !== 2 ||
    der[version.body + 1] !== 1 ||
    der[version.body + 2] !== 2
  )
    throw invalidCertificate();
  let offset = element(der, version.end, 2).end;
  for (let index = 0; index < 5; index++) offset = element(der, offset, 0x30).end;
  if (offset > tbs.end) throw invalidCertificate();
}

function publicDer(bytes) {
  if (bytes[0] === 0x30) return bytes;
  if (bytes.some((value) => value > 127)) throw invalidCertificate();
  const text = new TextDecoder().decode(bytes);
  const match =
    /^[\t\n\v\f\r ]*-----BEGIN CERTIFICATE-----([A-Za-z0-9+/=\t\n\v\f\r ]+)-----END CERTIFICATE-----[\t\n\v\f\r ]*$/.exec(
      text,
    );
  if (!match) throw invalidCertificate();
  const encoded = match[1].replace(/[\t\n\v\f\r ]/g, '');
  if (!/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(encoded))
    throw invalidCertificate();
  const binary = atob(encoded);
  if (btoa(binary) !== encoded) throw invalidCertificate();
  return Uint8Array.from(binary, (character) => character.charCodeAt(0));
}

export async function readPublicCertificate(file) {
  const bytes = await readBounded(
    file,
    (size) => size > 0 && size <= CERTIFICATE_BYTE_LIMIT,
    'Selecciona un certificado p\u00fablico de hasta 16 KiB.',
  );
  certificateEnvelope(publicDer(bytes));
  return new Blob([bytes], { type: 'application/octet-stream' });
}

export async function readDetachedSignature(file) {
  const bytes = await readBounded(
    file,
    (size) => size === DETACHED_SIGNATURE_BYTES,
    'Selecciona una firma binaria de exactamente 384 bytes.',
  );
  return new Blob([bytes], { type: 'application/octet-stream' });
}

function copyBytes(value) {
  if (value instanceof Uint8Array) return value.slice();
  if (value instanceof ArrayBuffer) return new Uint8Array(value.slice(0));
  throw new Error('No se recibieron los bytes de la declaraci\u00f3n y su recibo.');
}

export function declarationDownloads(statementBytes, receiptBytes) {
  const statement = copyBytes(statementBytes);
  const receipt = copyBytes(receiptBytes);
  if (
    statement.length !== DECLARATION_BYTES ||
    new TextDecoder().decode(statement.subarray(0, 6)) !== 'PCRED1' ||
    statement[6] !== 0 ||
    statement[7] !== 0 ||
    !receipt.length
  )
    throw new Error('La declaraci\u00f3n recibida no corresponde al formato admitido.');
  return Object.freeze({
    statement: new Blob([statement], { type: 'application/octet-stream' }),
    receipt: new Blob([receipt], { type: 'application/json' }),
  });
}
