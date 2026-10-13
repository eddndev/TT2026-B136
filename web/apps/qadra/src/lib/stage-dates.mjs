export const emptyDate = () => ({ precision: 'date', date: '', time: '', offset: '' });
function invalid() {
  throw new Error('Revisa la fecha, la hora y el desfase UTC declarado.');
}
function offsetMinutes(offset) {
  if (offset === 'Z') return 0;
  if (!/^[+-]\d{2}:\d{2}$/.test(offset) || offset === '-00:00') invalid();
  const hours = Number(offset.slice(1, 3)),
    minutes = Number(offset.slice(4));
  if (hours > 14 || minutes > 59 || (hours === 14 && minutes)) invalid();
  return (hours * 60 + minutes) * (offset[0] === '-' ? -1 : 1);
}
function calendarDate(value) {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value) || value.startsWith('0000')) invalid();
  const millis = Date.parse(`${value}T00:00:00Z`);
  if (!Number.isFinite(millis) || new Date(millis).toISOString().slice(0, 10) !== value) invalid();
  return millis;
}
export function dateInterval(value) {
  if (value.precision === 'date') {
    const first = calendarDate(value.date) - offsetMinutes(value.offset) * 60000;
    return [first, first + 86400000 - 1];
  }
  const millis = Date.parse(value.at);
  if (!Number.isFinite(millis)) invalid();
  return [millis, millis];
}
export function declaredDate(draft, now = Date.now()) {
  if (draft.precision === 'date' && draft.offset === 'Z') invalid();
  calendarDate(draft.date);
  offsetMinutes(draft.offset);
  let value;
  if (draft.precision === 'date')
    value = { precision: 'date', date: draft.date, offset: draft.offset };
  else if (draft.precision === 'instant') {
    if (!/^\d{2}:\d{2}(:\d{2})?$/.test(draft.time)) invalid();
    const [h, m, s = 0] = draft.time.split(':').map(Number);
    if (h > 23 || m > 59 || s > 59) invalid();
    value = {
      precision: 'instant',
      at: `${draft.date}T${draft.time.length === 5 ? `${draft.time}:00` : draft.time}${draft.offset}`,
    };
  } else invalid();
  if (dateInterval(value)[0] > now)
    throw new Error('La fecha declarada no puede estar completamente en el futuro.');
  return value;
}
export function dateLabel(value) {
  if (!value) return 'Sin fecha declarada';
  if (value.precision === 'date') return `${value.date} / sin hora / UTC${value.offset}`;
  return value.at.replace('T', ' ').replace(/Z$/, ' UTC+00:00');
}
