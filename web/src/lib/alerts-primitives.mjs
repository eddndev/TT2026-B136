import {
  factObject as object,
  factUuid as uuid,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import { deadlineInstant } from './deadline-time.mjs';

export { object, same };
export const alertMaximumRevision = 4294967295;
export function alertInvalid(message = 'La respuesta de alertas no conserva datos validos.') {
  throw new Error(message);
}
export function alertUuid(value) {
  if (uuid(value) !== value) alertInvalid();
  return value;
}
export function alertRevision(value, minimum = 0) {
  if (!Number.isInteger(value) || value < minimum || value > alertMaximumRevision) alertInvalid();
  return value;
}
export function alertInstant(value) {
  deadlineInstant(value);
  if (
    value.offset_seconds !== 0 ||
    value.unix_seconds < -62135596800 ||
    value.unix_seconds > 253402300799
  )
    alertInvalid();
  return value;
}
export function compareAlertInstants(a, b) {
  return a.unix_seconds < b.unix_seconds
    ? -1
    : a.unix_seconds > b.unix_seconds
      ? 1
      : a.nanosecond < b.nanosecond
        ? -1
        : a.nanosecond > b.nanosecond
          ? 1
          : 0;
}
export function alertKey(value) {
  return [value.created_at.unix_seconds, value.created_at.nanosecond, value.id];
}
export function compareAlertKeys(a, b) {
  for (let index = 0; index < a.length; index++) {
    if (a[index] < b[index]) return -1;
    if (a[index] > b[index]) return 1;
  }
  return 0;
}
