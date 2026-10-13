import { factObject, factUuid } from './procedural-fact-primitives.mjs';
import { hearingTimeParts } from './hearing-time.mjs';

export const memberRoles = ['owner', 'litigator', 'paralegal', 'client'];
export const memberMaxRevision = 9223372036854775807n;
export function memberInvalid() {
  throw new Error('Revisa los datos de la cuenta, los filtros y la respuesta del directorio.');
}
export function memberObject(value, keys, required = keys) {
  try {
    return factObject(value, keys, required);
  } catch {
    memberInvalid();
  }
}
export function memberUuid(value) {
  try {
    if (factUuid(value) !== value) memberInvalid();
  } catch {
    memberInvalid();
  }
  return value;
}
export function memberRevision(value) {
  if (
    typeof value !== 'string' ||
    !/^(0|[1-9][0-9]{0,18})$/.test(value) ||
    BigInt(value) > memberMaxRevision
  )
    memberInvalid();
  return value;
}
export function memberRole(value) {
  if (!memberRoles.includes(value)) memberInvalid();
  return value;
}
export function memberCursor(value) {
  if (
    typeof value !== 'string' ||
    !value.length ||
    value.length > 768 ||
    /[^\x21-\x7e]/.test(value)
  )
    memberInvalid();
  return value;
}
export function memberInstant(value) {
  const match =
    typeof value === 'string' &&
    /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.\d{1,9})?(?:Z|\+00:00)$/.exec(value);
  if (!match) memberInvalid();
  try {
    hearingTimeParts(`${match[1]}Z`);
  } catch {
    memberInvalid();
  }
  return value;
}
export function memberSummary(value, id, assigned = false) {
  memberObject(value, [
    'id',
    'email',
    'role',
    'active',
    'revision',
    ...(assigned ? ['assigned_at'] : []),
  ]);
  memberUuid(value.id);
  memberRole(value.role);
  memberRevision(value.revision);
  if (
    (id !== undefined && value.id !== id) ||
    typeof value.active !== 'boolean' ||
    typeof value.email !== 'string' ||
    value.email.length > 254 ||
    value.email !== value.email.trim().toLowerCase() ||
    /[^\x21-\x7e]/.test(value.email) ||
    !/^[^@]+@[^@]+$/.test(value.email)
  )
    memberInvalid();
  if (assigned && value.assigned_at !== null) memberInstant(value.assigned_at);
  return value;
}
export function memberAccessChange(input) {
  memberObject(input, ['expected_revision', 'role', 'active']);
  memberRevision(input.expected_revision);
  memberRole(input.role);
  if (typeof input.active !== 'boolean') memberInvalid();
  return { ...input };
}
export function memberAccessResult(value, id, change) {
  memberSummary(value, id);
  const expected = BigInt(change.expected_revision),
    received = BigInt(value.revision);
  if (
    value.role !== change.role ||
    value.active !== change.active ||
    (received !== expected && received !== expected + 1n)
  )
    memberInvalid();
  return value;
}
export function memberPageValue(value, query, caseId) {
  memberObject(value, ['items', 'has_more', 'next_cursor', ...(caseId ? ['case_id'] : [])]);
  if (caseId && value.case_id !== caseId) memberInvalid();
  if (
    !Array.isArray(value.items) ||
    value.items.length > query.limit ||
    typeof value.has_more !== 'boolean'
  )
    memberInvalid();
  let previous = '';
  for (const row of value.items) {
    memberSummary(row, undefined, !!caseId);
    if (
      row.id <= previous ||
      (query.role && row.role !== query.role) ||
      (query.email_prefix && !row.email.startsWith(query.email_prefix))
    )
      memberInvalid();
    if (caseId) {
      if (
        query.selection === 'available'
          ? !row.active || row.assigned_at !== null
          : row.assigned_at === null
      )
        memberInvalid();
    } else if (query.status !== 'all' && row.active !== (query.status === 'active'))
      memberInvalid();
    previous = row.id;
  }
  if (value.has_more) {
    memberCursor(value.next_cursor);
    if (!value.items.length || value.next_cursor === query.cursor) memberInvalid();
  } else if (value.next_cursor !== null) memberInvalid();
  return value;
}
