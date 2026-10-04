import {
  factObject as object,
  factInvalid as invalid,
  factSame as same,
  factRevision as revision,
} from './procedural-fact-primitives.mjs';
import {
  uuid,
  digest,
  text,
  derivedResultValues,
  derivedResultCommand,
} from './hearing-derived-deadline-command.mjs';
import { hearingTimeParts } from './hearing-time.mjs';
import { deadlineAuthor } from './deadline-material.mjs';
import { profileKinds } from './typed-participant-fields.mjs';

function anchor(row, command) {
  object(row, [
    'hearing_id',
    'revision',
    'values_digest',
    'submission_digest',
    'status',
    'kind',
    'scheduled_at',
    'scheduling_context',
  ]);
  uuid(row.hearing_id);
  revision(row.revision);
  digest(row.values_digest);
  digest(row.submission_digest);
  if (
    row.hearing_id !== command.hearing_id ||
    row.revision !== command.change.anchor_revision ||
    !['scheduled', 'cancelled'].includes(row.status)
  )
    invalid();
  if (!['initial', 'intermediate', 'oral_trial', 'sentencing'].includes(row.kind)) invalid();
  hearingTimeParts(row.scheduled_at);
  const c = row.scheduling_context;
  object(c, [
    'administration_revision',
    'administration_digest',
    'stage_revision',
    'stage',
    'stage_digest',
  ]);
  revision(c.administration_revision);
  digest(c.administration_digest);
  revision(c.stage_revision);
  if (c.stage_digest !== null) digest(c.stage_digest);
  const stage = {
    initial: 'investigation',
    intermediate: 'intermediate',
    oral_trial: 'trial',
    sentencing: 'trial',
  };
  if (c.stage !== stage[row.kind]) invalid();
}
function continuation(row, command) {
  const selected = command.change.continuation;
  if (selected === null) {
    if (row !== null) invalid();
    return;
  }
  object(row, [
    'hearing_id',
    'result_id',
    'revision',
    'values_digest',
    'submission_digest',
    'status',
  ]);
  uuid(row.hearing_id);
  uuid(row.result_id);
  revision(row.revision);
  digest(row.values_digest);
  digest(row.submission_digest);
  if (
    row.result_id !== selected.result_id ||
    row.revision !== selected.revision ||
    !['recorded', 'withdrawn'].includes(row.status)
  )
    invalid();
}
function projections(row, command) {
  if (!same(derivedResultValues(row.values), command.change.values)) invalid();
  anchor(row.anchor, command);
  continuation(row.continuation, command);
  const selected = row.values.attendees;
  if (!Array.isArray(row.attendees) || row.attendees.length !== selected.length) invalid();
  row.attendees.forEach((person, index) => {
    object(person, [
      'id',
      'revision',
      'profile',
      'display_name',
      'procedural_role',
      'kind',
      'subject',
      'values_digest',
      'subject_digest',
      'directory_status',
      'capacity',
      'observation',
    ]);
    uuid(person.id);
    revision(person.revision);
    digest(person.values_digest);
    const ref = selected[index];
    if (
      person.id !== ref.participant_id ||
      person.revision !== ref.revision ||
      person.capacity !== ref.capacity ||
      person.observation !== ref.observation ||
      !['active', 'archived'].includes(person.directory_status)
    )
      invalid();
    text(person.display_name, 200, false);
    text(person.procedural_role, 200, false);
    if (person.profile === 'manual') {
      if (person.kind !== null || person.subject !== null || person.subject_digest !== null)
        invalid();
    } else {
      if (person.profile !== 'typed' || !profileKinds.some((kind) => kind.key === person.kind))
        invalid();
      object(person.subject, ['id', 'revision', 'values_digest']);
      uuid(person.subject.id);
      revision(person.subject.revision);
      digest(person.subject.values_digest);
      digest(person.subject_digest);
      if (person.subject.values_digest !== person.subject_digest) invalid();
    }
  });
  const support = row.values.provenance.support;
  if (support === null) {
    if (row.support !== null) invalid();
  } else {
    const s = row.support;
    object(s, ['document_id', 'version', 'digest', 'name', 'format', 'policy']);
    uuid(s.document_id);
    revision(s.version);
    digest(s.digest);
    text(s.name, 128, false);
    if (
      s.document_id !== support.document_id ||
      s.version !== support.version ||
      s.digest !== support.digest ||
      !['pdf', 'docx'].includes(s.format) ||
      s.policy !== 'pdf_docx_v1'
    )
      invalid();
  }
}
export function derivedResultDraft(row, command, caseId, actorId) {
  object(row, [
    'case_id',
    'actor_id',
    'command',
    'result_revision',
    'values',
    'values_digest',
    'submission_digest',
    'anchor',
    'continuation',
    'observed_administration',
    'attendees',
    'support',
  ]);
  if (
    row.case_id !== caseId ||
    row.actor_id !== actorId ||
    row.result_revision !== 1 ||
    !same(derivedResultCommand(row.command), command)
  )
    invalid();
  digest(row.values_digest);
  digest(row.submission_digest);
  const a = row.observed_administration;
  object(a, ['revision', 'values_digest', 'status']);
  revision(a.revision);
  digest(a.values_digest);
  if (a.status !== 'active') invalid();
  projections(row, command);
  return row;
}
export function derivedResultRecord(row, command, caseId, actorId) {
  object(row, [
    'case_id',
    'hearing_id',
    'id',
    'revision',
    'values',
    'values_digest',
    'status',
    'reason',
    'receipt',
    'anchor',
    'continuation',
    'recorded_administration_revision',
    'recorded_administration_digest',
    'recorded_at',
    'recorded_by',
    'attendees',
    'support',
  ]);
  if (
    row.case_id !== caseId ||
    row.hearing_id !== command.hearing_id ||
    row.id !== command.result_id ||
    row.revision !== 1 ||
    row.status !== 'recorded' ||
    row.reason !== null
  )
    invalid();
  digest(row.values_digest);
  revision(row.recorded_administration_revision);
  digest(row.recorded_administration_digest);
  derivedCaptureTime(row.recorded_at);
  deadlineAuthor(row.recorded_by);
  if (row.recorded_by.id !== actorId) invalid();
  const r = row.receipt;
  object(r, ['operation_id', 'action', 'expected_revision', 'submission_digest']);
  digest(r.submission_digest);
  if (r.operation_id !== command.operation_id || r.action !== 'record' || r.expected_revision !== 0)
    invalid();
  projections(row, command);
  return row;
}
export function derivedResultMatchesDraft(row, draft) {
  if (
    row.values_digest !== draft.values_digest ||
    row.receipt.submission_digest !== draft.submission_digest ||
    row.recorded_administration_revision !== draft.observed_administration.revision ||
    row.recorded_administration_digest !== draft.observed_administration.values_digest
  )
    invalid();
  for (const key of ['values', 'anchor', 'continuation', 'attendees', 'support'])
    if (!same(row[key], draft[key])) invalid();
}

export function derivedCaptureTime(value) {
  const match =
    typeof value === 'string' &&
    /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2}:\d{2})(?:\.(\d{1,9}))?(Z|[+-]\d{2}:\d{2})$/.exec(value);
  if (!match || match[1].startsWith('0000')) invalid();
  const local = Date.parse(`${match[1]}T${match[2]}Z`);
  if (
    !Number.isFinite(local) ||
    new Date(local).toISOString().slice(0, 19) !== `${match[1]}T${match[2]}`
  )
    invalid();
  const zone = match[4],
    hours = zone === 'Z' ? 0 : Number(zone.slice(1, 3)),
    minutes = zone === 'Z' ? 0 : Number(zone.slice(4));
  if (hours > 23 || minutes > 59 || zone === '-00:00') invalid();
  const offset_seconds = (hours * 3600 + minutes * 60) * (zone[0] === '-' ? -1 : 1);
  const unix_seconds = local / 1000 - offset_seconds;
  const year = new Date(unix_seconds * 1000).getUTCFullYear();
  if (year < 1 || year > 9999) invalid();
  return { unix_seconds, nanosecond: Number((match[3] || '').padEnd(9, '0')), offset_seconds };
}
