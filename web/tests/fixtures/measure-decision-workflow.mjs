import { measureRecord, measureCaseId, otherMeasureId, clone } from './measure-records.mjs';

export { measureCaseId, clone };
export const foreignDecisionId = 'ffffffff-ffff-4fff-8fff-ffffffffffff';
export const decisionBase = `/cases/${measureCaseId}/measure-decisions`;

export function measureDecisionOperation({ family = 'g1', count = 1, noChanges = false } = {}) {
  const detail = measureRecord({ family: family === 'g1' ? 'm1' : 'm2' });
  const history = clone(detail.record_history);
  const entry = family === 'g1' ? history.records.judicial.groups.pop() : history.decisions.pop();
  const group = entry.capture;
  if (count === 2) {
    const second = measureRecord({ id: otherMeasureId });
    const capture = clone(second.record.capture);
    group.measures.push(capture);
    group.review.results.push(clone(capture.result));
    group.review.material.result_sources.push({
      id: otherMeasureId,
      sources: clone(capture.result.sources),
    });
    group.review.command.outcome.effects.push({
      action: 'impose',
      proposal: { id: otherMeasureId, values: clone(capture.result.values) },
    });
  }
  if (noChanges) {
    group.review.command.outcome = {
      kind: 'no_measure_change',
      statement: 'Sin cambios declarados',
    };
    group.review.results = [];
    group.review.material.result_sources = [];
    group.measures = [];
  }
  return {
    family,
    group,
    origin: entry.origin,
    ...(family === 'g1'
      ? { measure_history: { groups: history.records.judicial.groups } }
      : { record_history: history }),
  };
}

export const preparedDecision = (options) => {
  const operation = measureDecisionOperation(options);
  return { family: operation.family, review: clone(operation.group.review) };
};

export function decisionClient(factory, reply) {
  const calls = [];
  const api = factory(async (path, options) => {
    calls.push({ path, options: clone(options) });
    return typeof reply === 'function' ? reply(path, options) : clone(reply);
  }).caseMeasureDecisions(measureCaseId);
  return { api, calls };
}
