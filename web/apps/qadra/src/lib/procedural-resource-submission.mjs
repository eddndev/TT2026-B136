import { factSame as same } from './procedural-fact-primitives.mjs';
import { resourcePreparedValue, resourceRecordValue } from './procedural-resource-validation.mjs';
export function resourceRequest(prepared) {
  return {
    command: structuredClone(prepared.command),
    expected_submission_digest: prepared.submission_digest,
  };
}
export function resourceMatches(record, prepared) {
  try {
    resourcePreparedValue(prepared);
    const command = prepared.command,
      change = command.change;
    resourceRecordValue(record, prepared.case_id, command.resource_id, prepared.result_revision);
    return (
      record.receipt.operation_id === command.operation_id &&
      record.receipt.action === change.action &&
      record.receipt.expected_revision === change.expected_revision &&
      record.receipt.submission_digest === prepared.submission_digest &&
      same(record.receipt.previous, prepared.previous) &&
      record.status === prepared.status &&
      record.reason === (change.reason ?? null) &&
      same(record.recorded_by, prepared.recorded_by) &&
      same(record.values, prepared.values) &&
      same(record.sources, prepared.sources) &&
      same(record.act, prepared.act) &&
      same(record.recorded_administration, prepared.observed_administration) &&
      same(record.recorded_stage, prepared.observed_stage)
    );
  } catch {
    return false;
  }
}
export async function readResourceSubmission(api, prepared) {
  let record;
  try {
    record = await api.revision(prepared.command.resource_id, prepared.result_revision);
  } catch (error) {
    if (error.status === 404 && error.code === 'procedural_resource_not_found')
      return { state: 'absent' };
    throw error;
  }
  return { state: resourceMatches(record, prepared) ? 'matched' : 'different', record };
}
