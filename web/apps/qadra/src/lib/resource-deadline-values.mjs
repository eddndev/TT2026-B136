import {
  factObject as object,
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
  factDigest as digest,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import { resourceActivityReference, resourceActivityCommand } from './resource-activity-values.mjs';
import { resourceRecordValue } from './procedural-resource-validation.mjs';
import { resourceActivityRecord } from './resource-activity-validation.mjs';
import { factAdministration } from './procedural-fact-validation.mjs';
import { deadlineNormalizeCommand } from './deadline-values.mjs';
import { deadlinePreparedValue } from './deadline-validation.mjs';
import { deadlineMatches } from './deadline-submission.mjs';

export function resourceDeadlineCommand(raw) {
  object(raw, [
    'case_id',
    'resource_id',
    'association_id',
    'expected_resource_revision',
    'resource',
    'act',
    'deadline',
  ]);
  const command = {
    case_id: uuid(raw.case_id),
    resource_id: uuid(raw.resource_id),
    association_id: uuid(raw.association_id),
    expected_resource_revision: revision(raw.expected_resource_revision),
    resource: resourceActivityReference(raw.resource),
    act: raw.act === null ? null : resourceActivityReference(raw.act, true),
    deadline: deadlineNormalizeCommand(raw.deadline),
  };
  if (
    command.deadline.change.action !== 'register' ||
    command.deadline.change.expected_revision !== 0 ||
    command.deadline.change.definition.input.selection.case_id !== command.case_id ||
    command.resource.id !== command.resource_id ||
    command.resource.revision > command.expected_resource_revision ||
    (command.act && command.act.resource_revision > command.expected_resource_revision)
  )
    invalid();
  return command;
}
function sameAdministrationCapture(resource, deadline) {
  if (resource.kind !== 'recorded' || deadline.kind !== 'recorded') return same(resource, deadline);
  const match = /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.(\d{1,9}))?Z$/.exec(
    resource.changed_at,
  );
  if (!match) invalid();
  const milliseconds = Date.parse(`${match[1]}Z`),
    instant = new Date(milliseconds);
  if (
    !Number.isFinite(milliseconds) ||
    instant.getUTCFullYear() < 1 ||
    instant.getUTCFullYear() > 9999 ||
    instant.toISOString().slice(0, 19) !== match[1]
  )
    invalid();
  const changed_at = {
    unix_seconds: milliseconds / 1000,
    nanosecond: Number((match[2] || '').padEnd(9, '0')),
    offset_seconds: 0,
  };
  return same(
    { ...resource, changed_at },
    {
      ...deadline,
      changed_at: { ...deadline.changed_at, offset_seconds: 0 },
    },
  );
}
export function resourceDeadlineDraft(value) {
  object(value, ['command', 'deadline', 'association', 'submission_digest']);
  const c = resourceDeadlineCommand(value.command),
    a = value.association,
    d = value.deadline;
  if (!same(c, value.command)) invalid();
  digest(value.submission_digest);
  deadlinePreparedValue(d);
  if (d.case_id !== c.case_id || d.result_revision !== 1 || !same(d.command, c.deadline)) invalid();
  object(a, [
    'command',
    'resource',
    'act',
    'recorded_by',
    'observed_administration',
    'observed_resource_head',
    'submission_digest',
  ]);
  const command = resourceActivityCommand(a.command);
  const expected = {
    case_id: c.case_id,
    resource_id: c.resource_id,
    association_id: c.association_id,
    operation_id: c.deadline.operation_id,
    expected_resource_revision: c.expected_resource_revision,
    change: {
      action: 'link',
      expected_revision: 0,
      resource: c.resource,
      act: c.act,
      target: {
        kind: 'deadline',
        id: c.deadline.deadline_id,
        revision: 1,
        capture_digest: d.capture_digest,
      },
    },
  };
  if (!same(command, a.command) || !same(command, expected)) invalid();
  object(a.recorded_by, ['id', 'email']);
  uuid(a.recorded_by.id);
  if (
    d.author.kind !== 'user' ||
    a.recorded_by.id !== d.actor_id ||
    a.recorded_by.email !== d.author.email
  )
    invalid();
  digest(a.submission_digest);
  factAdministration(a.observed_administration, c.case_id);
  if (!sameAdministrationCapture(a.observed_administration, d.calculation.material.administration))
    invalid();
  const head = resourceActivityReference(a.observed_resource_head);
  if (head.id !== c.resource_id || head.revision !== c.expected_resource_revision) invalid();
  const resource = resourceRecordValue(a.resource, c.case_id, c.resource_id, c.resource.revision);
  if (
    resource.receipt.capture_digest !== c.resource.capture_digest ||
    (head.revision === c.resource.revision && head.capture_digest !== c.resource.capture_digest)
  )
    invalid();
  if (c.act === null) {
    if (a.act !== null) invalid();
  } else {
    const act = resourceRecordValue(a.act, c.case_id, c.resource_id, c.act.resource_revision);
    if (
      act.act?.id !== c.act.id ||
      act.act.revision !== c.act.revision ||
      act.receipt.capture_digest !== c.act.capture_digest ||
      (act.revision === resource.revision && !same(act, resource)) ||
      (act.revision === head.revision && act.receipt.capture_digest !== head.capture_digest)
    )
      invalid();
  }
  return value;
}
export function resourceDeadlineRecordsMatch(result, raw) {
  try {
    const draft = resourceDeadlineDraft(raw),
      c = draft.command,
      a = draft.association;
    object(result, ['deadline', 'association']);
    if (!deadlineMatches(result.deadline, draft.deadline)) return false;
    const row = resourceActivityRecord(
      result.association,
      c.case_id,
      c.resource_id,
      c.association_id,
      1,
    );
    const expectedSelection = { resource: c.resource, act: c.act, target: a.command.change.target };
    return (
      row.status === 'linked' &&
      row.reason === null &&
      row.receipt.operation_id === c.deadline.operation_id &&
      row.receipt.submission_digest === a.submission_digest &&
      row.receipt.expected_resource_revision === c.expected_resource_revision &&
      same(row.selection, expectedSelection) &&
      same(row.recorded_by, a.recorded_by) &&
      same(row.recorded_administration, a.observed_administration) &&
      same(row.recorded_resource_head, a.observed_resource_head) &&
      same(row.sources.resource, a.resource) &&
      same(row.sources.act, a.act) &&
      same(row.sources.target, { kind: 'deadline', record: result.deadline })
    );
  } catch {
    return false;
  }
}

export function resourceDeadlineMatches(result, raw) {
  try {
    object(result, ['deadline', 'association', 'submission_digest']);
    return (
      result.submission_digest === raw.submission_digest &&
      resourceDeadlineRecordsMatch(
        { deadline: result.deadline, association: result.association },
        raw,
      )
    );
  } catch {
    return false;
  }
}
