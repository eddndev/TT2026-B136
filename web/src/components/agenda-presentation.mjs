import { defaultAgendaFilters } from '../lib/hearing-agenda.mjs';
import { hearingInstant } from '../lib/hearing-time.mjs';
import { agendaDays, agendaPeriod, agendaCivilDay } from '../lib/agenda-periods.mjs';
import {
  agendaKey,
  compareAgendaKeys,
  combinedAgendaQuery,
} from '../lib/combined-agenda-query.mjs';

export function initialAgendaSelection(saved) {
  const fallback = defaultAgendaFilters();
  return {
    view: saved?.view || (saved ? 'custom' : 'week'),
    date: saved?.date || saved?.from || fallback.from,
    from: saved?.from || fallback.from,
    until: saved?.until || fallback.until,
    offset: saved?.offset || fallback.offset,
    kind: saved?.kind || 'all',
    hearing_status: saved?.hearing_status || saved?.status || 'scheduled',
  };
}

export function agendaSelection(value) {
  const range =
    value.view === 'custom'
      ? { from: value.from, until: value.until }
      : agendaPeriod(value.view, value.date);
  agendaDays(range.from, range.until);
  const boundary = (date) => {
    const at = hearingInstant({ date, time: '00:00:00', offset: value.offset });
    return new Date(Date.parse(at)).toISOString().replace('.000Z', 'Z');
  };
  const filters = {
    ...value,
    ...range,
    hearing_status: value.kind === 'deadline' ? 'scheduled' : value.hearing_status,
  };
  const input = {
    from: boundary(range.from),
    until: boundary(range.until),
    kind: filters.kind,
    hearing_status: filters.hearing_status,
    limit: 20,
  };
  combinedAgendaQuery(input);
  return { filters, range, query: input };
}

export const agendaRecord = (item) => item[item.kind];
export const agendaIdentity = (item) => `${item.kind}:${agendaRecord(item).id}`;

export function mergeAgendaItems(previous, incoming) {
  const found = new Map(previous.map((item) => [agendaIdentity(item), item]));
  for (const item of incoming) {
    const key = agendaIdentity(item);
    const existing = found.get(key);
    if (!existing || agendaRecord(item).revision > agendaRecord(existing).revision)
      found.set(key, item);
  }
  return [...found.values()].sort((left, right) =>
    compareAgendaKeys(
      agendaKey(left.at, left.kind, agendaRecord(left).id),
      agendaKey(right.at, right.kind, agendaRecord(right).id),
    ),
  );
}

export function agendaGroups(rows, offset) {
  const groups = new Map();
  for (const item of rows) {
    const day = agendaCivilDay(item.at, offset);
    if (!groups.has(day)) groups.set(day, []);
    groups.get(day).push(item);
  }
  return groups;
}

export function agendaIntent(item) {
  const record = agendaRecord(item);
  return {
    kind: item.kind,
    case_id: record.case_id,
    revision: record.revision,
    [item.kind === 'hearing' ? 'hearing_id' : 'deadline_id']: record.id,
  };
}
