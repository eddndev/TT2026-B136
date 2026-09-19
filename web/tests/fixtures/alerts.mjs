export const alertUserId = '00000000-0000-4000-8000-000000000004';
export const alertOperationId = '00000000-0000-4000-8000-000000000005';
export const alertOtherId = '00000000-0000-4000-8000-000000000006';
export const clone = (value) => structuredClone(value);

export function alertPreferenceValues() {
  return {
    hearing_upcoming: { lead_hours: [48, 24], channels: { internal: true, email: true } },
    deadline_upcoming: { lead_hours: [48, 24], channels: { internal: true, email: true } },
    overdue_unattended: { internal: true, email: true },
    review_required: { internal: true, email: true },
    due_changed_soon: { internal: true, email: true },
  };
}

export function alertPreferenceRequest() {
  return {
    operation_id: alertOperationId,
    expected_revision: 0,
    values: alertPreferenceValues(),
  };
}

export const alertInstant = (unix_seconds, nanosecond = 0) => ({
  unix_seconds,
  nanosecond,
  offset_seconds: 0,
});
export const alertCheckedAt = alertInstant(1767312000, 999999999);
export const alertId = '00000000-0000-4000-8000-000000000010';

export function alertPreferences(persisted = false) {
  return {
    preferences: {
      user_id: alertUserId,
      revision: persisted ? 1 : 0,
      values: alertPreferenceValues(),
      updated_at: persisted ? alertInstant(1767225600, 123456789) : null,
      receipt: persisted ? { operation_id: alertOperationId, expected_revision: 0 } : null,
      email_transport: 'disabled',
    },
  };
}

export function alertRecord(kind = 'upcoming', subject = 'deadline') {
  const due = alertInstant(1767315600, 123456789);
  const kinds = {
    upcoming: { kind, lead_hours: 24, activity_at: due },
    overdue_unattended: { kind, due_at: alertInstant(1767225600, 123456789) },
    review_required: { kind },
    due_changed_soon: {
      kind,
      previous_due_at: alertInstant(1767315600, 1),
      current_due_at: due,
    },
  };
  return {
    id: alertId,
    recipient_id: alertUserId,
    occurrence_id: '00000000-0000-4000-8000-000000000011',
    subject: {
      kind: subject,
      case_id: '00000000-0000-4000-8000-000000000012',
      id: '00000000-0000-4000-8000-000000000013',
    },
    subject_title: subject === 'hearing' ? 'Audiencia inicial' : 'Respuesta declarada',
    case_title: 'Defensa inicial',
    case_reference: 'NUC-123',
    kind: kinds[kind],
    origin: { revision: 3, evidence_digest: 'a'.repeat(64) },
    trigger_at: alertInstant(kind === 'overdue_unattended' ? 1767225600 : 1767229200, 123456789),
    created_at: alertInstant(1767229200, 123456789),
    read_at: null,
    state: { kind: 'active' },
    email: { kind: 'disabled' },
  };
}

export function alertPage(rows = [alertRecord()]) {
  return {
    checked_at: clone(alertCheckedAt),
    alerts: clone(rows),
    has_more: false,
    next_cursor: null,
  };
}

export function alertDetail(row = alertRecord()) {
  return { checked_at: clone(alertCheckedAt), alert: clone(row) };
}

export function alertReadReceipt() {
  const value = alertDetail();
  value.alert.read_at = alertInstant(1767230000, 9);
  return { operation_id: alertOperationId, ...value };
}

export function alertCursor(row, read = 'all', state = 'active') {
  return `a1:${row.created_at.unix_seconds}:${String(row.created_at.nanosecond).padStart(9, '0')}:${row.id}:${read}:${state}`;
}
