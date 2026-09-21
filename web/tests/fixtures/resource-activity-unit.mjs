import { resourceRecord, resourceActor, resourceCaseId } from './procedural-resource-unit.mjs';
import { hearingRecord, hearingPrepared, hearingCommand } from './hearings.mjs';
import { v2Record, v2Prepared, timedPrepared, ids, instant } from './deadline-v2-unit.mjs';
export const associationId = 'e0000000-0000-4000-8000-000000000001';
export const activityOperationId = 'e0000000-0000-4000-8000-000000000002';
export const clone = (v) => structuredClone(v);
export const activityActor = { ...resourceActor };
export const activityCheckedAt = { ...instant(), offset_seconds: 0 };
export function activityPrepared(kind = 'hearing') {
  const caseId = kind === 'deadline' ? ids(1) : resourceCaseId;
  const resource = JSON.parse(JSON.stringify(resourceRecord()).replaceAll(resourceCaseId, caseId));
  const target = kind === 'hearing' ? hearingRecord() : v2Record(timedPrepared());
  target.case_id = caseId;
  if (kind === 'hearing') target.recorded_by.id = activityActor.id;
  const selection = {
    resource: {
      id: resource.id,
      revision: resource.revision,
      capture_digest: resource.receipt.capture_digest,
    },
    act: null,
    target: {
      kind,
      id: target.id,
      revision: target.revision,
      ...(kind === 'hearing'
        ? { submission_digest: target.receipt.submission_digest }
        : { capture_digest: target.receipt.capture_digest }),
    },
  };
  return {
    case_id: caseId,
    resource_id: resource.id,
    command: {
      case_id: caseId,
      resource_id: resource.id,
      association_id: associationId,
      operation_id: activityOperationId,
      expected_resource_revision: 2,
      change: { action: 'link', expected_revision: 0, ...clone(selection) },
    },
    result_revision: 1,
    selection,
    status: 'linked',
    sources: { resource, act: null, target: { kind, record: target } },
    previous: null,
    recorded_by: clone(activityActor),
    observed_administration: clone(resource.recorded_administration),
    observed_resource_head: { id: resource.id, revision: 2, capture_digest: '9'.repeat(64) },
    submission_digest: '8'.repeat(64),
  };
}
export function activityRecord(draft = activityPrepared()) {
  return {
    case_id: draft.case_id,
    resource_id: draft.resource_id,
    id: draft.command.association_id,
    revision: draft.result_revision,
    selection: clone(draft.selection),
    status: draft.status,
    reason: draft.command.change.reason ?? null,
    sources: clone(draft.sources),
    receipt: {
      operation_id: draft.command.operation_id,
      action: draft.command.change.action,
      expected_revision: draft.command.change.expected_revision,
      expected_resource_revision: draft.command.expected_resource_revision,
      previous: clone(draft.previous),
      submission_digest: draft.submission_digest,
      capture_digest: '7'.repeat(64),
    },
    recorded_by: clone(draft.recorded_by),
    recorded_at: '2026-09-19T10:00:00Z',
    recorded_administration: clone(draft.observed_administration),
    recorded_resource_head: clone(draft.observed_resource_head),
  };
}
export function activityView(draft = activityPrepared()) {
  const association = activityRecord(draft);
  const kind = association.selection.target.kind;
  const current =
    kind === 'hearing'
      ? hearingRecord(hearingPrepared(hearingCommand('replace', 1)))
      : v2Record(v2Prepared('correct'));
  current.case_id = draft.case_id;
  if (kind === 'hearing') {
    current.recorded_by.id = activityActor.id;
    current.values.venue = 'Nueva sala';
    current.receipt.submission_digest = '6'.repeat(64);
  } else
    current.operational = {
      freshness: 'current',
      checked_at: clone(activityCheckedAt),
      changed_dependencies: [],
      due_at: null,
    };
  return {
    association,
    checked_at: clone(activityCheckedAt),
    current_target: { kind, record: current },
  };
}
