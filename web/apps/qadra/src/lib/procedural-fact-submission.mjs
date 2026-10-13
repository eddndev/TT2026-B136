import { factSame as same } from './procedural-fact-primitives.mjs';
import {
  factPreparedValue,
  factRecordValue,
  factAdministrationFollows,
} from './procedural-fact-validation.mjs';
export function factRequest(prepared) {
  return {
    command: structuredClone(prepared.command),
    expected_submission_digest: prepared.submission_digest,
  };
}
export function factMatches(record, prepared) {
  try {
    factPreparedValue(prepared);
    const command = prepared.command,
      change = command.change;
    factRecordValue(
      record,
      prepared.case_id,
      command.family,
      command.resolution_id,
      command.id,
      prepared.result_revision,
    );
    const receipt = record.receipt;
    return (
      record.recorded_by.id === prepared.actor_id &&
      record.values_digest === prepared.values_digest &&
      record.status === (change.action === 'withdraw' ? 'withdrawn' : 'recorded') &&
      record.reason === (change.reason ?? null) &&
      receipt.operation_id === command.operation_id &&
      receipt.action === change.action &&
      receipt.expected_revision === change.expected_revision &&
      receipt.sources_digest === prepared.sources_digest &&
      receipt.submission_digest === prepared.submission_digest &&
      same(record.values, prepared.values) &&
      same(record.sources, prepared.sources) &&
      factAdministrationFollows(
        record.recorded_administration,
        prepared.observed_administration,
        prepared.case_id,
      )
    );
  } catch {
    return false;
  }
}
export async function readFactSubmission(api, prepared) {
  let record;
  try {
    record = await api.revision(prepared.command.id, prepared.result_revision);
  } catch (error) {
    if (error.status === 404 && error.code === 'procedural_fact_not_found')
      return { state: 'absent' };
    throw error;
  }
  return { state: factMatches(record, prepared) ? 'matched' : 'different', record };
}
