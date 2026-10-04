import {
  factObject as object,
  factInvalid as invalid,
  factSame as same,
  factRevision as revision,
} from './procedural-fact-primitives.mjs';
import {
  resourceHearingUuid as uuid,
  resourceHearingDigest as digest,
  resourceHearingText as text,
} from './resource-hearing-values.mjs';
import { hearingResultDraft, hearingResultValues } from './hearing-result-values.mjs';
import { deadlineNormalizeCommand } from './deadline-values.mjs';
import { deadlineResponsible } from './deadline-material.mjs';

export { uuid, digest, text };
export function derivedPrincipal(raw) {
  deadlineResponsible(raw);
  uuid(raw.id);
  if (!['owner', 'litigator'].includes(raw.role)) invalid('La identidad no permite este registro.');
  return raw;
}
export function derivedResultValues(raw) {
  object(raw, [
    'occurrence',
    'extent',
    'event_time',
    'summary',
    'attendees',
    'agreements',
    'provenance',
  ]);
  if (!Array.isArray(raw.attendees) || !Array.isArray(raw.agreements)) invalid();
  for (const row of raw.attendees)
    object(row, ['participant_id', 'revision', 'capacity', 'observation']);
  for (const row of raw.agreements) object(row, ['id', 'text']);
  object(raw.provenance, ['kind', 'reference', 'support']);
  if (raw.provenance.support !== null)
    object(raw.provenance.support, ['document_id', 'version', 'digest']);
  // Historical validation checks representation; the server owns future-time admission.
  const normalized = hearingResultValues(hearingResultDraft({ values: raw }), 253402300799999);
  normalized.event_time = structuredClone(raw.event_time);
  if (!same(normalized, raw)) invalid();
  if (normalized.event_time.precision === 'instant')
    normalized.event_time.at = normalized.event_time.at.replace(/\+00:00$/, 'Z');
  return normalized;
}
export function derivedResultCommand(raw) {
  object(raw, ['operation_id', 'hearing_id', 'result_id', 'change']);
  for (const key of ['operation_id', 'hearing_id', 'result_id']) uuid(raw[key]);
  const c = raw.change;
  object(c, ['action', 'expected_revision', 'anchor_revision', 'continuation', 'values']);
  if (c.action !== 'record' || c.expected_revision !== 0) invalid();
  revision(c.anchor_revision);
  if (c.continuation !== null) {
    object(c.continuation, ['result_id', 'revision']);
    uuid(c.continuation.result_id);
    revision(c.continuation.revision);
    if (c.continuation.result_id === raw.result_id) invalid();
  }
  return {
    ...structuredClone(raw),
    change: { ...structuredClone(c), values: derivedResultValues(c.values) },
  };
}
export function derivedCommand(raw, caseId, hearingId) {
  object(raw, ['case_id', 'result', 'deadline']);
  uuid(raw.case_id);
  const result = derivedResultCommand(raw.result),
    deadline = deadlineNormalizeCommand(raw.deadline);
  if (!same(deadline, raw.deadline) || deadline.change.action !== 'register') invalid();
  const selection = deadline.change.definition.input.selection,
    source = selection.source;
  if (
    raw.case_id !== caseId ||
    result.hearing_id !== hearingId ||
    selection.case_id !== caseId ||
    source.kind !== 'known' ||
    source.value.family !== 'hearing_result' ||
    source.value.hearing_id !== hearingId ||
    source.value.result_id !== result.result_id ||
    source.value.revision !== 1 ||
    (source.value.agreement_id !== null &&
      !result.change.values.agreements.some((row) => row.id === source.value.agreement_id))
  )
    invalid('La fuente debe ser el resultado inicial de esta audiencia y expediente.');
  return { case_id: caseId, result, deadline };
}
