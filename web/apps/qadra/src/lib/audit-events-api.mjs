import { auditCursor, auditFingerprint, auditInvalid, auditQuery } from './audit-event-query.mjs';
import { auditPage, auditPosition } from './audit-event-values.mjs';

export function auditEventsApi(request) {
  let active = true,
    generation = 0,
    continuation = null;
  const check = (current = generation) => {
    if (!active || current !== generation)
      throw new Error('La consulta de actividad ya no est\u00e1 abierta.');
  };
  return {
    dispose() {
      active = false;
      generation++;
      continuation = null;
    },
    async list(raw) {
      check();
      const query = auditQuery(raw),
        fingerprint = auditFingerprint(query),
        current = ++generation;
      const prior = query.cursor === null ? null : continuation;
      if (
        query.cursor !== null &&
        (!prior || prior.cursor !== query.cursor || prior.fingerprint !== fingerprint)
      )
        auditInvalid();
      continuation = null;
      const params = new URLSearchParams({
        from: query.from,
        until: query.until,
        limit: String(query.limit),
      });
      for (const key of ['actor', 'action', 'resource', 'cursor'])
        if (query[key] !== null) params.set(key, query[key]);
      try {
        const value = await request(`/audit/events?${params}`);
        check(current);
        auditPage(value, query, prior);
        if (value.has_more)
          continuation = {
            cursor: auditCursor(value.next_cursor),
            fingerprint,
            maximum: value.snapshot_max_sequence,
            last: auditPosition(value.events.at(-1)),
          };
        return value;
      } catch (error) {
        check(current);
        throw error;
      }
    },
  };
}
