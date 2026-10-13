import { hearingInstant } from './hearing-time.mjs';
export function defaultAgendaFilters(now = Date.now()) {
  const from = new Date(now).toISOString().slice(0, 10);
  const until = new Date(Date.parse(`${from}T00:00:00Z`) + 7 * 86400000).toISOString().slice(0, 10);
  return { from, until, offset: '+00:00', status: 'scheduled' };
}
export function agendaQuery(filters) {
  if (!['scheduled', 'cancelled', 'all'].includes(filters.status))
    throw new Error('Selecciona un estado de audiencia.');
  const boundary = (date) => hearingInstant({ date, time: '00:00:00', offset: filters.offset });
  const from = Date.parse(boundary(filters.from)),
    until = Date.parse(boundary(filters.until));
  if (until <= from || until - from > 366 * 86400000)
    throw new Error('El rango debe ser positivo y no superar 366 d\u00edas.');
  const utc = (millis) => new Date(millis).toISOString().replace('.000Z', 'Z');
  return { from: utc(from), until: utc(until), status: filters.status };
}
