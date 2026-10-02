import { factObject } from './procedural-fact-primitives.mjs';
export const auditPageTextBytes = 262144;
export const auditSequenceMaximum = 9223372036854775807n;
const encoder = new TextEncoder();
export function auditInvalid() {
  throw new Error('No se pudo validar la actividad. Revisa los filtros y actualiza la consulta.');
}
export function auditBytes(value) {
  if (typeof value !== 'string' || /[\ud800-\udfff]/u.test(value)) auditInvalid();
  return encoder.encode(value).byteLength;
}
export function auditSequence(value) {
  if (typeof value !== 'string' || !/^(0|[1-9]\d{0,18})$/.test(value)) auditInvalid();
  const exact = BigInt(value);
  if (exact > auditSequenceMaximum) auditInvalid();
  return exact;
}
export function auditInstant(value) {
  const match =
    typeof value === 'string' &&
    /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.(\d{1,9}))?(Z|[+-]\d{2}:\d{2})$/.exec(value);
  if (!match || value.startsWith('0000')) auditInvalid();
  const base = Date.parse(`${match[1]}Z`);
  if (!Number.isFinite(base) || new Date(base).toISOString().slice(0, 19) !== match[1])
    auditInvalid();
  let offset = 0;
  if (match[3] !== 'Z') {
    const hours = Number(match[3].slice(1, 3)),
      minutes = Number(match[3].slice(4));
    if (hours > 23 || minutes > 59) auditInvalid();
    offset = (hours * 60 + minutes) * 60000 * (match[3][0] === '-' ? -1 : 1);
  }
  const milliseconds = base - offset,
    year = new Date(milliseconds).getUTCFullYear();
  if (year < 1 || year > 9999) auditInvalid();
  return BigInt(milliseconds / 1000) * 1000000000n + BigInt((match[2] || '').padEnd(9, '0'));
}
export function auditCursor(value) {
  if (
    typeof value !== 'string' ||
    !value.length ||
    value.length > 4096 ||
    /[^\x21-\x7e]/.test(value)
  )
    auditInvalid();
  return value;
}
export function auditQuery(raw) {
  factObject(
    raw,
    ['from', 'until', 'actor', 'action', 'resource', 'limit', 'cursor'],
    ['from', 'until'],
  );
  const from = auditInstant(raw.from),
    until = auditInstant(raw.until),
    limit = raw.limit === undefined ? 20 : raw.limit;
  if (
    from >= until ||
    until - from > 366n * 86400n * 1000000000n ||
    !Number.isInteger(limit) ||
    limit < 1 ||
    limit > 100
  )
    auditInvalid();
  const value = {
    from: raw.from,
    until: raw.until,
    limit,
    cursor: raw.cursor == null ? null : auditCursor(raw.cursor),
  };
  for (const [key, max] of [
    ['actor', 254],
    ['action', 128],
    ['resource', 1024],
  ]) {
    value[key] = raw[key] == null ? null : raw[key];
    if (
      value[key] !== null &&
      (!value[key].length ||
        auditBytes(value[key]) > max ||
        /[\u0000-\u001f\u007f-\u009f]/.test(value[key]))
    )
      auditInvalid();
  }
  return value;
}
export function auditFingerprint(query) {
  return JSON.stringify([
    auditInstant(query.from).toString(),
    auditInstant(query.until).toString(),
    query.actor,
    query.action,
    query.resource,
  ]);
}
