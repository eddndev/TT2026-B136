import {
  object,
  alertInvalid as invalid,
  alertUuid as uuid,
  alertRevision as revision,
  alertInstant as instant,
  compareAlertInstants as compare,
  alertKey,
  compareAlertKeys,
} from './alerts-primitives.mjs';
import { alertCursor } from './alerts-query.mjs';

function observed(value, created, checked) {
  instant(value);
  if (compare(value, created) < 0 || compare(value, checked) > 0) invalid();
}
function capturedText(value, maximum) {
  if (
    typeof value !== 'string' ||
    !value ||
    value !== value.trim() ||
    [...value].length > maximum ||
    /[\u0000-\u001f\u007f-\u009f\ud800-\udfff]/u.test(value)
  )
    invalid();
}
function kindValue(value, row) {
  const tag = value?.kind;
  if (tag === 'upcoming') {
    object(value, ['kind', 'lead_hours', 'activity_at']);
    instant(value.activity_at);
    if (
      !Number.isInteger(value.lead_hours) ||
      value.lead_hours < 1 ||
      value.lead_hours > 720 ||
      row.trigger_at.unix_seconds !== value.activity_at.unix_seconds - value.lead_hours * 3600 ||
      row.trigger_at.nanosecond !== value.activity_at.nanosecond ||
      compare(row.created_at, value.activity_at) >= 0
    )
      invalid();
    return;
  }
  if (row.subject.kind !== 'deadline') invalid();
  if (tag === 'overdue_unattended') {
    object(value, ['kind', 'due_at']);
    instant(value.due_at);
    if (compare(row.trigger_at, value.due_at) !== 0) invalid();
  } else if (tag === 'review_required') object(value, ['kind']);
  else if (tag === 'due_changed_soon') {
    object(value, ['kind', 'previous_due_at', 'current_due_at']);
    instant(value.previous_due_at);
    instant(value.current_due_at);
    if (compare(value.previous_due_at, value.current_due_at) === 0) invalid();
  } else invalid();
}
function stateValue(value, row, checked) {
  if (value?.kind === 'active') object(value, ['kind']);
  else if (value?.kind === 'resolved') {
    object(value, ['kind', 'at', 'reason']);
    if (
      ![
        'superseded',
        'attention_recorded',
        'target_retired',
        'cancelled_hearing',
        'no_longer_eligible',
      ].includes(value.reason)
    )
      invalid();
    observed(value.at, row.created_at, checked);
  } else invalid();
}
function emailValue(value, row, checked) {
  if (value?.kind === 'accepted') {
    object(value, ['kind', 'accepted_at']);
    observed(value.accepted_at, row.created_at, checked);
  } else {
    object(value, ['kind']);
    if (!['disabled', 'pending', 'sending', 'failed', 'unknown', 'cancelled'].includes(value.kind))
      invalid();
  }
}
export function alertRecordValue(row, actor, checked, id) {
  object(row, [
    'id',
    'recipient_id',
    'occurrence_id',
    'subject',
    'kind',
    'origin',
    'subject_title',
    'case_title',
    'case_reference',
    'trigger_at',
    'created_at',
    'read_at',
    'state',
    'email',
  ]);
  uuid(row.id);
  uuid(row.occurrence_id);
  if (uuid(row.recipient_id) !== actor || (id !== undefined && row.id !== id)) invalid();
  object(row.subject, ['kind', 'case_id', 'id']);
  if (!['hearing', 'deadline'].includes(row.subject.kind)) invalid();
  uuid(row.subject.case_id);
  uuid(row.subject.id);
  capturedText(row.subject_title, 200);
  capturedText(row.case_title, 200);
  capturedText(row.case_reference, 100);
  object(row.origin, ['revision', 'evidence_digest']);
  revision(row.origin.revision, 1);
  if (
    typeof row.origin.evidence_digest !== 'string' ||
    !/^[0-9a-f]{64}$/.test(row.origin.evidence_digest)
  )
    invalid();
  instant(row.created_at);
  instant(row.trigger_at);
  if (compare(row.trigger_at, row.created_at) > 0 || compare(row.created_at, checked) > 0)
    invalid();
  if (row.read_at !== null) observed(row.read_at, row.created_at, checked);
  kindValue(row.kind, row);
  stateValue(row.state, row, checked);
  emailValue(row.email, row, checked);
  return row;
}
export function alertDetailValue(value, actor, id, operation) {
  object(
    value,
    operation === undefined ? ['checked_at', 'alert'] : ['operation_id', 'checked_at', 'alert'],
  );
  instant(value.checked_at);
  alertRecordValue(value.alert, actor, value.checked_at, id);
  if (
    operation !== undefined &&
    (uuid(value.operation_id) !== operation || value.alert.read_at === null)
  )
    invalid('La respuesta no confirma esta lectura.');
  return value;
}
export function alertPageValue(value, actor, query) {
  object(value, ['checked_at', 'alerts', 'has_more', 'next_cursor']);
  instant(value.checked_at);
  if (
    !Array.isArray(value.alerts) ||
    value.alerts.length > query.limit ||
    typeof value.has_more !== 'boolean' ||
    (value.has_more ? value.next_cursor === null : value.next_cursor !== null)
  )
    invalid();
  let previous = query.after;
  const identities = new Set();
  for (const row of value.alerts) {
    alertRecordValue(row, actor, value.checked_at);
    const key = alertKey(row);
    if (
      (query.read === 'unread' && row.read_at !== null) ||
      (query.state === 'active' && row.state.kind !== 'active') ||
      identities.has(row.id) ||
      (previous && compareAlertKeys(key, previous) >= 0)
    )
      invalid();
    identities.add(row.id);
    previous = key;
  }
  if (value.has_more) {
    const next = alertCursor(value.next_cursor, query);
    if (
      next[0] > value.checked_at.unix_seconds ||
      (next[0] === value.checked_at.unix_seconds && next[1] > value.checked_at.nanosecond) ||
      (query.after && compareAlertKeys(next, query.after) >= 0) ||
      (previous && compareAlertKeys(next, previous) > 0)
    )
      invalid();
  }
  return value;
}
