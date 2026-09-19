import { memberObject, memberRole, memberCursor, memberInvalid } from './members-values.mjs';

export function memberQuery(input = {}, caseScope = false) {
  const mode = caseScope ? 'selection' : 'status';
  memberObject(input, ['limit', mode, 'role', 'email_prefix', 'cursor'], []);
  const limit = input.limit === undefined ? 50 : input.limit;
  if (!Number.isInteger(limit) || limit < 1 || limit > 100) memberInvalid();
  const value = input[mode] === undefined ? (caseScope ? 'assigned' : 'active') : input[mode];
  if (!(caseScope ? ['assigned', 'available'] : ['active', 'inactive', 'all']).includes(value))
    memberInvalid();
  const query = { limit, [mode]: value };
  if (input.role !== undefined) query.role = memberRole(input.role);
  if (input.email_prefix !== undefined) {
    if (typeof input.email_prefix !== 'string' || /[^\x20-\x7e]/.test(input.email_prefix))
      memberInvalid();
    const prefix = input.email_prefix.trim().toLowerCase();
    if (prefix.length > 254) memberInvalid();
    if (prefix) query.email_prefix = prefix;
  }
  if (input.cursor !== undefined) query.cursor = memberCursor(input.cursor);
  return query;
}
