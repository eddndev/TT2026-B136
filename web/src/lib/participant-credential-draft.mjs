import {
  readPublicCertificate,
  readDetachedSignature,
  declarationDownloads,
} from './participant-credential-files.mjs';

const invalid = () => new Error('No se pudo recuperar el material p\u00fablico conservado.');
const blob = (value) => value instanceof Blob && Object.getPrototypeOf(value) === Blob.prototype;
const named = (value) => {
  if (!value || typeof value.name !== 'string' || !blob(value.blob)) throw invalid();
  return { name: value.name, blob: value.blob };
};

export function captureCredentialDraft(certificate, prepared, signature, retained) {
  const previous =
    prepared || signature
      ? { signature, statement: prepared?.statement ?? null, receipt: prepared?.receipt ?? null }
      : retained;
  if (!certificate && !previous) return null;
  return {
    certificate: certificate ? named(certificate) : null,
    retained: previous
      ? {
          signature: previous.signature ? named(previous.signature) : null,
          statement: previous.statement ?? null,
          receipt: previous.receipt ?? null,
        }
      : null,
  };
}

export async function readCredentialDraft(value, canApply) {
  if (!canApply()) return null;
  if (value === null) return { certificate: null, retained: null };
  if (!value || typeof value !== 'object') throw invalid();
  let certificate = null,
    signature = null,
    statement = null,
    receipt = null;
  if (value.certificate !== null) {
    const source = named(value.certificate);
    const result = await readPublicCertificate(source.blob);
    if (!canApply()) return null;
    certificate = Object.freeze({ name: source.name, blob: result });
  }
  if (value.retained !== null) {
    const previous = value.retained;
    if (!previous || typeof previous !== 'object') throw invalid();
    if (previous.signature !== null) {
      const source = named(previous.signature);
      const result = await readDetachedSignature(source.blob);
      if (!canApply()) return null;
      signature = Object.freeze({ name: source.name, blob: result });
    }
    if (previous.statement !== null || previous.receipt !== null) {
      if (!blob(previous.statement) || !blob(previous.receipt)) throw invalid();
      const statementBytes = await previous.statement.arrayBuffer();
      if (!canApply()) return null;
      const receiptBytes = await previous.receipt.arrayBuffer();
      if (!canApply()) return null;
      ({ statement, receipt } = declarationDownloads(statementBytes, receiptBytes));
    }
    if (signature && !statement) throw invalid();
  }
  return {
    certificate,
    retained:
      signature || statement || receipt ? Object.freeze({ signature, statement, receipt }) : null,
  };
}
