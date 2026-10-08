import { preparedAdministration } from './measure-administrations.mjs';
import { inMeasureCase, clone } from './measure-decision-browser.mjs';

export { clone };
const commitment = (number) => number.toString(16).padStart(64, '0');
const reference = (capture) => ({
  id: capture.result.id,
  revision: capture.result.revision,
  capture_digest: capture.capture_digest,
});

export function prepareBrowserAdministration(state, raw) {
  const command = clone(raw),
    target = state.records.get(command.target.id).at(-1);
  if (!state.sequence.has(command.operation_id))
    state.sequence.set(command.operation_id, state.sequence.size + 1);
  const number = state.sequence.get(command.operation_id);
  const source = target.record.capture.result;
  const result = {
    id: target.reference.id,
    revision: target.reference.revision + 1,
    previous: clone(target.reference),
    record_root: clone(target.record_root),
    judicial_origin: clone(target.judicial_origin),
    last_judicial: clone(target.last_judicial),
    last_action: target.last_action,
    validity: command.action.kind === 'correct' ? 'valid' : 'entered_in_error',
    values: clone(source.values),
    sources: clone(source.sources),
    projection: clone(source.projection),
  };
  let replacement = null;
  if (command.action.kind === 'correct') {
    const values = command.action.values;
    result.values.conditions = values.conditions;
    result.values.validity = clone(values.validity);
    const field = result.values.supervision.kind === 'known' ? 'statement' : 'reason';
    result.values.supervision[field] = values.supervision_text;
  } else if (command.action.kind === 'replace_entered_in_error') {
    replacement = clone(result);
    replacement.id = command.action.replacement_id;
    replacement.revision = 1;
    replacement.validity = 'valid';
    replacement.record_root = {
      kind: 'administrative',
      operation_id: command.operation_id,
      measure_id: command.action.replacement_id,
    };
    replacement.values.subject = clone(command.action.subject);
    replacement.sources.subject = clone(state.replacementSubject);
    replacement.projection.subject = {
      case_id: state.caseId,
      id: state.replacementSubject.id,
      revision: state.replacementSubject.revision,
      kind: state.replacementSubject.values.kind,
      display_name: state.replacementSubject.values.name.value,
    };
  }
  const review = inMeasureCase(
    preparedAdministration({ action: command.action.kind }),
    state.caseId,
  );
  Object.assign(review, {
    case_id: state.caseId,
    actor: clone(state.actor),
    command,
    context: clone(state.context),
    support: clone(state.support),
    result,
    replacement,
    submission_digest: commitment(1000 + number),
    review_digest: commitment(2000 + number),
  });
  return review;
}

export function commitBrowserAdministration(state, review) {
  const command = review.command,
    saved = state.operations.get(command.operation_id);
  if (saved) return clone(saved);
  const number = state.sequence.get(command.operation_id);
  const recordedAt = `2026-10-${String(number + 20).padStart(2, '0')}T12:00:00Z`;
  const target = state.records.get(command.target.id).at(-1);
  const history = clone(target.record_history);
  const records = [review.result, ...(review.replacement ? [review.replacement] : [])].map(
    (result, index) => ({
      family: 'c1',
      case_id: state.caseId,
      operation_id: command.operation_id,
      result: clone(result),
      actor: clone(review.actor),
      context: clone(review.context),
      support: clone(review.support),
      review_digest: review.review_digest,
      recorded_at: recordedAt,
      capture_digest: commitment(3000 + number * 2 + index),
    }),
  );
  const replacementLink = review.replacement
    ? {
        entered_in_error: reference(records[0]),
        replacement: reference(records[1]),
      }
    : null;
  records.sort((left, right) => left.result.id.localeCompare(right.result.id));
  const capture = {
    family: 'a1',
    review: clone(review),
    records,
    replacement_link: replacementLink,
    recorded_at: recordedAt,
    capture_digest: commitment(4000 + number),
  };
  const origin = {
    case_id: state.caseId,
    operation_id: command.operation_id,
    submission_digest: review.submission_digest,
    review_digest: review.review_digest,
    capture_digest: capture.capture_digest,
  };
  const operation = { capture, origin, record_history: history };
  for (const entry of records) {
    const result = entry.result,
      recordHistory = clone(history);
    recordHistory.records.administrative.push({ origin: clone(origin), capture: clone(capture) });
    const detail = {
      case_id: state.caseId,
      reference: reference(entry),
      family: 'c1',
      validity: result.validity,
      last_action: result.last_action,
      record_root: clone(result.record_root),
      judicial_origin: clone(result.judicial_origin),
      last_judicial: clone(result.last_judicial),
      record: {
        family: 'c1',
        owner: { operation_id: command.operation_id, capture_digest: capture.capture_digest },
        capture: clone(entry),
      },
      record_history: recordHistory,
    };
    state.records.set(result.id, [...(state.records.get(result.id) ?? []), detail]);
  }
  state.operations.set(command.operation_id, clone(operation));
  return clone(operation);
}
