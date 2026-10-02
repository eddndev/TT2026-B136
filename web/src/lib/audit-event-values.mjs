import { factObject } from './procedural-fact-primitives.mjs';
import {
  auditBytes,
  auditCursor,
  auditInstant,
  auditInvalid,
  auditPageTextBytes,
  auditSequence,
} from './audit-event-query.mjs';

export function auditPosition(event) {
  return [auditInstant(event.timestamp), auditSequence(event.sequence)];
}
export function auditAfter(next, prior) {
  return next[0] > prior[0] || (next[0] === prior[0] && next[1] > prior[1]);
}
export function auditPage(value, query, continuation = null) {
  factObject(value, ['checked_at', 'snapshot_max_sequence', 'events', 'has_more', 'next_cursor']);
  auditInstant(value.checked_at);
  const maximum =
    value.snapshot_max_sequence === null ? null : auditSequence(value.snapshot_max_sequence);
  if (
    !Array.isArray(value.events) ||
    value.events.length > query.limit ||
    typeof value.has_more !== 'boolean' ||
    (value.has_more && (value.events.length !== query.limit || value.next_cursor === null)) ||
    (!value.has_more && value.next_cursor !== null) ||
    (maximum === null && (value.events.length || value.has_more)) ||
    (continuation && continuation.maximum !== value.snapshot_max_sequence)
  )
    auditInvalid();
  if (value.next_cursor !== null) {
    auditCursor(value.next_cursor);
    if (value.next_cursor === query.cursor) auditInvalid();
  }
  const from = auditInstant(query.from),
    until = auditInstant(query.until),
    seen = new Set();
  let prior = continuation?.last || null,
    bytes = 0;
  for (const event of value.events) {
    factObject(event, ['sequence', 'timestamp', 'actor', 'action', 'resource']);
    const position = auditPosition(event);
    if (
      maximum === null ||
      position[1] > maximum ||
      position[0] < from ||
      position[0] >= until ||
      (prior && !auditAfter(position, prior)) ||
      seen.has(event.sequence)
    )
      auditInvalid();
    for (const key of ['actor', 'action', 'resource']) {
      bytes += auditBytes(event[key]);
      if (query[key] !== null && event[key] !== query[key]) auditInvalid();
    }
    if (bytes > auditPageTextBytes) auditInvalid();
    seen.add(event.sequence);
    prior = position;
  }
  return value;
}
