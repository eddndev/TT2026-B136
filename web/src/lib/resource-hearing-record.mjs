import { factObject as object, factInvalid as invalid } from './procedural-fact-primitives.mjs';
import {
  resourceHearingDigest as digest,
  resourceHearingUtc as utc,
} from './resource-hearing-values.mjs';
import { resourceHearingCommand } from './resource-hearing-command.mjs';
import { resourceHearingActor, resourceHearingMaterial } from './resource-hearing-capture.mjs';

export function resourceHearingRecord(row, caseId, resourceId) {
  object(row, [
    'case_id',
    'resource_id',
    'id',
    'revision',
    'operation_id',
    'association_id',
    'expected_resource_revision',
    'resource',
    'act',
    'values',
    'sources',
    'recorded_by',
    'recorded_at',
    'recorded_administration',
    'recorded_resource_head',
    'submission_digest',
    'capture_digest',
  ]);
  if (row.case_id !== caseId || row.resource_id !== resourceId || row.revision !== 1) invalid();
  const command = resourceHearingCommand({
    case_id: row.case_id,
    resource_id: row.resource_id,
    hearing_id: row.id,
    operation_id: row.operation_id,
    association_id: row.association_id,
    expected_resource_revision: row.expected_resource_revision,
    resource: row.resource,
    act: row.act,
    values: row.values,
  });
  digest(row.submission_digest);
  digest(row.capture_digest);
  resourceHearingActor(row.recorded_by);
  const recordedAt = utc(row.recorded_at);
  object(row.sources, ['resource', 'act', 'support', 'participants']);
  resourceHearingMaterial(
    command,
    row.sources.resource,
    row.sources.act,
    { support: row.sources.support, participants: row.sources.participants },
    row.recorded_administration,
    row.recorded_resource_head,
  );
  if (
    row.recorded_administration.kind === 'recorded' &&
    recordedAt < utc(row.recorded_administration.changed_at)
  )
    invalid();
  for (const captured of [row.sources.resource, row.sources.act].filter(Boolean)) {
    if (recordedAt < utc(captured.recorded_at)) invalid();
  }
  return row;
}
