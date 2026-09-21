import { factObject, factUuid } from './procedural-fact-primitives.mjs';
import { hearingTimeParts } from './hearing-time.mjs';

export const incidentFailureLabels = {
  malformed_vault: 'Estructura cifrada no v\u00e1lida',
  authentication_failed: 'Autenticaci\u00f3n del contenido no v\u00e1lida',
  digest_mismatch: 'El resumen del contenido no coincide',
  snapshot_changed: 'La captura cambi\u00f3 durante la comprobaci\u00f3n',
};
export function incidentInvalid() {
  throw new Error('La respuesta de incidentes no conserva datos v\u00e1lidos.');
}
function object(value, keys, required = keys) {
  try {
    return factObject(value, keys, required);
  } catch {
    incidentInvalid();
  }
}
export function incidentUuid(value) {
  try {
    if (factUuid(value) !== value) incidentInvalid();
  } catch {
    incidentInvalid();
  }
  return value;
}
function instant(value) {
  const match =
    typeof value === 'string' &&
    /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.(\d{1,9}))?(?:Z|\+00:00)$/.exec(value);
  if (!match) incidentInvalid();
  try {
    hearingTimeParts(`${match[1]}Z`);
  } catch {
    incidentInvalid();
  }
  return `${match[1]}.${(match[2] || '').padEnd(9, '0')}`;
}
export function incidentTimeLabel(value) {
  instant(value);
  return `${value.replace('T', ' ').replace(/(?:Z|\+00:00)$/, '')} UTC`;
}
export function incidentRecordValue(value, id) {
  object(value, [
    'id',
    'observation_id',
    'case_id',
    'document_id',
    'document_version',
    'requester_id',
    'failure',
    'detected_at',
    'recorded_at',
    'expected_digest',
    'observed_snapshot_digest',
  ]);
  for (const name of ['id', 'observation_id', 'case_id', 'document_id', 'requester_id'])
    incidentUuid(value[name]);
  if (
    (id !== undefined && value.id !== id) ||
    !Number.isInteger(value.document_version) ||
    value.document_version < 1 ||
    value.document_version > 4294967295 ||
    typeof value.failure !== 'string' ||
    !Object.hasOwn(incidentFailureLabels, value.failure)
  )
    incidentInvalid();
  for (const name of ['expected_digest', 'observed_snapshot_digest'])
    if (typeof value[name] !== 'string' || !/^[0-9a-f]{64}$/.test(value[name])) incidentInvalid();
  if (instant(value.detected_at) > instant(value.recorded_at)) incidentInvalid();
  return value;
}
export function incidentQuery(input = {}) {
  object(input, ['limit', 'after_id'], []);
  const limit = input.limit === undefined ? 50 : input.limit;
  if (!Number.isInteger(limit) || limit < 1 || limit > 100) incidentInvalid();
  const query = { limit };
  if (input.after_id !== undefined) query.after_id = incidentUuid(input.after_id);
  return query;
}
export function incidentPageValue(value, query) {
  object(value, ['incidents', 'has_more', 'next_after_id']);
  if (
    !Array.isArray(value.incidents) ||
    value.incidents.length > query.limit ||
    typeof value.has_more !== 'boolean'
  )
    incidentInvalid();
  let previous = query.after_id || '';
  for (const row of value.incidents) {
    incidentRecordValue(row);
    if (row.id <= previous) incidentInvalid();
    previous = row.id;
  }
  if (value.has_more) {
    incidentUuid(value.next_after_id);
    if (!value.incidents.length || value.next_after_id !== previous) incidentInvalid();
  } else if (value.next_after_id !== null) incidentInvalid();
  return value;
}
