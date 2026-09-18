import { factObject as object, factInvalid as invalid } from './procedural-fact-primitives.mjs';
const pad = (value, size = 2) => String(value).padStart(size, '0');
export function deadlineInstant(value) {
  object(value, ['unix_seconds', 'nanosecond', 'offset_seconds']);
  if (
    !Number.isSafeInteger(value.unix_seconds) ||
    !Number.isInteger(value.nanosecond) ||
    value.nanosecond < 0 ||
    value.nanosecond > 999999999 ||
    !Number.isInteger(value.offset_seconds) ||
    Math.abs(value.offset_seconds) > 93599 ||
    !Number.isFinite(new Date((value.unix_seconds + value.offset_seconds) * 1000).getTime())
  )
    invalid('El instante calculado no conserva componentes representables.');
  return value;
}
export function deadlineInstantLabel(raw) {
  const value = deadlineInstant(raw),
    local = new Date((value.unix_seconds + value.offset_seconds) * 1000);
  const year = local.getUTCFullYear(),
    y = year < 0 ? `-${pad(-year, 4)}` : pad(year, 4);
  const date = `${y}-${pad(local.getUTCMonth() + 1)}-${pad(local.getUTCDate())}`;
  const time = `${pad(local.getUTCHours())}:${pad(local.getUTCMinutes())}:${pad(local.getUTCSeconds())}.${pad(value.nanosecond, 9)}`;
  const offset = Math.abs(value.offset_seconds),
    seconds = offset % 60;
  const zone = `UTC${value.offset_seconds < 0 ? '-' : '+'}${pad(Math.floor(offset / 3600))}:${pad(Math.floor(offset / 60) % 60)}${seconds ? `:${pad(seconds)}` : ''}`;
  return `${date} ${time} / ${zone}`;
}
