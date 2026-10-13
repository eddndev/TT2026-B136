import { memberUuid, memberRevision, memberInstant } from './members-values.mjs';
const roles = new Set(['litigator', 'paralegal']);
const statuses = new Set(['pending', 'pending_mfa', 'accepted', 'expired']);
const deliveries = new Set(['pending', 'sent', 'uncertain', 'rejected']);
function invalid() {
  throw new Error('La respuesta de invitaciones no es valida.');
}
function summary(value, id) {
  if (!value || typeof value !== 'object') invalid();
  memberUuid(value.id);
  memberUuid(value.operation_id);
  memberRevision(value.revision);
  if (
    value.revision === '0' ||
    (id && value.id !== id) ||
    !roles.has(value.role) ||
    !statuses.has(value.status) ||
    !deliveries.has(value.delivery) ||
    typeof value.email !== 'string' ||
    value.email.length > 254
  )
    invalid();
  memberInstant(value.issued_at);
  memberInstant(value.expires_at);
  if (value.accepted_at !== null) memberInstant(value.accepted_at);
  return value;
}
export function invitationsApi(request) {
  let active = true;
  async function call(path, options) {
    if (!active) invalid();
    const result = await request(`/member-invitations${path}`, options);
    if (!active) invalid();
    return result;
  }
  return {
    dispose() {
      active = false;
    },
    async list(after = null) {
      if (after) memberUuid(after);
      const value = await call(`?limit=20${after ? `&after=${after}` : ''}`);
      if (!value || !Array.isArray(value.items) || value.items.length > 20) invalid();
      let previous = after ?? '';
      for (const row of value.items) {
        summary(row);
        if (row.id <= previous) invalid();
        previous = row.id;
      }
      if (value.next !== null && (!value.items.length || value.next !== previous)) invalid();
      return value;
    },
    async get(id) {
      memberUuid(id);
      return summary(await call(`/${id}`), id);
    },
    async issue(data) {
      memberUuid(data.operation_id);
      if (!roles.has(data.role)) invalid();
      return summary(await call('', { method: 'POST', data }), data.operation_id);
    },
    async resend(id, data) {
      memberUuid(id);
      memberUuid(data.operation_id);
      memberRevision(data.expected_revision);
      return summary(await call(`/${id}/resend`, { method: 'POST', data }), id);
    },
  };
}
