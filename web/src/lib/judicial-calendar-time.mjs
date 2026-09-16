const months = [
  'enero',
  'febrero',
  'marzo',
  'abril',
  'mayo',
  'junio',
  'julio',
  'agosto',
  'septiembre',
  'octubre',
  'noviembre',
  'diciembre',
];
const pad = (value, width = 2) => String(value).padStart(width, '0');
function monthLength(year, month) {
  return [
    31,
    year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0) ? 29 : 28,
    31,
    30,
    31,
    30,
    31,
    31,
    30,
    31,
    30,
    31,
  ][month - 1];
}
export function civilDate(value) {
  if (typeof value !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(value))
    throw new Error('Usa una fecha civil completa: a\u00f1o, mes y d\u00eda.');
  const [year, month, day] = value.split('-').map(Number);
  if (
    year < 1 ||
    year > 9999 ||
    month < 1 ||
    month > 12 ||
    day < 1 ||
    day > monthLength(year, month)
  )
    throw new Error('La fecha civil no existe. Revisa el a\u00f1o, mes y d\u00eda.');
  return value;
}
export function civilDays(from, through) {
  return (
    (Date.parse(`${civilDate(through)}T00:00:00Z`) - Date.parse(`${civilDate(from)}T00:00:00Z`)) /
      86400000 +
    1
  );
}
export function calendarMonth(value) {
  if (typeof value !== 'string' || !/^\d{4}-\d{2}$/.test(value))
    throw new Error('Selecciona un mes completo.');
  const from = civilDate(`${value}-01`),
    [year, month] = value.split('-').map(Number),
    length = monthLength(year, month);
  const weekday = new Date(`${from}T00:00:00Z`).getUTCDay();
  return {
    from,
    through: `${value}-${pad(length)}`,
    length,
    firstWeekday: weekday === 0 ? 7 : weekday,
  };
}
export function moveCalendarMonth(value, delta) {
  calendarMonth(value);
  if (![-1, 1].includes(delta)) throw new Error('Desplazamiento de mes no v\u00e1lido.');
  const [year, month] = value.split('-').map(Number),
    index = (year - 1) * 12 + month - 1 + delta;
  if (index < 0 || index >= 9999 * 12) return null;
  return `${pad(Math.floor(index / 12) + 1, 4)}-${pad((index % 12) + 1)}`;
}
export function civilLabel(value) {
  const [year, month, day] = civilDate(value).split('-');
  return `${Number(day)} de ${months[Number(month) - 1]} de ${year}`;
}
