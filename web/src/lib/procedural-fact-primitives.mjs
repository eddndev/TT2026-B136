import { caseText } from './case-administration.mjs';
export const factMaxRevision = 4294967295;
export function factInvalid(message = 'Revisa los valores y las referencias exactas declaradas.') {
  throw new Error(message);
}
export function factObject(value, keys, required = keys) {
  if (
    !value ||
    typeof value !== 'object' ||
    Array.isArray(value) ||
    Object.keys(value).some((key) => !keys.includes(key)) ||
    required.some((key) => !Object.hasOwn(value, key))
  )
    factInvalid();
  return value;
}
export function factUuid(value) {
  if (typeof value !== 'string' || !/^[a-f\d]{8}(-[a-f\d]{4}){3}-[a-f\d]{12}$/i.test(value))
    factInvalid('Consulta un identificador exacto v\u00e1lido.');
  return value.toLowerCase();
}
export function factRevision(value, maximum = factMaxRevision) {
  if (!Number.isInteger(value) || value < 1 || value > maximum)
    factInvalid('Consulta una revisi\u00f3n v\u00e1lida.');
  return value;
}
export function factDigest(value) {
  if (typeof value !== 'string' || !/^[a-f\d]{64}$/i.test(value))
    factInvalid('Consulta una huella exacta v\u00e1lida.');
  return value.toLowerCase();
}
export function factText(value, label = 'Texto declarado', limit = 1000, multiline = true) {
  if (typeof value !== 'string' || /[\ud800-\udfff]/u.test(value))
    factInvalid(`${label}: revisa los caracteres.`);
  return caseText(value, { key: label, label, limit, required: true, multiline });
}
export const factLabel = (value) => factText(value, 'Descripci\u00f3n declarada', 200, false);
export const factOptional = (value, parse) => (value == null ? null : parse(value));
export function factSame(a, b) {
  if (a === b) return true;
  if (
    !a ||
    !b ||
    typeof a !== 'object' ||
    typeof b !== 'object' ||
    Array.isArray(a) !== Array.isArray(b)
  )
    return false;
  const keys = Object.keys(a),
    other = Object.keys(b);
  return (
    keys.length === other.length &&
    keys.every((key) => Object.hasOwn(b, key) && factSame(a[key], b[key]))
  );
}
