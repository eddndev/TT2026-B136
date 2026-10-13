import { factSame as same } from './procedural-fact-primitives.mjs';
import { deadlinePreparedValue, deadlineRecordValue } from './deadline-validation.mjs';
import { deadlineAdministrationFollows } from './deadline-material.mjs';
export function deadlineRequest(prepared) {
  return {
    command: structuredClone(prepared.command),
    expected_submission_digest: prepared.submission_digest,
  };
}
export function deadlineMatches(record, prepared) {
  try {
    deadlinePreparedValue(prepared);
    const command = prepared.command,
      change = command.change;
    deadlineRecordValue(record, prepared.case_id, command.deadline_id, prepared.result_revision);
    const receipt = record.receipt,
      observed = prepared.calculation.material.administration,
      captured = record.calculation.material.administration;
    const preparedCalculation = structuredClone(prepared.calculation),
      recordCalculation = structuredClone(record.calculation),
      preparedTracking = structuredClone(prepared.tracking),
      recordTracking = structuredClone(record.tracking);
    const forward = ['register', 'correct'].includes(change.action);
    if (forward) {
      if (!deadlineAdministrationFollows(captured, observed, prepared.case_id)) return false;
      if (
        !deadlineAdministrationFollows(
          recordTracking.administration,
          preparedTracking.administration,
          prepared.case_id,
        ) ||
        !same(captured, recordTracking.administration) ||
        !same(observed, preparedTracking.administration)
      )
        return false;
      delete preparedCalculation.material.administration;
      delete recordCalculation.material.administration;
      delete preparedTracking.administration;
      delete recordTracking.administration;
    }
    return (
      same(record.recorded_by, prepared.author) &&
      record.status === prepared.status &&
      record.reason === (change.reason ?? null) &&
      receipt.operation_id === command.operation_id &&
      receipt.action === change.action &&
      receipt.expected_revision === change.expected_revision &&
      receipt.review_digest === prepared.review_digest &&
      receipt.submission_digest === prepared.submission_digest &&
      same(receipt.version, prepared.receipt_version) &&
      (forward || receipt.capture_digest === prepared.capture_digest) &&
      (!forward ||
        !same(captured, observed) ||
        receipt.capture_digest === prepared.capture_digest) &&
      same(record.definition, prepared.definition) &&
      same(record.responsible, prepared.responsible) &&
      same(record.attention, prepared.attention) &&
      same(recordCalculation, preparedCalculation) &&
      same(recordTracking, preparedTracking)
    );
  } catch {
    return false;
  }
}
export async function readDeadlineSubmission(api, prepared) {
  deadlinePreparedValue(prepared);
  let record;
  try {
    record = await api.revision(prepared.command.deadline_id, prepared.result_revision);
  } catch (error) {
    if (error.status === 404 && error.code === 'deadline_not_found') return { state: 'absent' };
    throw error;
  }
  return { state: deadlineMatches(record, prepared) ? 'matched' : 'different', record };
}
