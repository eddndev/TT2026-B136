import {
  object,
  alertInvalid as invalid,
  alertUuid as uuid,
  alertInstant as instant,
} from './alerts-primitives.mjs';

export function alertCursor(value, query) {
  if (typeof value !== 'string' || value.length > 128 || !/^[\x21-\x7e]+$/.test(value)) invalid();
  const parts = value.split(':');
  if (
    parts.length !== 6 ||
    parts[0] !== 'a1' ||
    parts[4] !== query.read ||
    parts[5] !== query.state ||
    !/^\d{9}$/.test(parts[2])
  )
    invalid();
  const seconds = Number(parts[1]),
    nanos = Number(parts[2]);
  if (!Number.isSafeInteger(seconds) || String(seconds) !== parts[1]) invalid();
  instant({ unix_seconds: seconds, nanosecond: nanos, offset_seconds: 0 });
  uuid(parts[3]);
  return [seconds, nanos, parts[3]];
}
export function alertsQuery(input = {}) {
  object(input, ['limit', 'read', 'state', 'cursor'], []);
  const { limit = 20, read = 'all', state = 'active', cursor } = input;
  if (
    !Number.isInteger(limit) ||
    limit < 1 ||
    limit > 100 ||
    !['all', 'unread'].includes(read) ||
    !['active', 'all'].includes(state)
  )
    invalid();
  const query = { limit, read, state };
  const parameters = new URLSearchParams({ limit, read, state });
  query.after = cursor === undefined ? null : alertCursor(cursor, query);
  if (cursor !== undefined) parameters.set('cursor', cursor);
  return { query, parameters };
}
