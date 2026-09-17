export const hearingCaseId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
export const hearingId = '10000000-0000-4000-8000-000000000001';
export const operationId = '20000000-0000-4000-8000-000000000002';
export const hearingContext = {
  case_id: hearingCaseId,
  case_revision: 1,
  case_values_digest: 'c'.repeat(64),
  title: 'Defensa inicial',
  reference: 'NUC-123',
  administrative_status: 'active',
  profile_complete: true,
  stage_revision: 1,
  stage: 'investigation',
  stage_values_digest: null,
};
export const hearingValues = {
  kind: 'initial',
  scheduled_at: '2026-10-01T09:02:03-06:00',
  modality: 'in_person',
  venue: 'Sala privada declarada',
  note: 'Nota reservada',
  participants: [],
  conviction_basis: null,
};
export function hearingCommand(action = 'schedule', revision = 0) {
  return {
    operation_id: operationId,
    hearing_id: hearingId,
    change:
      action === 'cancel'
        ? { action, expected_revision: revision, reason: 'Cancelacion declarada' }
        : {
            action,
            expected_revision: revision,
            expected_case_revision: 1,
            expected_stage_revision: 1,
            values: structuredClone(hearingValues),
            ...(action === 'replace' ? { reason: 'Nueva programacion' } : {}),
          },
  };
}
export function hearingPrepared(command = hearingCommand()) {
  return {
    case_id: hearingCaseId,
    actor_id: 'user',
    command: structuredClone(command),
    result_revision: command.change.expected_revision + 1,
    values: structuredClone(command.change.values || hearingValues),
    values_digest: 'a'.repeat(64),
    submission_digest: 'b'.repeat(64),
  };
}
export function hearingRecord(prepared = hearingPrepared()) {
  const change = prepared.command.change;
  return {
    case_id: prepared.case_id,
    id: prepared.command.hearing_id,
    revision: prepared.result_revision,
    values: structuredClone(prepared.values),
    values_digest: prepared.values_digest,
    status: change.action === 'cancel' ? 'cancelled' : 'scheduled',
    reason: change.reason || null,
    receipt: {
      operation_id: prepared.command.operation_id,
      action: change.action,
      expected_revision: change.expected_revision,
      expected_context:
        change.action === 'cancel'
          ? null
          : {
              case_revision: change.expected_case_revision,
              stage_revision: change.expected_stage_revision,
            },
      submission_digest: prepared.submission_digest,
    },
    scheduling_context: {
      administration_revision: 1,
      administration_digest: 'c'.repeat(64),
      stage_revision: 1,
      stage: 'investigation',
      stage_digest: null,
    },
    recorded_administration_revision: 1,
    recorded_administration_digest: 'c'.repeat(64),
    recorded_at: '2026-09-16T12:00:00Z',
    recorded_by: { id: prepared.actor_id, email: 'hatz@example.com' },
    participants: [],
    support: null,
  };
}
export function hearingRow(record = hearingRecord()) {
  return {
    case_id: record.case_id,
    case_title: 'Defensa inicial',
    case_reference: 'NUC-123',
    case_status: 'active',
    id: record.id,
    revision: record.revision,
    kind: record.values.kind,
    scheduled_at: record.values.scheduled_at,
    modality: record.values.modality,
    status: record.status,
    participant_count: record.values.participants.length,
  };
}
