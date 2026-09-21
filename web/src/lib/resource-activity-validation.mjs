import {
  resourceActivityCommand,
  resourceActivitySelection,
  resourceActivityReference,
} from './resource-activity-values.mjs';
import { resourceActivitySources, resourceActivityCurrent } from './resource-activity-sources.mjs';
import { factAdministration } from './procedural-fact-validation.mjs';
import {
  factObject as object,
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
  factDigest as digest,
  factSame as same,
  factText as text,
} from './procedural-fact-primitives.mjs';

function actor(value) {
  object(value, ['id', 'email']);
  uuid(value.id);
  if (typeof value.email !== 'string' || !value.email) invalid();
}
function previous(value, expected) {
  if (expected === 0) {
    if (value !== null) invalid();
    return;
  }
  object(value, ['revision', 'capture_digest']);
  revision(value.revision);
  digest(value.capture_digest);
  if (value.revision !== expected) invalid();
}
function context(value, head, caseId, resourceId, expected) {
  factAdministration(value, caseId);
  resourceActivityReference(head);
  if (head.id !== resourceId || head.revision !== expected) invalid();
}
export function resourceActivityDraft(value) {
  object(value, [
    'case_id',
    'resource_id',
    'command',
    'result_revision',
    'selection',
    'status',
    'sources',
    'previous',
    'recorded_by',
    'observed_administration',
    'observed_resource_head',
    'submission_digest',
  ]);
  const command = resourceActivityCommand(value.command),
    change = command.change;
  if (
    !same(command, value.command) ||
    command.case_id !== value.case_id ||
    command.resource_id !== value.resource_id ||
    value.result_revision !== change.expected_revision + 1 ||
    value.status !== (change.action === 'link' ? 'linked' : 'unlinked')
  )
    invalid();
  resourceActivitySelection(value.selection, value.resource_id, command.expected_resource_revision);
  if (
    change.action === 'link' &&
    !same(value.selection, { resource: change.resource, act: change.act, target: change.target })
  )
    invalid();
  previous(value.previous, change.expected_revision);
  actor(value.recorded_by);
  digest(value.submission_digest);
  context(
    value.observed_administration,
    value.observed_resource_head,
    value.case_id,
    value.resource_id,
    command.expected_resource_revision,
  );
  resourceActivitySources(value.sources, value.selection, value.case_id, value.resource_id);
  return value;
}
export function resourceActivityRecord(row, caseId, resourceId, id, exact) {
  object(row, [
    'case_id',
    'resource_id',
    'id',
    'revision',
    'selection',
    'status',
    'reason',
    'sources',
    'receipt',
    'recorded_by',
    'recorded_at',
    'recorded_administration',
    'recorded_resource_head',
  ]);
  if (
    row.case_id !== caseId ||
    row.resource_id !== resourceId ||
    (id !== undefined && row.id !== id) ||
    (exact !== undefined && row.revision !== exact)
  )
    invalid();
  uuid(row.id);
  revision(row.revision);
  actor(row.recorded_by);
  if (
    typeof row.recorded_at !== 'string' ||
    !row.recorded_at.endsWith('Z') ||
    !Number.isFinite(Date.parse(row.recorded_at))
  )
    invalid();
  const receipt = row.receipt;
  object(receipt, [
    'operation_id',
    'action',
    'expected_revision',
    'expected_resource_revision',
    'previous',
    'submission_digest',
    'capture_digest',
  ]);
  uuid(receipt.operation_id);
  digest(receipt.submission_digest);
  digest(receipt.capture_digest);
  revision(receipt.expected_resource_revision);
  if (
    !['link', 'unlink'].includes(receipt.action) ||
    receipt.expected_revision !== row.revision - 1 ||
    (receipt.action === 'link') !== (row.revision === 1) ||
    row.status !== (receipt.action === 'link' ? 'linked' : 'unlinked')
  )
    invalid();
  if (receipt.action === 'unlink') {
    if (text(row.reason, 'Motivo') !== row.reason) invalid();
  } else if (row.reason !== null) invalid();
  previous(receipt.previous, receipt.expected_revision);
  resourceActivitySelection(row.selection, resourceId, receipt.expected_resource_revision);
  context(
    row.recorded_administration,
    row.recorded_resource_head,
    caseId,
    resourceId,
    receipt.expected_resource_revision,
  );
  resourceActivitySources(row.sources, row.selection, caseId, resourceId);
  return row;
}
export function resourceActivityView(value, caseId, resourceId, id, exact) {
  object(value, ['association', 'checked_at', 'current_target']);
  resourceActivityRecord(value.association, caseId, resourceId, id, exact);
  resourceActivityCurrent(value.current_target, value.association, value.checked_at);
  return value;
}
export function resourceActivityMatches(row, draft) {
  try {
    resourceActivityDraft(draft);
    const command = draft.command;
    resourceActivityRecord(
      row,
      draft.case_id,
      draft.resource_id,
      command.association_id,
      draft.result_revision,
    );
    return (
      row.receipt.operation_id === command.operation_id &&
      row.receipt.action === command.change.action &&
      row.receipt.expected_resource_revision === command.expected_resource_revision &&
      row.receipt.submission_digest === draft.submission_digest &&
      same(row.receipt.previous, draft.previous) &&
      same(row.selection, draft.selection) &&
      same(row.sources, draft.sources) &&
      same(row.recorded_by, draft.recorded_by) &&
      same(row.recorded_administration, draft.observed_administration) &&
      same(row.recorded_resource_head, draft.observed_resource_head) &&
      row.reason === (command.change.reason ?? null)
    );
  } catch {
    return false;
  }
}
