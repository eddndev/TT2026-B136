import {
  factObject as object,
  factInvalid as invalid,
  factUuid as uuid,
  factDigest as digest,
  factRevision as revision,
  factText as text,
} from './procedural-fact-primitives.mjs';
import { hearingTimeParts } from './hearing-time.mjs';

export const resourceHearingKinds = {
  appeal_arguments: 'Alegatos aclaratorios de apelacion',
  written_revocation: 'Audiencia de revocacion escrita',
};
export function resourceHearingUuid(value) {
  if (uuid(value) !== value) invalid('La identidad debe conservar su forma exacta.');
  return value;
}
export function resourceHearingDigest(value) {
  if (digest(value) !== value) invalid('La huella debe conservar su forma exacta.');
  return value;
}
export function resourceHearingText(value, limit = 1000, multiline = true) {
  if (text(value, 'Captura declarada', limit, multiline) !== value) invalid();
  return value;
}
export function resourceHearingUtc(value) {
  const match =
    typeof value === 'string' &&
    /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.(\d{1,9}))?Z$/.exec(value);
  if (!match) invalid('La captura requiere un instante UTC exacto.');
  hearingTimeParts(`${match[1]}Z`);
  return (
    BigInt(Date.parse(`${match[1]}Z`) / 1000) * 1000000000n +
    BigInt((match[2] || '').padEnd(9, '0'))
  );
}
function scheduling(row) {
  if (
    !Object.hasOwn(resourceHearingKinds, row.kind) ||
    !['in_person', 'videoconference'].includes(row.modality)
  )
    invalid();
  hearingTimeParts(row.scheduled_at);
}
export function resourceHearingOverview(row) {
  object(row, [
    'case_id',
    'resource_id',
    'id',
    'revision',
    'kind',
    'scheduled_at',
    'modality',
    'participant_count',
    'association_id',
    'capture_digest',
  ]);
  for (const key of ['case_id', 'resource_id', 'id', 'association_id'])
    resourceHearingUuid(row[key]);
  resourceHearingDigest(row.capture_digest);
  if (
    row.revision !== 1 ||
    !Number.isInteger(row.participant_count) ||
    row.participant_count < 0 ||
    row.participant_count > 32
  )
    invalid();
  scheduling(row);
  return row;
}
export function resourceHearingReference(row, act = false) {
  object(row, ['id', 'revision', 'capture_digest', ...(act ? ['resource_revision'] : [])]);
  resourceHearingUuid(row.id);
  revision(row.revision);
  resourceHearingDigest(row.capture_digest);
  if (act) revision(row.resource_revision);
  return row;
}
export function resourceHearingValues(row) {
  object(row, [
    'kind',
    'scheduled_at',
    'modality',
    'venue',
    'note',
    'participants',
    'scheduling_basis',
  ]);
  scheduling(row);
  resourceHearingText(row.venue, 500, false);
  if (row.note !== null) resourceHearingText(row.note);
  if (!Array.isArray(row.participants) || row.participants.length > 32) invalid();
  let previous = null;
  for (const person of row.participants) {
    object(person, ['participant_id', 'revision']);
    resourceHearingUuid(person.participant_id);
    revision(person.revision);
    if (previous !== null && person.participant_id <= previous) invalid();
    previous = person.participant_id;
  }
  const basis = row.scheduling_basis;
  object(basis, ['statement', 'support']);
  resourceHearingText(basis.statement);
  object(basis.support, ['document_id', 'version', 'digest']);
  resourceHearingUuid(basis.support.document_id);
  revision(basis.support.version);
  resourceHearingDigest(basis.support.digest);
  return row;
}
