import { createHash } from 'node:crypto';
import { caseId } from './helpers.mjs';
import { prepareBrowserDeadline } from '../fixtures/deadline-browser.mjs';
import { resourceDeadlineResult } from '../fixtures/resource-deadline-unit.mjs';
import { resourceRef } from './session-activity-command-fixtures.mjs';

const clone = (value) => structuredClone(value);
const hash = (value) => createHash('sha256').update(JSON.stringify(value)).digest('hex');

function deadlineAdministration(recorded) {
  if (recorded.kind !== 'recorded') return clone(recorded);
  const [seconds, fraction = ''] = recorded.changed_at.replace(/Z$/, '').split('.');
  return {
    ...clone(recorded),
    changed_at: {
      unix_seconds: Date.parse(`${seconds}Z`) / 1000,
      nanosecond: Number(fraction.padEnd(9, '0')),
      offset_seconds: 0,
    },
  };
}

// These transport fixtures bind actors and receipts; they do not prove cryptographic validity.
export function installResourceDeadlineCommands(state) {
  state.compositePrepare = (command) => {
    const history = state.resources.get(command.resource_id);
    const observed = {
      ...clone(state.original.recorded_administration),
      revision: state.caseRevision,
      status: state.caseStatus,
      changed_by: { id: state.current.user.id, email: state.current.user.email },
    };
    const deadline = prepareBrowserDeadline(command.deadline, null, state, caseId);
    deadline.actor_id = state.current.user.id;
    deadline.author = { kind: 'user', id: state.current.user.id, email: state.current.user.email };
    deadline.calculation.material.administration = deadlineAdministration(observed);
    deadline.tracking.administration = clone(deadline.calculation.material.administration);
    deadline.review_digest = hash(deadline.calculation);
    deadline.capture_digest = hash({ command: command.deadline, definition: deadline.definition });
    deadline.submission_digest = hash(deadline);
    const association = {
      command: {
        case_id: caseId,
        resource_id: command.resource_id,
        association_id: command.association_id,
        operation_id: command.deadline.operation_id,
        expected_resource_revision: command.expected_resource_revision,
        change: {
          action: 'link',
          expected_revision: 0,
          resource: clone(command.resource),
          act: clone(command.act),
          target: {
            kind: 'deadline',
            id: command.deadline.deadline_id,
            revision: 1,
            capture_digest: deadline.capture_digest,
          },
        },
      },
      resource: clone(history.find((row) => row.revision === command.resource.revision)),
      act: command.act
        ? clone(history.find((row) => row.revision === command.act.resource_revision))
        : null,
      recorded_by: { id: deadline.actor_id, email: deadline.author.email },
      observed_administration: observed,
      observed_resource_head: resourceRef(history.at(-1)),
      submission_digest: '',
    };
    association.submission_digest = hash(association);
    const draft = { command: clone(command), deadline, association, submission_digest: '' };
    draft.submission_digest = hash(draft);
    state.compositeDrafts.set(command.deadline.operation_id, clone(draft));
    return draft;
  };
  state.publishCompositeReceipts = (draft, { deadline = true, association = true } = {}) => {
    const result = resourceDeadlineResult(draft);
    if (deadline) {
      state.deadlineRecords.set(result.deadline.id, [clone(result.deadline)]);
      state.deadline = [clone(result.deadline)];
    }
    if (association) state.associations.set(result.association.id, [clone(result.association)]);
    return result;
  };
  state.compositeCommit = (draft) => {
    const previous = state.jointOperations.get(draft.command.deadline.operation_id);
    if (previous) return clone(previous.result);
    const result = state.publishCompositeReceipts(draft);
    state.jointOperations.set(draft.command.deadline.operation_id, {
      draft: clone(draft),
      result: clone(result),
    });
    return result;
  };
}
