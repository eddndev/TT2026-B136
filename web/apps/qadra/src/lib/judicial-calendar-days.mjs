// Projections of the classified days of one judicial calendar revision onto
// the shared month calendar (MonthCalendar) and dated list (UpcomingList).
// A day row is { date, state, origin, source_ids, explanation } as returned
// by the days endpoint.
import { civilLabel } from './judicial-calendar-time.mjs';
import { calendarStates } from './judicial-calendar-labels.mjs';

const symbols = { countable: 'C', excluded: 'E', unresolved: '?', outside_coverage: '--' };
// The API state names the tone keys of month-grid toneClasses.
const tones = {
  countable: 'countable',
  excluded: 'excluded',
  unresolved: 'unresolved',
  outside_coverage: 'outside',
};

// Every state with its marker symbol, label and tint, in legend order.
export const calendarLegend = Object.entries(calendarStates).map(([state, label]) => ({
  state,
  symbol: symbols[state],
  label,
  tone: tones[state],
}));

const dayLabel = (day) => `${civilLabel(day.date)}: ${calendarStates[day.state]}`;

// describe(date) for MonthGrid. A date without a loaded row (while a month
// is loading or after its query failed) is named only by its date, carries
// no state and is disabled, since choosing it would show nothing.
export function calendarDayDescriber(days) {
  const byDate = new Map(days.map((day) => [day.date, day]));
  return (date) => {
    const day = byDate.get(date);
    if (!day) return { label: civilLabel(date), attrs: { disabled: true } };
    return {
      label: dayLabel(day),
      tone: tones[day.state],
      marker: { text: symbols[day.state] },
      attrs: { 'data-state': day.state },
    };
  };
}

// Where the classification of a day comes from.
export function calendarDayOrigin(day) {
  if (day.state === 'outside_coverage')
    return 'Esta revisi\u00f3n no cubre la fecha; no se aplica una regla por defecto.';
  return day.origin === 'exception' ? 'Excepci\u00f3n declarada' : 'Patr\u00f3n semanal declarado';
}

// One UpcomingList row. The classification covers the whole civil day.
export function calendarDayRow(day, selected) {
  return {
    key: day.date,
    value: day,
    label: dayLabel(day),
    pressed: day.date === selected,
    attrs: { 'data-state': day.state },
    date: day.date,
    dateText: civilLabel(day.date),
    badge: symbols[day.state],
    title: calendarStates[day.state],
    meta: [calendarDayOrigin(day)],
    timeText: 'Todo el d\u00eda',
  };
}

// The first and last covered months, which bound the month navigation.
export function calendarCoverageMonths(coverage) {
  return { min: coverage.from.slice(0, 7), max: coverage.through.slice(0, 7) };
}
