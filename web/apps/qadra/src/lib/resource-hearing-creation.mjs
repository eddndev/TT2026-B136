import {
  factObject as object,
  factInvalid as invalid,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import { resourceHearingRecord } from './resource-hearing-record.mjs';
import { resourceHearingDigest as digest } from './resource-hearing-values.mjs';

export function resourceHearingCreation(value, expected) {
  resourceHearingCreationScope(value, expected.case_id, expected.resource_id);
  const row = value.hearing;
  if (
    !same(expected, {
      case_id: row.case_id,
      resource_id: row.resource_id,
      id: row.id,
      revision: row.revision,
      kind: row.values.kind,
      scheduled_at: row.values.scheduled_at,
      modality: row.values.modality,
      participant_count: row.values.participants.length,
      association_id: row.association_id,
      capture_digest: row.capture_digest,
    })
  )
    invalid('La captura no corresponde a la audiencia seleccionada en Agenda.');
  return value;
}

export function resourceHearingCreationScope(value, caseId, resourceId) {
  object(value, ['hearing', 'association', 'origin', 'submission_digest']);
  const row = resourceHearingRecord(value.hearing, caseId, resourceId);
  if (
    !same(value.origin, {
      case_id: row.case_id,
      resource_id: row.resource_id,
      hearing_id: row.id,
      operation_id: row.operation_id,
      association_id: row.association_id,
      submission_digest: row.submission_digest,
      capture_digest: row.capture_digest,
    }) ||
    value.submission_digest !== row.submission_digest
  )
    invalid();
  const association = value.association;
  object(association, [
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
    association.case_id !== row.case_id ||
    association.resource_id !== row.resource_id ||
    association.id !== row.association_id ||
    association.revision !== 1 ||
    association.status !== 'linked' ||
    association.reason !== null ||
    association.recorded_at !== row.recorded_at ||
    !same(association.recorded_by, row.recorded_by) ||
    !same(association.recorded_administration, row.recorded_administration) ||
    !same(association.recorded_resource_head, row.recorded_resource_head) ||
    !same(association.selection, {
      resource: row.resource,
      act: row.act,
      target: {
        kind: 'resource_hearing',
        id: row.id,
        revision: row.revision,
        capture_digest: row.capture_digest,
      },
    }) ||
    !same(association.sources, {
      resource: row.sources.resource,
      act: row.sources.act,
      target: { kind: 'resource_hearing', record: row },
    })
  )
    invalid('La asociacion original no corresponde a la captura de audiencia.');
  const receipt = association.receipt;
  object(receipt, [
    'operation_id',
    'action',
    'expected_revision',
    'expected_resource_revision',
    'previous',
    'submission_digest',
    'capture_digest',
  ]);
  if (
    receipt.operation_id !== row.operation_id ||
    receipt.action !== 'link' ||
    receipt.expected_revision !== 0 ||
    receipt.previous !== null ||
    receipt.expected_resource_revision !== row.expected_resource_revision
  )
    invalid();
  digest(receipt.submission_digest);
  digest(receipt.capture_digest);
  return value;
}

export function resourceHearingSubmission(value, draft) {
  const command = draft.command;
  resourceHearingCreationScope(value, command.case_id, command.resource_id);
  const row = value.hearing;
  if (
    row.id !== command.hearing_id ||
    row.operation_id !== command.operation_id ||
    row.association_id !== command.association_id ||
    row.expected_resource_revision !== command.expected_resource_revision ||
    row.submission_digest !== draft.submission_digest ||
    !same(row.resource, command.resource) ||
    !same(row.act, command.act) ||
    !same(row.values, command.values) ||
    !same(row.recorded_by, draft.recorded_by) ||
    !same(row.recorded_administration, draft.observed_administration) ||
    !same(row.recorded_resource_head, draft.observed_resource_head) ||
    !same(row.sources, { resource: draft.resource, act: draft.act, ...draft.sources })
  )
    invalid(
      'La captura no corresponde al envio retenido. Conserva el comando para consultar su resultado.',
    );
  return value;
}
