// The coming agenda of the overview: the next thirty local days of the
// combined Agenda (docs/agenda-api.md), shown as a short list and one month.
// It reuses the Agenda query, rows and day grouping so both screens read the
// same activities the same way.
import { civilLabel } from '../lib/judicial-calendar-time.mjs';
import { agendaRow } from './agenda-rows.mjs';
import { agendaGroups, agendaRecord, agendaSelection } from './agenda-presentation.mjs';

export const comingDays = 30;
export const comingRows = 6;
const pad = (value) => String(value).padStart(2, '0');

// The explicit UTC offset of the browser at now, as the Agenda query writes
// it. A zero offset is '+00:00' because '-00:00' is not accepted.
export function localOffset(now = new Date()) {
  const minutes = -now.getTimezoneOffset();
  const sign = minutes < 0 ? '-' : '+';
  const absolute = Math.abs(minutes);
  return `${sign}${pad(Math.floor(absolute / 60))}:${pad(absolute % 60)}`;
}

// today is the local civil date ('YYYY-MM-DD'). The query keeps the page size
// of the Agenda; when the period holds more, the overview says so.
export function dashboardAgendaSelection(today, offset, days = comingDays) {
  const until = new Date(Date.parse(`${today}T00:00:00Z`) + days * 86400000)
    .toISOString()
    .slice(0, 10);
  const selection = agendaSelection({
    view: 'custom',
    from: today,
    until,
    date: today,
    offset,
    kind: 'all',
    hearing_status: 'scheduled',
  });
  return { range: selection.range, query: selection.query };
}

// UpcomingList rows: the Agenda row with its case reference (the list shows
// it in mono), the case title as the only detail and the local hour. Ordinary hearings carry their case fields on the record.
export function dashboardAgendaRows(items, offset, max = comingRows) {
  return items.slice(0, max).map((item) => {
    const row = agendaRow(item, offset);
    const context = item.kind === 'hearing' ? agendaRecord(item) : item;
    const hour = row.time.datetime.slice(11, 16);
    return {
      ...row,
      label: `${row.badge}: ${row.title}, ${context.case_reference}, ${row.dateText} ${hour}`,
      meta: [context.case_title],
      time: { datetime: row.time.datetime, text: hour },
    };
  });
}

// describe(date) for MonthCalendar: days with activities are named with
// their count and marked with the navy dot; every other day keeps its name.
export function dashboardAgendaDay(items, offset) {
  const groups = agendaGroups(items, offset);
  return (date) => {
    const total = groups.get(date)?.length ?? 0;
    if (!total) return { label: civilLabel(date) };
    return {
      label: `${civilLabel(date)}: ${total} ${total === 1 ? 'actividad' : 'actividades'}`,
      marker: { dot: true },
      attrs: { 'data-agenda-day': date },
    };
  };
}

// Admits a page only while its request is the latest one, the signed-in
// account is the one that asked and the session still admits responses.
// load resolves { page }, { error } or null for a response that must not
// render; stop disposes the client and refuses every later response.
export function dashboardAgendaLoader({ scoped, session }) {
  let generation = 0,
    alive = true;
  const actor = () => {
    const user = session?.principal();
    return user ? `${user.id}:${user.email}:${user.role}` : null;
  };
  return {
    async load(query) {
      if (!alive) return null;
      const request = ++generation,
        asked = actor();
      const current = () =>
        alive && request === generation && (!session || session.canAdmit()) && actor() === asked;
      try {
        const page = await scoped.list(query);
        return current() ? { page } : null;
      } catch (failure) {
        return current() ? { error: failure.message } : null;
      }
    },
    stop() {
      alive = false;
      generation++;
      scoped.dispose();
    },
  };
}
