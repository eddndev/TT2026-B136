import { hearingInstant, hearingTimeParts } from './hearing-time.mjs';

function invalid() {
  throw new Error('Revisa la fecha, su precisi\u00f3n y el desfase UTC del hecho declarado.');
}
function dateValue(parts) {
  const start = hearingInstant({ ...parts, time: '00:00:00' });
  hearingInstant({ ...parts, time: '23:59:59' });
  return {
    value: { precision: 'date', date: parts.date, offset: parts.offset },
    first: Date.parse(start),
  };
}
export function hearingResultTime(draft, now = Date.now()) {
  if (!draft || !Number.isFinite(now)) invalid();
  let value, first;
  if (draft.precision === 'date') ({ value, first } = dateValue(draft));
  else if (draft.precision === 'instant') {
    value = { precision: 'instant', at: hearingInstant(draft) };
    first = Date.parse(value.at);
  } else invalid();
  if (first > now) throw new Error('La fecha declarada no puede estar completamente en el futuro.');
  return value;
}
export function hearingResultTimeDraft(value) {
  if (value === undefined || value === null)
    return { precision: 'date', date: '', time: '', offset: '' };
  if (value.precision === 'date') {
    if (Object.keys(value).some((key) => !['precision', 'date', 'offset'].includes(key))) invalid();
    dateValue(value);
    return { ...value, time: '' };
  }
  if (value.precision === 'instant') {
    if (Object.keys(value).some((key) => !['precision', 'at'].includes(key))) invalid();
    return { precision: 'instant', ...hearingTimeParts(value.at) };
  }
  invalid();
}
export function hearingResultTimeLabel(value) {
  const parts = hearingResultTimeDraft(value);
  if (!value) return 'Sin fecha declarada';
  return `${parts.date} / ${parts.precision === 'date' ? 'sin hora' : parts.time} / UTC${parts.offset}`;
}
