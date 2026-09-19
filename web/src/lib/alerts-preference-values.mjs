import {
  object,
  same,
  alertInvalid as invalid,
  alertUuid as uuid,
  alertRevision as revision,
  alertInstant as instant,
} from './alerts-primitives.mjs';

const families = ['hearing_upcoming', 'deadline_upcoming'];
const direct = ['overdue_unattended', 'review_required', 'due_changed_soon'];

function channels(value) {
  object(value, ['internal', 'email']);
  if (typeof value.internal !== 'boolean' || typeof value.email !== 'boolean') invalid();
}
export function alertPreferenceValues(raw, canonical = false) {
  object(raw, [...families, ...direct]);
  const value = structuredClone(raw);
  for (const key of families) {
    const family = value[key];
    object(family, ['lead_hours', 'channels']);
    const hours = family.lead_hours;
    if (
      !Array.isArray(hours) ||
      hours.length > 8 ||
      new Set(hours).size !== hours.length ||
      hours.some((hour) => !Number.isInteger(hour) || hour < 1 || hour > 720)
    )
      invalid();
    const sorted = [...hours].sort((a, b) => b - a);
    if (canonical && !same(hours, sorted)) invalid();
    family.lead_hours = sorted;
    channels(family.channels);
  }
  for (const key of direct) channels(value[key]);
  return value;
}
function defaults() {
  const both = () => ({ internal: true, email: true });
  return {
    hearing_upcoming: { lead_hours: [48, 24], channels: both() },
    deadline_upcoming: { lead_hours: [48, 24], channels: both() },
    overdue_unattended: both(),
    review_required: both(),
    due_changed_soon: both(),
  };
}
export function alertPreferenceCommand(value) {
  object(value, ['operation_id', 'expected_revision', 'values']);
  uuid(value.operation_id);
  revision(value.expected_revision);
  return { ...value, values: alertPreferenceValues(value.values) };
}
export function alertReadCommand(value) {
  object(value, ['operation_id']);
  uuid(value.operation_id);
  return { ...value };
}
export function alertPreferencesEnvelope(value, actor, command) {
  object(value, ['preferences']);
  const current = value.preferences;
  object(current, ['user_id', 'revision', 'values', 'updated_at', 'receipt', 'email_transport']);
  if (uuid(current.user_id) !== actor || !['ready', 'disabled'].includes(current.email_transport))
    invalid();
  revision(current.revision);
  alertPreferenceValues(current.values, true);
  if (current.revision === 0) {
    if (
      current.updated_at !== null ||
      current.receipt !== null ||
      !same(current.values, defaults())
    )
      invalid();
  } else {
    instant(current.updated_at);
    object(current.receipt, ['operation_id', 'expected_revision']);
    uuid(current.receipt.operation_id);
    revision(current.receipt.expected_revision);
    if (current.revision !== current.receipt.expected_revision + 1) invalid();
  }
  if (
    command &&
    (!current.receipt ||
      current.receipt.operation_id !== command.operation_id ||
      current.receipt.expected_revision !== command.expected_revision ||
      !same(current.values, command.values))
  )
    invalid('La respuesta no confirma estas preferencias.');
  return value;
}
