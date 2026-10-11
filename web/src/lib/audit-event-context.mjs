import { factObject, factUuid } from './procedural-fact-primitives.mjs';
import { auditInvalid } from './audit-event-query.mjs';

const sources = new Map([
  ['http', 'HTTP'],
  ['cli', 'CLI'],
  ['internal', 'Interno'],
]);
const outcomes = new Map([
  ['access.granted', 'Concedido'],
  ['access.denied', 'Denegado'],
  ['access.rejected', 'Rechazado'],
  ['access.failed', 'Fallo'],
]);

function ipv4(value) {
  const parts = value.split('.');
  return (
    parts.length === 4 &&
    parts.every((part) => /^(0|[1-9]\d{0,2})$/.test(part) && Number(part) <= 255)
  );
}

function ipAddress(value) {
  if (
    typeof value !== 'string' ||
    value.length < 2 ||
    value.length > 45 ||
    /[^a-f\d:.]/i.test(value)
  )
    auditInvalid();
  if (!value.includes(':')) {
    if (!ipv4(value)) auditInvalid();
    return;
  }
  let address = value;
  if (address.includes('.')) {
    const tail = address.lastIndexOf(':');
    if (!ipv4(address.slice(tail + 1))) auditInvalid();
    address = `${address.slice(0, tail + 1)}0:0`;
  }
  const halves = address.split('::');
  if (halves.length > 2) auditInvalid();
  const groups = halves.flatMap((half) => (half ? half.split(':') : []));
  if (
    groups.some((part) => !/^[a-f\d]{1,4}$/i.test(part)) ||
    (halves.length === 1 ? groups.length !== 8 : groups.length >= 8)
  )
    auditInvalid();
}

export function auditContext(value) {
  if (value === null) return null;
  factObject(value, ['user_id', 'ip_address', 'source', 'request_id']);
  for (const key of ['user_id', 'request_id']) {
    if (value[key] !== null) {
      factUuid(value[key]);
      if (value[key].length !== 36) auditInvalid();
    }
  }
  if (value.ip_address !== null) ipAddress(value.ip_address);
  if (!sources.has(value.source)) auditInvalid();
  return value;
}

export const auditSourceLabel = (context) => sources.get(context?.source) ?? 'No disponible';
export const auditOutcomeLabel = (event) =>
  (event.context && outcomes.get(event.action)) || 'No disponible';
