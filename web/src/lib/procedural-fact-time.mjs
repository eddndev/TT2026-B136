import { civilDate } from './judicial-calendar-time.mjs';
import { factInvalid, factObject } from './procedural-fact-primitives.mjs';
const pad = (value, length = 2) => String(value).padStart(length, '0');
const first = Date.parse('0001-01-01T00:00:00Z');
const last = Date.parse('9999-12-31T23:59:59.999Z');
function component(value, min, max) {
  if (!Number.isInteger(value) || value < min || value > max)
    factInvalid('Revisa los componentes del tiempo declarado.');
  return value;
}
export function factTime(raw) {
  const precision = raw?.precision;
  const dateKeys = ['precision', 'year', 'month', 'day', 'offset_seconds'];
  const keys =
    precision === 'unknown'
      ? ['precision']
      : precision === 'date'
        ? dateKeys
        : precision === 'minute'
          ? [...dateKeys, 'hour', 'minute']
          : precision === 'second'
            ? [...dateKeys, 'hour', 'minute', 'second']
            : null;
  if (!keys) factInvalid('Selecciona la precisi\u00f3n temporal declarada.');
  factObject(
    raw,
    keys,
    keys.filter((key) => key !== 'offset_seconds'),
  );
  if (precision === 'unknown') return { precision };
  const year = component(raw.year, 1, 9999),
    month = component(raw.month, 1, 12),
    day = component(raw.day, 1, 31);
  const date = civilDate(`${pad(year, 4)}-${pad(month)}-${pad(day)}`);
  const offset_seconds =
    raw.offset_seconds == null ? null : component(raw.offset_seconds, -50400, 50400);
  if (offset_seconds !== null && offset_seconds % 60)
    factInvalid('El desfase debe indicar minutos completos.');
  const value = { precision, year, month, day, offset_seconds };
  let seconds = 0,
    span = 86399999;
  if (precision !== 'date') {
    value.hour = component(raw.hour, 0, 23);
    value.minute = component(raw.minute, 0, 59);
    seconds = value.hour * 3600 + value.minute * 60;
    span = 59999;
  }
  if (precision === 'second') {
    value.second = component(raw.second, 0, 59);
    seconds += value.second;
    span = 0;
  }
  // Endpoints test representability only; they are never returned as declared times.
  if (offset_seconds !== null) {
    const start = Date.parse(`${date}T00:00:00Z`) + (seconds - offset_seconds) * 1000;
    if (start < first || start + span > last)
      factInvalid('El tiempo declarado excede los a\u00f1os 1 a 9999 en UTC.');
  }
  return value;
}
export function factTimeLabel(raw) {
  const value = factTime(raw);
  if (value.precision === 'unknown') return 'Fecha desconocida';
  const date = `${pad(value.year, 4)}-${pad(value.month)}-${pad(value.day)}`;
  const hour =
    value.precision === 'date'
      ? 'sin hora'
      : `${pad(value.hour)}:${pad(value.minute)}${value.precision === 'second' ? `:${pad(value.second)}` : ''}`;
  const offset = value.offset_seconds;
  const zone =
    offset === null
      ? 'desfase no declarado'
      : `UTC${offset < 0 ? '-' : '+'}${pad(Math.floor(Math.abs(offset) / 3600))}:${pad((Math.abs(offset) / 60) % 60)}`;
  return `${date} / ${hour} / ${zone}`;
}
