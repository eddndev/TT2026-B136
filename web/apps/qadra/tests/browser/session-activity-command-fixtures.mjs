import { createHash } from 'node:crypto';
import { caseId } from './helpers.mjs';
import { resourceCommandFixture } from '../fixtures/procedural-resource-unit.mjs';
import {
  activityPrepared,
  activityRecord,
  activityView,
  activityCheckedAt,
} from '../fixtures/resource-activity-unit.mjs';
const clone = (value) => structuredClone(value);
const hash = (value) => createHash('sha256').update(JSON.stringify(value)).digest('hex');
export const resourceRef = (row) => ({
  id: row.id,
  revision: row.revision,
  capture_digest: row.receipt.capture_digest,
});
export const targetRef = (kind, row) => ({
  kind,
  id: row.id,
  revision: row.revision,
  ...(kind === 'hearing'
    ? { submission_digest: row.receipt.submission_digest }
    : { capture_digest: row.receipt.capture_digest }),
});

function targets(kind) {
  const value = activityView(activityPrepared(kind));
  const bound = JSON.parse(JSON.stringify(value).replaceAll(value.association.case_id, caseId));
  const rows = [bound.association.sources.target.record, bound.current_target.record];
  if (kind === 'hearing') {
    rows[0].values.venue = 'Sala historica uno';
    rows[1].values.venue = 'Sala actual dos';
  } else {
    rows[0].definition.title = 'Plazo con calculo historico';
    rows[1].definition.title = 'Plazo actual sin fecha operativa';
    rows[1].receipt.operation_id = 'e0000000-0000-4000-8000-000000000098';
    rows[1].receipt.submission_digest = '6'.repeat(64);
    rows[1].receipt.capture_digest = '5'.repeat(64);
  }
  return rows;
}
export function installActivityCommands(state, original) {
  state.hearing = targets('hearing');
  state.deadline = targets('deadline');
  state.hearings.set(state.hearing[0].id, state.hearing);
  state.seedResourceActs = () => {
    const command = resourceCommandFixture('record_act');
    command.resource_id = original.id;
    command.change.expected_revision = 1;
    command.change.values.kind = 'interposition';
    command.change.values.statement = 'Interposicion original declarada';
    command.change.values.evidence = [clone(original.values.resolution_evidence)];
    state.act = state.resourceCommit(state.resourcePrepare(command));
    const correction = resourceCommandFixture('correct_act');
    correction.operation_id = 'c0000000-0000-4000-8000-000000000099';
    correction.resource_id = original.id;
    correction.change.expected_revision = 2;
    correction.change.values = clone(command.change.values);
    correction.change.values.statement = 'Interposicion corregida declarada';
    state.resource = state.resourceCommit(state.resourcePrepare(correction));
  };
  state.activityPrepare = (command) => {
    const previous = state.associations.get(command.association_id)?.at(-1),
      change = command.change,
      history = state.resources.get(command.resource_id);
    const selection =
      change.action === 'link'
        ? { resource: clone(change.resource), act: clone(change.act), target: clone(change.target) }
        : clone(previous.selection);
    const sources =
      change.action === 'link'
        ? {
            resource: clone(history.find((row) => row.revision === selection.resource.revision)),
            act:
              selection.act === null
                ? null
                : clone(history.find((row) => row.revision === selection.act.resource_revision)),
            target: {
              kind: selection.target.kind,
              record: clone(
                state[selection.target.kind].find(
                  (row) =>
                    row.id === selection.target.id && row.revision === selection.target.revision,
                ),
              ),
            },
          }
        : clone(previous.sources);
    const prepared = {
      case_id: caseId,
      resource_id: command.resource_id,
      command: clone(command),
      result_revision: change.expected_revision + 1,
      selection,
      sources,
      status: change.action === 'link' ? 'linked' : 'unlinked',
      previous: previous
        ? { revision: previous.revision, capture_digest: previous.receipt.capture_digest }
        : null,
      recorded_by: { id: state.current.user.id, email: state.current.user.email },
      observed_administration: {
        ...clone(original.recorded_administration),
        revision: state.caseRevision,
        status: state.caseStatus,
      },
      observed_resource_head: resourceRef(history.at(-1)),
      submission_digest: '',
    };
    prepared.submission_digest = hash(prepared);
    return prepared;
  };
  state.activityCommit = (prepared) => {
    const row = activityRecord(prepared);
    row.receipt.capture_digest = hash(row);
    state.associations.set(row.id, [...(state.associations.get(row.id) || []), row]);
    return row;
  };
  state.activityView = (association) => ({
    association: clone(association),
    checked_at: clone(activityCheckedAt),
    current_target: {
      kind: association.selection.target.kind,
      record: clone(state[association.selection.target.kind].at(-1)),
    },
  });
  state.seedAssociation = (kind = 'hearing') =>
    state.activityCommit(
      state.activityPrepare({
        case_id: caseId,
        resource_id: original.id,
        association_id: 'e0000000-0000-4000-8000-000000000001',
        operation_id: 'e0000000-0000-4000-8000-000000000002',
        expected_resource_revision: state.resource.revision,
        change: {
          action: 'link',
          expected_revision: 0,
          resource: resourceRef(original),
          act: null,
          target: targetRef(kind, state[kind][0]),
        },
      }),
    );
}
