import { hearingInstant, hearingTimeParts } from './hearing-time.mjs';
import { deadlineInstant } from './deadline-time.mjs';

const dayMilliseconds = 86400000;
const maximumDays = 366;

function civilMilliseconds(value) {
  try {
    return Date.parse(hearingInstant({ date: value, time: '00:00:00', offset: '+00:00' }));
  } catch {
    throw new Error('Revisa la fecha de la agenda; usa una fecha existente entre 0001 y 9999.');
  }
}

function civilDate(milliseconds) {
  const date = new Date(milliseconds);
  const year = date.getUTCFullYear();
  if (!Number.isFinite(milliseconds) || year < 1 || year > 9999)
    throw new Error('El periodo debe conservar fechas entre 0001 y 9999.');
  return date.toISOString().slice(0, 10);
}

function periodView(view) {
  if (!['day', 'week', 'month'].includes(view))
    throw new Error('Selecciona una vista diaria, semanal o mensual.');
}

function monthStart(milliseconds, direction = 0) {
  const date = new Date(milliseconds);
  date.setUTCDate(1);
  date.setUTCMonth(date.getUTCMonth() + direction);
  return date.getTime();
}

function explicitOffsetMinutes(offset) {
  try {
    hearingInstant({ date: '2000-01-02', time: '00:00:00', offset });
  } catch {
    throw new Error('Declara un desfase UTC entre -14:00 y +14:00; -00:00 no es valido.');
  }
  const minutes = Number(offset.slice(1, 3)) * 60 + Number(offset.slice(4));
  return offset[0] === '-' ? -minutes : minutes;
}

export function agendaPeriod(view, date) {
  periodView(view);
  const selected = civilMilliseconds(date);
  let from = selected;
  let until = selected + dayMilliseconds;
  if (view === 'week') {
    const mondayOffset = (new Date(selected).getUTCDay() + 6) % 7;
    from = selected - mondayOffset * dayMilliseconds;
    until = from + 7 * dayMilliseconds;
  } else if (view === 'month') {
    from = monthStart(selected);
    until = monthStart(selected, 1);
  }
  return { from: civilDate(from), until: civilDate(until) };
}

export function shiftAgendaPeriod(view, date, direction) {
  periodView(view);
  if (direction !== -1 && direction !== 1)
    throw new Error('Selecciona el periodo anterior o el siguiente.');
  const selected = civilMilliseconds(date);
  if (view !== 'month') {
    const days = view === 'week' ? 7 : 1;
    return civilDate(selected + direction * days * dayMilliseconds);
  }
  const next = monthStart(selected, direction);
  const lastDay = new Date(monthStart(next, 1) - dayMilliseconds).getUTCDate();
  const day = Math.min(new Date(selected).getUTCDate(), lastDay);
  return civilDate(next + (day - 1) * dayMilliseconds);
}

function instantMilliseconds(instant) {
  if (instant !== null && typeof instant === 'object') {
    const value = deadlineInstant(instant);
    const milliseconds = value.unix_seconds * 1000;
    civilDate(milliseconds);
    return milliseconds;
  }
  const match =
    typeof instant === 'string' &&
    /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.\d{1,9})?(Z|[+-]\d{2}:\d{2})$/.exec(instant);
  if (!match) throw new Error('Revisa el instante; requiere fecha, segundos y desfase UTC.');
  const wholeSecond = match[1] + match[2];
  try {
    hearingTimeParts(wholeSecond);
  } catch {
    throw new Error('Revisa el instante; su fecha, hora o desfase UTC no son validos.');
  }
  return Date.parse(wholeSecond);
}

export function agendaCivilDay(instant, offset) {
  const milliseconds = instantMilliseconds(instant);
  const minutes = explicitOffsetMinutes(offset);
  // Whole-minute offsets cannot move a fraction to another civil day. Removing
  // the validated fraction avoids rounding nanoseconds into the following day.
  return civilDate(milliseconds + minutes * 60000);
}

export function agendaDays(from, until) {
  const first = civilMilliseconds(from);
  const boundary = civilMilliseconds(until);
  const count = (boundary - first) / dayMilliseconds;
  if (count < 1 || count > maximumDays)
    throw new Error('El intervalo de la agenda debe contener entre 1 y 366 dias.');
  return Array.from({ length: count }, (_, index) => civilDate(first + index * dayMilliseconds));
}
