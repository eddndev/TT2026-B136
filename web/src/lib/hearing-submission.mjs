export function hearingRequest(prepared) {
  return {
    command: structuredClone(prepared.command),
    expected_submission_digest: prepared.submission_digest,
  };
}
export function hearingMatches(record, prepared) {
  const command = prepared.command,
    change = command.change,
    receipt = record?.receipt;
  const context = receipt?.expected_context;
  return (
    !!receipt &&
    record.case_id === prepared.case_id &&
    record.id === command.hearing_id &&
    record.revision === prepared.result_revision &&
    record.values_digest === prepared.values_digest &&
    record.status === (change.action === 'cancel' ? 'cancelled' : 'scheduled') &&
    record.recorded_by?.id === prepared.actor_id &&
    receipt.operation_id === command.operation_id &&
    receipt.action === change.action &&
    receipt.expected_revision === change.expected_revision &&
    receipt.submission_digest === prepared.submission_digest &&
    (change.action === 'cancel'
      ? context === null
      : context?.case_revision === change.expected_case_revision &&
        context?.stage_revision === change.expected_stage_revision)
  );
}
export async function readHearingSubmission(api, prepared) {
  let record;
  try {
    record = await api.revision(prepared.command.hearing_id, prepared.result_revision);
  } catch (failure) {
    if (failure.status === 404 && failure.code === 'hearing_not_found') return { state: 'absent' };
    throw failure;
  }
  return { state: hearingMatches(record, prepared) ? 'matched' : 'different', record };
}
