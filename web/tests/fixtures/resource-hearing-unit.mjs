import {
  resourceRecord,
  resourcePrepared,
  resourceCommandFixture,
  resourceActor,
} from './procedural-resource-unit.mjs';

export const clone = (value) => structuredClone(value);
export const resourceHearingId = 'f0000000-0000-4000-8000-000000000001';
export const resourceHearingOperationId = 'f0000000-0000-4000-8000-000000000002';
export const resourceHearingAssociationId = 'f0000000-0000-4000-8000-000000000003';

export function resourceHearingCreation({
  withAct = false,
  participantCount = 2,
  kind = 'appeal_arguments',
} = {}) {
  const source = resourceRecord();
  source.recorded_at = '2026-01-01T00:00:00Z';
  source.values.kind = kind === 'appeal_arguments' ? 'appeal' : 'revocation';
  const act = withAct
    ? resourceRecord(resourcePrepared(resourceCommandFixture('record_act')))
    : null;
  if (act) {
    act.recorded_at = source.recorded_at;
    act.values = clone(source.values);
    act.receipt.capture_digest = '6'.repeat(64);
    act.act.values.evidence[0].document_id = 'd0000000-0000-4000-8000-000000000002';
    act.act.values.evidence[0].digest = 'b'.repeat(64);
    act.act.supports[0].document_id = act.act.values.evidence[0].document_id;
    act.act.supports[0].digest = act.act.values.evidence[0].digest;
    act.act.supports[0].name = 'senalamiento-del-acto.pdf';
  }
  const support = clone(act ? act.act.supports[0] : source.sources.supports[0]);
  const participants = Array.from({ length: participantCount }, (_, index) => ({
    case_id: source.case_id,
    id: `60000000-0000-4000-8000-${(index + 1).toString(16).padStart(12, '0')}`,
    revision: index + 1,
    values_digest: '4'.repeat(64),
    directory_status: 'active',
    subject: null,
    display_name: `Persona historica ${index + 1}`,
    procedural_role: 'Testigo',
    organization: null,
    kind: null,
  }));
  const hearing = {
    case_id: source.case_id,
    resource_id: source.id,
    id: resourceHearingId,
    revision: 1,
    operation_id: resourceHearingOperationId,
    association_id: resourceHearingAssociationId,
    expected_resource_revision: 5,
    resource: {
      id: source.id,
      revision: source.revision,
      capture_digest: source.receipt.capture_digest,
    },
    act: act
      ? {
          id: act.act.id,
          revision: act.act.revision,
          resource_revision: act.revision,
          capture_digest: act.receipt.capture_digest,
        }
      : null,
    values: {
      kind,
      scheduled_at: '2026-01-01T18:00:00-06:00',
      modality: 'in_person',
      venue: 'Sala de audiencia del recurso',
      note: 'Programacion declarada por el operador',
      participants: participants.map((row) => ({ participant_id: row.id, revision: row.revision })),
      scheduling_basis: {
        statement: 'Senalamiento declarado con soporte exacto',
        support: {
          document_id: support.document_id,
          version: support.version,
          digest: support.digest,
        },
      },
    },
    sources: { resource: source, act, support, participants },
    recorded_by: clone(resourceActor),
    recorded_at: '2026-01-01T00:00:01.123456789Z',
    recorded_administration: clone(source.recorded_administration),
    recorded_resource_head: { id: source.id, revision: 5, capture_digest: '9'.repeat(64) },
    submission_digest: '8'.repeat(64),
    capture_digest: '7'.repeat(64),
  };
  const association = {
    case_id: hearing.case_id,
    resource_id: hearing.resource_id,
    id: hearing.association_id,
    revision: 1,
    selection: {
      resource: clone(hearing.resource),
      act: clone(hearing.act),
      target: {
        kind: 'resource_hearing',
        id: hearing.id,
        revision: 1,
        capture_digest: hearing.capture_digest,
      },
    },
    status: 'linked',
    reason: null,
    sources: {
      resource: clone(source),
      act: clone(act),
      target: {
        kind: 'resource_hearing',
        record: clone(hearing),
      },
    },
    receipt: {
      operation_id: hearing.operation_id,
      action: 'link',
      expected_revision: 0,
      expected_resource_revision: 5,
      previous: null,
      submission_digest: '5'.repeat(64),
      capture_digest: '3'.repeat(64),
    },
    recorded_by: clone(hearing.recorded_by),
    recorded_at: hearing.recorded_at,
    recorded_administration: clone(hearing.recorded_administration),
    recorded_resource_head: clone(hearing.recorded_resource_head),
  };
  return {
    hearing,
    association,
    origin: {
      case_id: hearing.case_id,
      resource_id: hearing.resource_id,
      hearing_id: hearing.id,
      operation_id: hearing.operation_id,
      association_id: hearing.association_id,
      submission_digest: hearing.submission_digest,
      capture_digest: hearing.capture_digest,
    },
    submission_digest: hearing.submission_digest,
  };
}

export function resourceHearingOverview(value = resourceHearingCreation()) {
  const row = value.hearing;
  return {
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
  };
}

export function resourceHearingAgendaItem(
  value = resourceHearingCreation(),
  caseStatus = 'active',
) {
  return {
    kind: 'resource_hearing',
    at: {
      unix_seconds: Date.parse(value.hearing.values.scheduled_at) / 1000,
      nanosecond: 0,
      offset_seconds: 0,
    },
    case_title: 'Expediente de recurso',
    case_reference: 'RESOURCE-1',
    case_status: caseStatus,
    resource_hearing: resourceHearingOverview(value),
  };
}
