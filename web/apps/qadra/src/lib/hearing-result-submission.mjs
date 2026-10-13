export function hearingResultRequest(prepared) {
  return {
    command: structuredClone(prepared.command),
    expected_submission_digest: prepared.submission_digest,
  };
}
function sameSource(actual, expected, continuation = false) {
  if (!actual || !expected) return actual === null && expected === null;
  const keys = ['hearing_id', 'revision', 'values_digest', 'submission_digest'];
  if (continuation) keys.push('result_id');
  return keys.every((key) => expected[key] !== undefined && actual[key] === expected[key]);
}
export function hearingResultMatches(record, prepared) {
  const command = prepared?.command,
    change = command?.change,
    receipt = record?.receipt;
  if (!command || !change || !receipt || !prepared.anchor || !record.anchor) return false;
  if (
    ![
      prepared.case_id,
      prepared.actor_id,
      prepared.values_digest,
      prepared.submission_digest,
      command.hearing_id,
      command.result_id,
      command.operation_id,
    ].every((value) => typeof value === 'string' && value.length > 0)
  )
    return false;
  if (
    !Number.isInteger(change.expected_revision) ||
    change.expected_revision < 0 ||
    change.expected_revision >= 4294967295
  )
    return false;
  if ((change.action === 'record') !== (change.expected_revision === 0)) return false;
  return (
    ['record', 'correct', 'withdraw'].includes(change.action) &&
    record.case_id === prepared.case_id &&
    record.hearing_id === command.hearing_id &&
    record.id === command.result_id &&
    record.revision === prepared.result_revision &&
    record.revision === change.expected_revision + 1 &&
    record.values_digest === prepared.values_digest &&
    record.status === (change.action === 'withdraw' ? 'withdrawn' : 'recorded') &&
    record.reason === (change.action === 'record' ? null : change.reason) &&
    record.recorded_by?.id === prepared.actor_id &&
    receipt.operation_id === command.operation_id &&
    receipt.action === change.action &&
    receipt.expected_revision === change.expected_revision &&
    receipt.submission_digest === prepared.submission_digest &&
    sameSource(record.anchor, prepared.anchor) &&
    sameSource(record.continuation, prepared.continuation, true)
  );
}
export async function readHearingResultSubmission(api, prepared) {
  let record;
  try {
    record = await api.revision(prepared.command.result_id, prepared.result_revision);
  } catch (failure) {
    if (failure.status === 404 && failure.code === 'hearing_result_not_found')
      return { state: 'absent' };
    throw failure;
  }
  return { state: hearingResultMatches(record, prepared) ? 'matched' : 'different', record };
}
