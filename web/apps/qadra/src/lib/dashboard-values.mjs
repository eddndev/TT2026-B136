import { factObject, factUuid } from './procedural-fact-primitives.mjs';
import { hearingTimeParts } from './hearing-time.mjs';

const counts = [
  'active_cases',
  'pending_contracts',
  'deadlines_overdue',
  'deadlines_due_48h',
  'deadlines_due_7d',
  'deadlines_unresolved',
];
function invalid() {
  throw new Error('No se pudieron validar los indicadores. Actualiza la consulta.');
}
function count(value) {
  if (!Number.isSafeInteger(value) || value < 0) invalid();
}
function instant(value) {
  const match =
    typeof value === 'string' &&
    /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.\d{1,9})?(Z|[+-]\d{2}:\d{2})$/.exec(value);
  if (!match) invalid();
  hearingTimeParts(`${match[1]}${match[2]}`);
}
export function dashboardValue(value) {
  try {
    factObject(value, ['checked_at', 'scope', ...counts, 'workload']);
    instant(value.checked_at);
    if (!['office', 'assigned_cases'].includes(value.scope)) invalid();
    counts.forEach((key) => count(value[key]));
    if (value.deadlines_due_48h > value.deadlines_due_7d || !Array.isArray(value.workload))
      invalid();
    const seen = new Set();
    for (const row of value.workload) {
      factObject(row, ['user_id', 'email', 'active_cases']);
      if (factUuid(row.user_id) !== row.user_id || seen.has(row.user_id)) invalid();
      seen.add(row.user_id);
      count(row.active_cases);
      if (
        row.active_cases > value.active_cases ||
        typeof row.email !== 'string' ||
        row.email.length > 254 ||
        row.email !== row.email.trim().toLowerCase() ||
        /[^\x21-\x7e]/.test(row.email) ||
        !/^[^@]+@[^@]+$/.test(row.email)
      )
        invalid();
    }
  } catch {
    invalid();
  }
  return value;
}
