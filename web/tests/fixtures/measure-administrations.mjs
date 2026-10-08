import {
  measureCaseId,
  measureRecord,
  otherMeasureId,
  foreignMeasureId,
  clone,
} from './measure-records.mjs';

export { measureCaseId, foreignMeasureId, clone };
export const administrationBase = `/cases/${measureCaseId}/measure-administrative-operations`;
export const laterOperationId = 'a2000000-0000-4000-8000-000000000004';

export function measureAdministrationOperation({ action = 'correct', operationId } = {}) {
  const detail = measureRecord({
    family: 'c1',
    validity: action === 'correct' ? 'valid' : 'entered_in_error',
  });
  const record_history = clone(detail.record_history);
  const { capture, origin } = record_history.records.administrative.pop();
  if (action === 'correct') {
    const conditions = 'Comparecer en el domicilio corregido del soporte';
    capture.review.command.action.values.conditions = conditions;
    capture.review.result.values.conditions = conditions;
    capture.records[0].result.values.conditions = conditions;
  }
  if (operationId) {
    origin.operation_id = operationId;
    capture.review.command.operation_id = operationId;
    for (const record of capture.records) record.operation_id = operationId;
  }
  if (action === 'replace_entered_in_error') {
    const review = capture.review;
    const replacement = clone(review.result);
    replacement.id = otherMeasureId;
    replacement.revision = 1;
    replacement.validity = 'valid';
    replacement.record_root = {
      kind: 'administrative',
      operation_id: review.command.operation_id,
      measure_id: otherMeasureId,
    };
    const subject = replacement.sources.subject;
    subject.id = 'a4000000-0000-4000-8000-000000000002';
    subject.values.name.value = 'Persona sustituta declarada';
    subject.values_digest = 'a'.repeat(64);
    replacement.values.subject = {
      id: subject.id,
      revision: subject.revision,
      values_digest: subject.values_digest,
    };
    replacement.projection.subject.id = subject.id;
    replacement.projection.subject.display_name = subject.values.name.value;
    review.command.action = {
      kind: action,
      replacement_id: otherMeasureId,
      subject: clone(replacement.values.subject),
    };
    review.replacement = replacement;
    const record = {
      ...clone(capture.records[0]),
      result: clone(replacement),
      capture_digest: 'b'.repeat(64),
    };
    capture.records.push(record);
    const reference = (row) => ({
      id: row.result.id,
      revision: row.result.revision,
      capture_digest: row.capture_digest,
    });
    capture.replacement_link = {
      entered_in_error: reference(capture.records[0]),
      replacement: reference(record),
    };
  }
  return { capture, origin, record_history };
}

export const preparedAdministration = (options) =>
  clone(measureAdministrationOperation(options).capture.review);

export function administrationClient(factory, reply) {
  const calls = [];
  const api = factory(async (path, options) => {
    calls.push({ path, options: clone(options) });
    return typeof reply === 'function' ? reply(path, options) : clone(reply);
  }).caseMeasureAdministrations(measureCaseId);
  return { api, calls };
}
