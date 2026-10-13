import { factObject } from './procedural-fact-primitives.mjs';
import { reportInstant, reportInvalid, reportUuid, reportType } from './case-report-values.mjs';
const nil = '00000000-0000-0000-0000-000000000000';
function identity(value) {
  reportUuid(value);
  if (value === nil) reportInvalid();
  return value;
}
export function reportLitigatorQuery(value = {}) {
  factObject(value, ['limit', 'after_id', 'report_type'], []);
  const type = reportType(value);
  const query = {
    limit: value.limit ?? 20,
    after_id: value.after_id ?? null,
    ...(type ? { report_type: type } : {}),
  };
  if (!Number.isInteger(query.limit) || query.limit < 1 || query.limit > 100) reportInvalid();
  if (query.after_id !== null) identity(query.after_id);
  return query;
}
export function reportLitigatorPage(value, query) {
  factObject(value, ['scope', 'checked_at', 'litigators', 'has_more', 'next_after_id']);
  reportInstant(value.checked_at);
  if (
    !['office', 'assigned_cases'].includes(value.scope) ||
    !Array.isArray(value.litigators) ||
    value.litigators.length > query.limit ||
    typeof value.has_more !== 'boolean'
  )
    reportInvalid();
  let previous = query.after_id;
  for (const row of value.litigators) {
    factObject(row, ['user_id', 'email']);
    identity(row.user_id);
    if (
      (previous !== null && row.user_id <= previous) ||
      typeof row.email !== 'string' ||
      row.email.length > 254 ||
      row.email !== row.email.trim().toLowerCase() ||
      /[^\x21-\x7e]/.test(row.email) ||
      !/^[^@]+@[^@]+$/.test(row.email)
    )
      reportInvalid();
    previous = row.user_id;
  }
  if (
    value.has_more
      ? !value.litigators.length || value.next_after_id !== previous
      : value.next_after_id !== null
  )
    reportInvalid();
  return value;
}
