// Pure helpers behind the shared month calendar (MonthGrid, MonthCalendar).
// Months are 'YYYY-MM' strings and days are civil 'YYYY-MM-DD' strings within
// 0001-01-01..9999-12-31; the date rules come from judicial-calendar-time.
import {
  calendarMonth,
  civilDate,
  civilLabel,
  moveCalendarMonth,
} from './judicial-calendar-time.mjs';
import { calendarWeekdays } from './judicial-calendar-labels.mjs';

const pad = (value, width = 2) => String(value).padStart(width, '0');
const corners = { 0: 'tl', 6: 'tr', 35: 'bl', 41: 'br' };

// One initial per weekday, Monday first. The full name is kept because the
// initials alone are ambiguous (two "M").
export const weekdayInitials = calendarWeekdays.map((name) => ({ initial: name[0], name }));

export function monthOf(date) {
  return civilDate(date).slice(0, 7);
}

// 'enero de 2026': the civil label of the first day without its "1 de ".
export function monthTitle(month) {
  return civilLabel(calendarMonth(month).from).replace(/^1 de /, '');
}

// Always 42 cells (six Monday-first weeks), so two months shown side by side
// keep the same height. Cells that would fall outside the civil range keep
// their position with a null date.
export function monthCells(month, today = null) {
  const { from, firstWeekday } = calendarMonth(month);
  const start = Date.parse(`${from}T00:00:00Z`) - (firstWeekday - 1) * 86400000;
  return Array.from({ length: 42 }, (_, index) => {
    const instant = new Date(start + index * 86400000),
      year = instant.getUTCFullYear(),
      valid = year >= 1 && year <= 9999,
      date = valid
        ? `${pad(year, 4)}-${pad(instant.getUTCMonth() + 1)}-${pad(instant.getUTCDate())}`
        : null,
      inMonth = date !== null && date.slice(0, 7) === month;
    return {
      date,
      day: date === null ? null : instant.getUTCDate(),
      inMonth,
      isToday: inMonth && today !== null && date === today,
      corner: corners[index] ?? null,
    };
  });
}

// The neighbouring month, or null past 0001-01 or 9999-12.
export function shiftMonth(month, delta) {
  return moveCalendarMonth(month, delta);
}

export function visibleMonths(month, count) {
  if (![1, 2].includes(count)) throw new Error('Muestra uno o dos meses.');
  calendarMonth(month);
  return [month, shiftMonth(month, 1)].filter(Boolean).slice(0, count);
}

// Targets of the previous/next controls. min and max are optional 'YYYY-MM'
// bounds: previous stops at min, next stops once the last visible month
// reaches max.
export function monthNavigation(month, { count = 1, min = null, max = null } = {}) {
  const shown = visibleMonths(month, count),
    before = shiftMonth(month, -1),
    after = shiftMonth(month, 1);
  return {
    previous: before === null || (min !== null && month <= min) ? null : before,
    next: after === null || (max !== null && shown.at(-1) >= max) ? null : after,
  };
}

// Whether an ISO civil date lies in [from, until), with until excluded.
export function periodContains(date, from, until) {
  return date >= from && date < until;
}

// The browser's local civil date. now is a parameter so tests can fix it.
export function civilToday(now = new Date()) {
  return `${pad(now.getFullYear(), 4)}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

// Background tints for day cells, one literal string per tone so the
// Tailwind scanner sees every class.
export const toneClasses = {
  period: 'bg-brand-50',
  countable: 'bg-success/10',
  excluded: 'bg-brand-100',
  unresolved: 'bg-amber-50',
  outside: 'bg-slate-100 text-slate-500',
};

// Classes for a day of the month: its tone (white when it has none) plus the
// ink text colour unless the tone sets its own.
export function dayTone(tone) {
  const background = toneClasses[tone] ?? 'bg-white';
  return /\btext-/.test(background) ? background : `${background} text-ink`;
}
