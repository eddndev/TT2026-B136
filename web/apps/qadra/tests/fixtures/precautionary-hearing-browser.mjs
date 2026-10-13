import {
  precautionaryHearingOperation,
  precautionaryCaseId,
  clone,
} from './precautionary-hearing-unit.mjs';

export { clone };
export function precautionaryBrowserRecord(caseId, revision = 1) {
  return JSON.parse(
    JSON.stringify(precautionaryHearingOperation({ revision, participantCount: 0 })).replaceAll(
      precautionaryCaseId,
      caseId,
    ),
  );
}

export function preparePrecautionaryBrowser(state, command) {
  const previous = state.records.get(command.hearing_id)?.at(-1),
    revision = command.change.action === 'schedule' ? 1 : command.change.expected_revision + 1;
  const value = precautionaryBrowserRecord(command.case_id, revision).capture.review;
  value.command = clone(command);
  value.actor = clone(state.actor);
  value.observed_context = clone(state.context);
  if (command.change.action === 'cancel') {
    for (const field of ['resolved_values', 'scheduling_context', 'sources', 'participants'])
      value[field] = clone(previous.capture.review[field]);
  } else {
    value.resolved_values = clone(command.change.values);
    value.scheduling_context = clone(state.context);
    value.sources.support = {
      ...clone(command.change.values.scheduling_basis.support),
      name: state.support.name,
      format: 'pdf',
      policy: 'pdf_docx_v1',
    };
  }
  return value;
}

export function commitPrecautionaryBrowser(state, review) {
  const command = review.command,
    saved = state.operations.get(command.operation_id);
  if (saved) return clone(saved);
  const previous = state.records.get(command.hearing_id)?.at(-1),
    template = precautionaryBrowserRecord(command.case_id, review.result_revision);
  const capture = { ...template.capture, review: clone(review) };
  const first = previous?.history.captures[0] ?? capture;
  const value = {
    capture,
    history: {
      origin: {
        case_id: command.case_id,
        hearing_id: command.hearing_id,
        operation_id: first.review.command.operation_id,
        revision: 1,
        submission_digest: first.review.submission_digest,
        review_digest: first.review.review_digest,
        capture_digest: first.capture_digest,
      },
      captures: [...clone(previous?.history.captures ?? []), clone(capture)],
      record_history: clone(state.recordHistory ?? template.history.record_history),
    },
  };
  state.records.set(command.hearing_id, [
    ...(state.records.get(command.hearing_id) ?? []),
    clone(value),
  ]);
  state.operations.set(command.operation_id, clone(value));
  return clone(value);
}
