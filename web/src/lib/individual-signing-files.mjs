import { signingDigest } from './individual-signing-values.mjs';

export async function signingHash(bytes) {
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
  return Array.from(digest, (byte) => byte.toString(16).padStart(2, '0')).join('');
}

export async function checkedSigningPdf(blob, expectedDigest) {
  signingDigest(expectedDigest);
  if (!(blob instanceof Blob) || blob.size < 5 || blob.size > 16 * 1024 * 1024)
    throw new Error('El archivo no es un PDF admitido.');
  const bytes = new Uint8Array(await blob.arrayBuffer());
  if (
    new TextDecoder().decode(bytes.subarray(0, 5)) !== '%PDF-' ||
    (await signingHash(bytes)) !== expectedDigest
  )
    throw new Error('El PDF no coincide con el resumen de esta version.');
  return new Blob([bytes], { type: 'application/pdf' });
}
