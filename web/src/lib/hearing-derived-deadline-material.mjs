import {
  factObject as object,
  factInvalid as invalid,
  factSame as same,
  factRevision as revision,
} from './procedural-fact-primitives.mjs';
import { uuid, digest, text } from './hearing-derived-deadline-command.mjs';
import { derivedCaptureTime } from './hearing-derived-deadline-result.mjs';
import { deadlineProfileDefinition, deadlineProfileScope } from './deadline-profile-values.mjs';
import { calendarValues } from './judicial-calendar-values.mjs';
import { deadlineAuthor, deadlineResponsible } from './deadline-material.mjs';
import { deadlineResult } from './deadline-result.mjs';

function metadata(row) {
  uuid(row.id);
  revision(row.revision);
  const r = row.receipt;
  object(r, ['operation_id', 'action', 'expected_revision', 'submission_digest']);
  uuid(r.operation_id);
  digest(r.submission_digest);
  if (
    !['publish', 'replace'].includes(r.action) ||
    row.status !== 'published' ||
    r.expected_revision !== row.revision - 1 ||
    (r.action === 'publish') !== (row.revision === 1)
  )
    invalid();
  if (r.action === 'publish') {
    if (row.reason !== null) invalid();
  } else text(row.reason);
  deadlineAuthor(row.recorded_by);
  derivedCaptureTime(row.recorded_at);
}
function profile(row, caseId) {
  object(row, [
    'collection',
    'id',
    'revision',
    'status',
    'algorithm',
    'definition_digest',
    'scope',
    'reason',
    'receipt',
    'recorded_at',
    'recorded_by',
    'definition',
  ]);
  metadata(row);
  digest(row.definition_digest);
  deadlineProfileScope(row.scope, caseId);
  deadlineProfileDefinition(row.definition, caseId);
  if (
    row.algorithm !== 'v1' ||
    !same(row.collection, { kind: 'case', case_id: caseId }) ||
    !same(row.scope, row.definition.scope)
  )
    invalid();
}
function calendar(row) {
  object(row, [
    'id',
    'revision',
    'status',
    'values',
    'values_digest',
    'reason',
    'receipt',
    'recorded_at',
    'recorded_by',
  ]);
  metadata(row);
  digest(row.values_digest);
  if (!same(calendarValues(row.values), row.values)) invalid();
}
function pair(selected, head, reference, policy) {
  if (
    selected.id !== reference.id ||
    selected.revision !== reference.revision ||
    head.id !== selected.id ||
    head.revision < selected.revision ||
    (policy !== 'fixed' && head.revision !== selected.revision) ||
    (head.revision === selected.revision && !same(selected, head))
  )
    invalid();
}
export function derivedDeadlineMaterial(row, command) {
  object(row, [
    'definition',
    'tracking',
    'responsible',
    'profile',
    'profile_head',
    'calendar',
    'calendar_head',
    'result',
  ]);
  const definition = command.deadline.change.definition,
    policies = command.deadline.change.tracking;
  if (!same(row.definition, definition) || !same(row.tracking, policies)) invalid();
  deadlineResponsible(row.responsible);
  if (row.responsible.id !== definition.responsible_id) invalid();
  profile(row.profile, command.case_id);
  profile(row.profile_head, command.case_id);
  pair(row.profile, row.profile_head, definition.profile, policies.profile);
  if (!same(row.profile.scope, row.profile_head.scope)) invalid();
  const trigger = row.profile.definition.trigger;
  if (
    trigger.kind === 'source_field'
      ? trigger.field !== 'hearing_session_event_time'
      : trigger.family !== 'hearing_result'
  )
    invalid();
  if (definition.input.calendar === null) {
    if (row.calendar !== null || row.calendar_head !== null) invalid();
  } else {
    calendar(row.calendar);
    calendar(row.calendar_head);
    pair(row.calendar, row.calendar_head, definition.input.calendar, policies.calendar);
  }
  deadlineResult(row.result);
  if (!same(row.result.requirement, trigger)) invalid();
  return row;
}
export function derivedMaterialMatchesRecord(deadline, ready) {
  const material = ready.deadline,
    p = material.profile,
    c = deadline.calculation;
  if (
    !same(deadline.responsible, material.responsible) ||
    !same(c.result, material.result) ||
    !same(c.profile, {
      id: p.id,
      revision: p.revision,
      algorithm: p.algorithm,
      title: p.definition.title,
      scope: p.scope,
      status: p.status,
      definition_digest: p.definition_digest,
      submission_digest: p.receipt.submission_digest,
      href: `/api/v1/cases/${ready.command.case_id}/deadline-profiles/${p.id}/revisions/${p.revision}`,
    })
  )
    invalid();
  for (const name of ['calendar', 'calendar_head']) {
    const original = material[name],
      captured = c.material[name];
    if (original === null) {
      if (captured !== null) invalid();
    } else if (
      !same(captured, {
        id: original.id,
        revision: original.revision,
        values_digest: original.values_digest,
        submission_digest: original.receipt.submission_digest,
        status: original.status,
        title: original.values.scope.title,
        href: `/api/v1/judicial-calendars/${original.id}/revisions/${original.revision}`,
      })
    )
      invalid();
  }
}
