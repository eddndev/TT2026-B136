import { hearingCaseId, hearingId, hearingRecord } from './hearings.mjs';
export const resultId = '50000000-0000-4000-8000-000000000005';
export const resultValues = {
  occurrence: 'occurred',
  extent: 'partial',
  event_time: { precision: 'date', date: '2026-09-01', offset: '-06:00' },
  summary: 'Relato historico declarado',
  attendees: [],
  agreements: [],
  provenance: { kind: 'operator_note', reference: null, support: null },
};
export function resultAnchor(record = hearingRecord()) {
  return {
    hearing_id: record.id,
    revision: record.revision,
    values_digest: record.values_digest,
    submission_digest: record.receipt.submission_digest,
    status: record.status,
    kind: record.values.kind,
    scheduled_at: record.values.scheduled_at,
    scheduling_context: structuredClone(record.scheduling_context),
  };
}
export function resultCommand(action = 'record', expected_revision = 0) {
  return {
    operation_id: '60000000-0000-4000-8000-000000000006',
    hearing_id: hearingId,
    result_id: resultId,
    change: {
      action,
      expected_revision,
      ...(action === 'record'
        ? { anchor_revision: 1, continuation: null }
        : { reason: 'Dato precisado' }),
      ...(action === 'withdraw' ? {} : { values: structuredClone(resultValues) }),
    },
  };
}
export function resultPrepared(
  command = resultCommand(),
  anchor = resultAnchor(),
  base = null,
  continuation = null,
) {
  return {
    case_id: hearingCaseId,
    actor_id: 'user',
    command: structuredClone(command),
    result_revision: command.change.expected_revision + 1,
    values: structuredClone(command.change.values || base?.values || resultValues),
    values_digest: 'd'.repeat(64),
    submission_digest: 'e'.repeat(64),
    anchor: structuredClone(anchor),
    continuation: structuredClone(continuation),
    observed_administration: { revision: 1, values_digest: 'c'.repeat(64), status: 'active' },
    attendees: structuredClone(base?.attendees || []),
    support: base?.support || null,
  };
}
export function resultRecord(prepared = resultPrepared()) {
  return {
    id: prepared.command.result_id,
    case_id: prepared.case_id,
    hearing_id: prepared.command.hearing_id,
    revision: prepared.result_revision,
    status: prepared.command.change.action === 'withdraw' ? 'withdrawn' : 'recorded',
    reason: prepared.command.change.reason || null,
    values: structuredClone(prepared.values),
    values_digest: prepared.values_digest,
    anchor: structuredClone(prepared.anchor),
    continuation: structuredClone(prepared.continuation),
    recorded_administration_revision: 1,
    recorded_administration_digest: 'c'.repeat(64),
    recorded_by: { id: prepared.actor_id, email: 'hatz@example.com' },
    recorded_at: '2026-09-16T12:00:00Z',
    receipt: {
      operation_id: prepared.command.operation_id,
      action: prepared.command.change.action,
      expected_revision: prepared.command.change.expected_revision,
      submission_digest: prepared.submission_digest,
    },
    attendees: structuredClone(prepared.attendees),
    support: prepared.support,
  };
}
export function resultRow(record = resultRecord()) {
  return {
    id: record.id,
    case_id: record.case_id,
    hearing_id: record.hearing_id,
    revision: record.revision,
    status: record.status,
    occurrence: record.values.occurrence,
    extent: record.values.extent,
    event_time: record.values.event_time,
    attendee_count: record.values.attendees.length,
    agreement_count: record.values.agreements.length,
    anchor_revision: record.anchor.revision,
  };
}
export function resultHistoryRow(record) {
  return {
    revision: record.revision,
    action: record.receipt.action,
    status: record.status,
    reason: record.reason,
    values_digest: record.values_digest,
    submission_digest: record.receipt.submission_digest,
    recorded_by: record.recorded_by,
    recorded_at: record.recorded_at,
  };
}
