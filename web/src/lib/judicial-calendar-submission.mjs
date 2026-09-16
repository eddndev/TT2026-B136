export function calendarRequest(prepared) {
  return {
    command: structuredClone(prepared.command),
    expected_submission_digest: prepared.submission_digest,
  };
}
export function calendarMatches(record, prepared) {
  const command = prepared?.command,
    change = command?.change,
    receipt = record?.receipt;
  if (
    !command ||
    !change ||
    !receipt ||
    ![
      prepared.actor_id,
      prepared.values_digest,
      prepared.submission_digest,
      command.calendar_id,
      command.operation_id,
    ].every((v) => typeof v === 'string' && v.length)
  )
    return false;
  if (
    !Number.isInteger(change.expected_revision) ||
    change.expected_revision < 0 ||
    change.expected_revision >= 4294967295 ||
    (change.action === 'publish') !== (change.expected_revision === 0)
  )
    return false;
  return (
    ['publish', 'replace', 'retire'].includes(change.action) &&
    record.id === command.calendar_id &&
    record.revision === prepared.result_revision &&
    record.revision === change.expected_revision + 1 &&
    record.values_digest === prepared.values_digest &&
    record.status === (change.action === 'retire' ? 'retired' : 'published') &&
    record.reason === (change.action === 'publish' ? null : change.reason) &&
    record.recorded_by?.id === prepared.actor_id &&
    receipt.operation_id === command.operation_id &&
    receipt.action === change.action &&
    receipt.expected_revision === change.expected_revision &&
    receipt.submission_digest === prepared.submission_digest
  );
}
export async function readCalendarSubmission(api, prepared) {
  let record;
  try {
    record = await api.revision(prepared.command.calendar_id, prepared.result_revision);
  } catch (failure) {
    if (failure.status === 404 && failure.code === 'judicial_calendar_not_found')
      return { state: 'absent' };
    throw failure;
  }
  return { state: calendarMatches(record, prepared) ? 'matched' : 'different', record };
}
