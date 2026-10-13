function dateMilliseconds(value) {
  if (typeof value !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(value) || value.startsWith('0000'))
    throw new Error('Revisa la fecha de la audiencia; usa un a\u00f1o entre 0001 y 9999.');
  const milliseconds = Date.parse(`${value}T00:00:00Z`);
  if (!Number.isFinite(milliseconds) || new Date(milliseconds).toISOString().slice(0, 10) !== value)
    throw new Error('Revisa la fecha de la audiencia; debe existir en el calendario.');
  return milliseconds;
}

function completeTime(value) {
  if (typeof value !== 'string' || !/^\d{2}:\d{2}(:\d{2})?$/.test(value))
    throw new Error('Revisa la hora de la audiencia; usa horas, minutos y segundos completos.');
  const [hours, minutes, seconds = 0] = value.split(':').map(Number);
  if (hours > 23 || minutes > 59 || seconds > 59)
    throw new Error('Revisa la hora de la audiencia; no se admiten segundos intercalares.');
  return {
    value: value.length === 5 ? `${value}:00` : value,
    milliseconds: ((hours * 60 + minutes) * 60 + seconds) * 1000,
  };
}

function offsetMinutes(value) {
  if (typeof value !== 'string' || !/^[+-]\d{2}:\d{2}$/.test(value) || value === '-00:00')
    throw new Error('Declara el desfase UTC de la audiencia, por ejemplo -06:00 o +00:00.');
  const hours = Number(value.slice(1, 3)),
    minutes = Number(value.slice(4));
  if (hours > 14 || minutes > 59 || (hours === 14 && minutes))
    throw new Error('Revisa el desfase UTC; debe estar entre -14:00 y +14:00.');
  return (hours * 60 + minutes) * (value[0] === '-' ? -1 : 1);
}

export function hearingInstant(parts) {
  if (!parts || typeof parts !== 'object')
    throw new Error('Declara la fecha, la hora y el desfase UTC de la audiencia.');
  const day = dateMilliseconds(parts.date),
    time = completeTime(parts.time),
    offset = offsetMinutes(parts.offset);
  const utcYear = new Date(day + time.milliseconds - offset * 60000).getUTCFullYear();
  if (utcYear < 1 || utcYear > 9999)
    throw new Error('La fecha y el desfase deben conservar un a\u00f1o UTC entre 0001 y 9999.');
  return `${parts.date}T${time.value}${parts.offset}`;
}

export function hearingTimeParts(value) {
  const match =
    typeof value === 'string' &&
    /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2}:\d{2})(Z|[+-]\d{2}:\d{2})$/.exec(value);
  if (!match)
    throw new Error('Revisa la fecha y hora de la audiencia; requiere segundos y desfase UTC.');
  const parts = { date: match[1], time: match[2], offset: match[3] === 'Z' ? '+00:00' : match[3] };
  hearingInstant(parts);
  return parts;
}
