import { safeFilename, validId } from './documents.mjs';

const digestPattern = /^[0-9a-f]{64}$/;

export function contentReference(caseId, id, version, digest) {
  if (
    !validId(caseId) ||
    !validId(id) ||
    !Number.isInteger(version) ||
    version < 1 ||
    version > 4294967295 ||
    typeof digest !== 'string' ||
    !digestPattern.test(digest)
  )
    throw new Error(
      'La versi\u00f3n seleccionada no conserva una referencia de contenido v\u00e1lida.',
    );
}

export function contentValue(value, caseId, id, version, digest) {
  if (
    value.caseId !== caseId ||
    value.documentId !== id ||
    value.version !== String(version) ||
    typeof value.digest !== 'string' ||
    !digestPattern.test(value.digest) ||
    value.digest !== digest ||
    value.contentType !== 'application/octet-stream' ||
    value.blob?.type !== 'application/octet-stream'
  )
    throw new Error('La respuesta no corresponde al contenido de la versi\u00f3n seleccionada.');
  return value;
}

export function contentFilename(name, version) {
  const safe = safeFilename(name),
    dot = safe.lastIndexOf('.');
  const extension = dot > 0 && /^\.[A-Za-z0-9]{1,19}$/.test(safe.slice(dot)) ? safe.slice(dot) : '';
  const stem = extension ? safe.slice(0, dot) : safe;
  const suffix = `-v${version}`;
  return `${stem.slice(0, 124 - suffix.length - extension.length)}${suffix}${extension}`;
}
