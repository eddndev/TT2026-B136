import {
  factObject as object,
  factInvalid as invalid,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import { derivedCommand, derivedPrincipal, digest } from './hearing-derived-deadline-command.mjs';
import {
  derivedResultDraft,
  derivedResultRecord,
  derivedResultMatchesDraft,
  derivedCaptureTime,
} from './hearing-derived-deadline-result.mjs';
import {
  derivedDeadlineMaterial,
  derivedMaterialMatchesRecord,
} from './hearing-derived-deadline-material.mjs';
import { deadlineRecordValue } from './deadline-validation.mjs';

export function derivedReady(row, command, actorId) {
  object(row, ['state', 'command', 'result', 'deadline', 'review_digest']);
  if (
    row.state !== 'ready' ||
    !same(derivedCommand(row.command, command.case_id, command.result.hearing_id), command)
  )
    invalid();
  digest(row.review_digest);
  derivedResultDraft(row.result, command.result, command.case_id, actorId);
  derivedDeadlineMaterial(row.deadline, command);
  return row;
}
function sourceEvent(row, command) {
  object(row, [
    'sequence',
    'family',
    'source_id',
    'revision',
    'case_id',
    'hearing_id',
    'operation_id',
  ]);
  const s = row.sequence;
  if (
    typeof s !== 'string' ||
    !/^[1-9][0-9]{0,18}$/.test(s) ||
    (s.length === 19 && s > '9223372036854775807')
  )
    invalid();
  if (
    row.family !== 'hearing_result' ||
    row.source_id !== command.result.result_id ||
    row.revision !== 1 ||
    row.case_id !== command.case_id ||
    row.hearing_id !== command.result.hearing_id ||
    row.operation_id !== command.result.operation_id
  )
    invalid();
}
function origin(row, command, actorId, reviewDigest, captureDigest) {
  object(row, [
    'case_id',
    'hearing_id',
    'result_id',
    'result_revision',
    'result_operation_id',
    'deadline_id',
    'deadline_revision',
    'deadline_operation_id',
    'recorded_by',
    'review_digest',
    'capture_digest',
    'source_event',
  ]);
  derivedPrincipal(row.recorded_by);
  if (
    row.case_id !== command.case_id ||
    row.hearing_id !== command.result.hearing_id ||
    row.result_id !== command.result.result_id ||
    row.result_revision !== 1 ||
    row.result_operation_id !== command.result.operation_id ||
    row.deadline_id !== command.deadline.deadline_id ||
    row.deadline_revision !== 1 ||
    row.deadline_operation_id !== command.deadline.operation_id ||
    row.recorded_by.id !== actorId ||
    row.review_digest !== reviewDigest ||
    row.capture_digest !== captureDigest
  )
    invalid();
  sourceEvent(row.source_event, command);
}
function components(row, command) {
  const r = row.result,
    d = row.deadline,
    actor = row.origin.recorded_by;
  const material = d.calculation.material,
    source = material.source;
  if (
    d.receipt.version.kind !== 'v2' ||
    d.receipt.action !== 'register' ||
    d.receipt.operation_id !== command.deadline.operation_id ||
    !same(d.definition, command.deadline.change.definition) ||
    !same(d.tracking.policies, command.deadline.change.tracking)
  )
    invalid();
  if (
    d.recorded_by.kind !== 'user' ||
    d.recorded_by.id !== actor.id ||
    d.recorded_by.email !== actor.email ||
    r.recorded_by.id !== actor.id ||
    r.recorded_by.email !== actor.email
  )
    invalid();
  if (!same(derivedCaptureTime(r.recorded_at), d.recorded_at))
    invalid('Las capturas no conservan el mismo instante y desfase.');
  if (
    source.values_digest !== r.values_digest ||
    source.submission_digest !== r.receipt.submission_digest ||
    source.status !== r.status ||
    source.sources_digest !== null ||
    material.source_head.reference.revision !== 1
  )
    invalid();
  const a = material.administration;
  if (
    a.kind !== 'recorded' ||
    a.revision !== r.recorded_administration_revision ||
    a.values_digest !== r.recorded_administration_digest
  )
    invalid();
  const observation = d.tracking.observations.entries.find((value) => value.role === 'source');
  if (
    !observation ||
    observation.revision !== 1 ||
    observation.submission_digest !== r.receipt.submission_digest
  )
    invalid();
}
export function derivedRecord(row, command, actorId, ready = null) {
  object(row, [
    'case_id',
    'command',
    'result',
    'deadline',
    'origin',
    'review_digest',
    'capture_digest',
  ]);
  if (
    row.case_id !== command.case_id ||
    !same(derivedCommand(row.command, command.case_id, command.result.hearing_id), command)
  )
    invalid();
  digest(row.review_digest);
  digest(row.capture_digest);
  derivedResultRecord(row.result, command.result, command.case_id, actorId);
  deadlineRecordValue(row.deadline, command.case_id, command.deadline.deadline_id, 1);
  origin(row.origin, command, actorId, row.review_digest, row.capture_digest);
  components(row, command);
  if (ready !== null) {
    if (row.review_digest !== ready.review_digest) invalid();
    derivedResultMatchesDraft(row.result, ready.result);
    derivedMaterialMatchesRecord(row.deadline, ready);
  }
  return row;
}
