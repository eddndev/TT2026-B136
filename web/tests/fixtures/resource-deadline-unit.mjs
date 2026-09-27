import { activityPrepared, activityRecord, clone } from './resource-activity-unit.mjs';
import { timedPrepared, v2Record } from './deadline-v2-unit.mjs';
export { clone };
export function resourceDeadlinePrepared() {
  const deadline = timedPrepared(),
    existing = activityPrepared('deadline');
  const command = {
    case_id: deadline.case_id,
    resource_id: existing.resource_id,
    association_id: existing.command.association_id,
    expected_resource_revision: existing.command.expected_resource_revision,
    resource: clone(existing.selection.resource),
    act: null,
    deadline: clone(deadline.command),
  };
  const association = {
    command: {
      ...clone(existing.command),
      operation_id: deadline.command.operation_id,
      change: {
        action: 'link',
        expected_revision: 0,
        resource: clone(command.resource),
        act: null,
        target: {
          kind: 'deadline',
          id: deadline.command.deadline_id,
          revision: 1,
          capture_digest: deadline.capture_digest,
        },
      },
    },
    resource: clone(existing.sources.resource),
    act: null,
    recorded_by: { id: deadline.actor_id, email: deadline.author.email },
    observed_administration: clone(deadline.calculation.material.administration),
    observed_resource_head: clone(existing.observed_resource_head),
    submission_digest: existing.submission_digest,
  };
  return { command, deadline, association, submission_digest: '6'.repeat(64) };
}
export function resourceDeadlineResult(draft = resourceDeadlinePrepared()) {
  const deadline = v2Record(draft.deadline),
    a = draft.association;
  const association = activityRecord({
    case_id: draft.command.case_id,
    resource_id: draft.command.resource_id,
    command: clone(a.command),
    result_revision: 1,
    selection: {
      resource: clone(a.command.change.resource),
      act: clone(a.command.change.act),
      target: clone(a.command.change.target),
    },
    status: 'linked',
    sources: {
      resource: clone(a.resource),
      act: clone(a.act),
      target: { kind: 'deadline', record: clone(deadline) },
    },
    previous: null,
    recorded_by: clone(a.recorded_by),
    observed_administration: clone(a.observed_administration),
    observed_resource_head: clone(a.observed_resource_head),
    submission_digest: a.submission_digest,
  });
  return { deadline, association, submission_digest: draft.submission_digest };
}
