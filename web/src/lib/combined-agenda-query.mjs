import { hearingTimeParts } from './hearing-time.mjs';
import { factUuid as uuid } from './procedural-fact-primitives.mjs';

export function agendaInvalid() {
  throw new Error('La consulta de agenda no conserva un rango, orden o seguimiento valido.');
}
export function utcSeconds(value) {
  const parts = hearingTimeParts(value);
  if (parts.offset !== '+00:00') agendaInvalid();
  return Date.parse(value) / 1000;
}
export function agendaKey(at, kind, id) {
  if (!['hearing', 'deadline'].includes(kind)) agendaInvalid();
  uuid(id);
  return [at.unix_seconds, at.nanosecond, kind === 'hearing' ? 0 : 1, id];
}
export function compareAgendaKeys(a, b) {
  for (let i = 0; i < a.length; i++) {
    if (a[i] < b[i]) return -1;
    if (a[i] > b[i]) return 1;
  }
  return 0;
}
export function agendaCursor(token, query) {
  if (typeof token !== 'string' || token.length > 512 || !/^[\x21-\x7e]+$/.test(token))
    agendaInvalid();
  const parts = token.split(':');
  const prefix = ['a1', String(query.start), String(query.end), query.kind, query.hearing_status];
  if (parts.length !== 9 || prefix.some((value, index) => parts[index] !== value)) agendaInvalid();
  const number = (value) => {
    const n = Number(value);
    if (!Number.isSafeInteger(n) || String(n) !== value) agendaInvalid();
    return n;
  };
  const seconds = number(parts[5]),
    nanos = number(parts[6]),
    rank = number(parts[7]);
  if (
    seconds < query.start ||
    seconds >= query.end ||
    nanos < 0 ||
    nanos > 999999999 ||
    ![0, 1].includes(rank)
  )
    agendaInvalid();
  if (query.kind !== 'all' && rank !== (query.kind === 'hearing' ? 0 : 1)) agendaInvalid();
  uuid(parts[8]);
  return [seconds, nanos, rank, parts[8]];
}
export function combinedAgendaQuery({
  from,
  until,
  kind = 'all',
  hearing_status = 'scheduled',
  limit = 20,
  cursor,
} = {}) {
  const start = utcSeconds(from),
    end = utcSeconds(until);
  if (
    end <= start ||
    end - start > 366 * 86400 ||
    !Number.isInteger(limit) ||
    limit < 1 ||
    limit > 100 ||
    !['all', 'hearing', 'deadline'].includes(kind) ||
    !['scheduled', 'cancelled', 'all'].includes(hearing_status) ||
    (kind === 'deadline' && hearing_status !== 'scheduled')
  )
    agendaInvalid();
  const normalized = (seconds) => new Date(seconds * 1000).toISOString().replace('.000Z', 'Z');
  const query = {
    from: normalized(start),
    until: normalized(end),
    kind,
    hearing_status,
    limit,
    start,
    end,
  };
  const parameters = new URLSearchParams({
    from: query.from,
    until: query.until,
    kind,
    hearing_status,
    limit,
  });
  query.after = cursor === undefined ? null : agendaCursor(cursor, query);
  if (cursor !== undefined) parameters.set('cursor', cursor);
  return { query, parameters };
}
