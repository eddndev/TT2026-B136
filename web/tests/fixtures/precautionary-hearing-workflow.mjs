import {
  precautionaryCaseId,
  precautionaryHearingOperation,
  clone,
} from './precautionary-hearing-unit.mjs';

export { precautionaryCaseId, precautionaryHearingOperation, clone };
export const foreign = 'f0000000-0000-4000-8000-000000000099';
export const hearingBase = `/cases/${precautionaryCaseId}/precautionary-hearings`;
export const preparedHearing = (revision = 1) =>
  clone(precautionaryHearingOperation({ revision }).capture.review);
export const hearingPrincipal = () => clone(preparedHearing().actor);

export function hearingPage(count = 1) {
  const items = Array.from({ length: count }, (_, index) => {
    const operation = precautionaryHearingOperation({ revision: (index % 3) + 1 });
    const id = `f1000000-0000-4000-8000-${String(index + 1).padStart(12, '0')}`;
    for (const capture of operation.history.captures) {
      capture.review.command.hearing_id = id;
      capture.review.command.operation_id = `f2000000-0000-4000-8000-${String(index * 3 + capture.review.result_revision).padStart(12, '0')}`;
    }
    operation.history.origin.hearing_id = id;
    operation.history.origin.operation_id =
      operation.history.captures[0].review.command.operation_id;
    operation.capture = clone(operation.history.captures.at(-1));
    return operation;
  });
  return { case_id: precautionaryCaseId, items, has_more: false, next_after_id: null };
}

export function workflowClient(factory, reply) {
  const calls = [];
  const api = factory(async (path, options) => {
    calls.push({ path, options: clone(options) });
    return typeof reply === 'function' ? reply(path, options) : clone(reply);
  }, precautionaryCaseId);
  return { api, calls };
}
