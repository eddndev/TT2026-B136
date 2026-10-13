import { measureDecisionOperation } from './measure-decision-workflow.mjs';
import { measureRecord, measureCaseId, clone } from './measure-records.mjs';

export { clone };
export const inMeasureCase = (value, caseId) =>
  JSON.parse(JSON.stringify(value).replaceAll(measureCaseId, caseId));
const emptyHistory = () => ({
  records: { judicial: { groups: [] }, administrative: [] },
  decisions: [],
});
const commitment = (number) => number.toString(16).padStart(64, '0');
const supportOf = (state, reference) => ({
  ...clone(reference),
  name: state.support.name,
  format: 'pdf',
  policy: 'pdf_docx_v1',
});

export function seedBrowserMeasure(state) {
  const detail = inMeasureCase(measureRecord(), state.caseId);
  const operation = inMeasureCase(measureDecisionOperation(), state.caseId);
  const { review, decision } = operation.group;
  review.command.values.support = {
    document_id: state.support.id,
    version: state.support.version,
    digest: state.support.digest,
  };
  review.material.support = supportOf(state, review.command.values.support);
  decision.values = clone(review.command.values);
  decision.support = clone(review.material.support);
  detail.record_history.records.judicial.groups = [
    {
      origin: clone(operation.origin),
      capture: clone(operation.group),
    },
  ];
  state.records.set(detail.reference.id, [clone(detail)]);
  state.operations.set(operation.origin.operation_id, clone(operation));
  return { detail, operation };
}

export function prepareBrowserDecision(state, raw) {
  const command = clone(raw);
  if (!state.sequence.has(command.operation_id))
    state.sequence.set(command.operation_id, state.sequence.size + 1);
  const number = state.sequence.get(command.operation_id);
  const predecessors = [],
    results = [];
  function add(id, action, previous, values, effectKey = id) {
    const prior = previous ? state.records.get(previous.id).at(-1) : null;
    if (prior) predecessors.push(clone(prior.record));
    const origin = { operation_id: command.operation_id, decision_id: command.decision_id };
    const subject = prior?.record.capture.result.sources.subject ?? state.subject;
    results.push({
      id,
      revision: (prior?.reference.revision ?? 0) + 1,
      record_root: clone(prior?.record_root ?? { kind: 'judicial', origin }),
      judicial_origin: clone(prior?.judicial_origin ?? origin),
      effect_key: effectKey,
      action,
      previous: clone(previous),
      values: clone(values),
      sources: { subject: clone(subject), supervisor: null },
      projection: {
        subject: {
          case_id: state.caseId,
          id: subject.id,
          revision: subject.revision,
          kind: subject.values.kind,
          display_name: subject.values.name.value,
        },
        supervisor: null,
      },
    });
  }
  const priorValues = (ref) => state.records.get(ref.id).at(-1).record.capture.result.values;
  for (const effect of command.outcome.effects ?? []) {
    if (effect.action === 'impose') add(effect.proposal.id, 'impose', null, effect.proposal.values);
    else if (effect.action === 'substitute') {
      const key = [...effect.predecessors, ...effect.successors].map((row) => row.id).sort()[0];
      for (const previous of effect.predecessors)
        add(previous.id, 'substitute_out', previous, priorValues(previous), key);
      for (const next of effect.successors) add(next.id, 'substitute_in', null, next.values, key);
    } else
      add(
        effect.previous.id,
        effect.action,
        effect.previous,
        effect.action === 'modify' ? effect.values : priorValues(effect.previous),
      );
  }
  results.sort((left, right) => left.id.localeCompare(right.id));
  predecessors.sort((left, right) => left.capture.result.id.localeCompare(right.capture.result.id));
  return {
    family: 'g2',
    review: {
      case_id: state.caseId,
      actor: clone(state.actor),
      command,
      material: {
        context: clone(state.context),
        support: supportOf(state, command.values.support),
        anchor: clone(state.anchorMaterial ?? null),
        predecessors,
        result_sources: results.map((row) => ({ id: row.id, sources: clone(row.sources) })),
      },
      results,
      submission_digest: commitment(100 + number),
      review_digest: commitment(200 + number),
    },
  };
}

export function commitBrowserDecision(state, prepared) {
  const { review } = prepared,
    command = review.command;
  const saved = state.operations.get(command.operation_id);
  if (saved) return clone(saved);
  const number = state.sequence.get(command.operation_id);
  const recordedAt = `2026-10-${String(number + 10).padStart(2, '0')}T12:00:00Z`;
  const decisionDigest = commitment(300 + number),
    groupDigest = commitment(400 + number);
  const origin = {
    case_id: state.caseId,
    operation_id: command.operation_id,
    decision_id: command.decision_id,
    group_digest: groupDigest,
    submission_digest: review.submission_digest,
    review_digest: review.review_digest,
    decision_digest: decisionDigest,
  };
  const history = selectedHistory(state, review.results);
  const measures = review.results.map((result, index) => ({
    family: 'm2',
    case_id: state.caseId,
    result: clone(result),
    operation_id: command.operation_id,
    decision_id: command.decision_id,
    decision_digest: decisionDigest,
    actor: clone(review.actor),
    recorded_at: recordedAt,
    capture_digest: commitment(500 + number * 32 + index),
  }));
  const group = {
    family: 'g2',
    review: clone(review),
    decision: {
      case_id: state.caseId,
      operation_id: command.operation_id,
      decision_id: command.decision_id,
      actor: clone(review.actor),
      context: clone(review.material.context),
      values: clone(command.values),
      support: clone(review.material.support),
      anchor: clone(review.material.anchor),
      recorded_at: recordedAt,
      capture_digest: decisionDigest,
    },
    measures,
    substitutions: (command.outcome.effects ?? [])
      .filter((effect) => effect.action === 'substitute')
      .map((effect) => {
        const reference = (id) => {
          const capture = measures.find((row) => row.result.id === id);
          return { id, revision: capture.result.revision, capture_digest: capture.capture_digest };
        };
        return {
          effect_key: [...effect.predecessors, ...effect.successors].map((row) => row.id).sort()[0],
          predecessors: effect.predecessors.map((previous) => ({
            previous: clone(previous),
            result: reference(previous.id),
          })),
          successors: effect.successors.map((next) => reference(next.id)),
        };
      }),
    recorded_at: recordedAt,
    capture_digest: groupDigest,
  };
  const operation = { family: 'g2', group, origin, record_history: history };
  for (const capture of measures) {
    const result = capture.result;
    const reference = {
      id: result.id,
      revision: result.revision,
      capture_digest: capture.capture_digest,
    };
    const owner = {
      operation_id: command.operation_id,
      decision_id: command.decision_id,
      group_digest: groupDigest,
    };
    const recordHistory = clone(history);
    recordHistory.decisions.push({ origin: clone(origin), capture: clone(group) });
    const detail = {
      case_id: state.caseId,
      reference,
      family: 'm2',
      validity: 'valid',
      last_action: result.action,
      record_root: clone(result.record_root),
      judicial_origin: clone(result.judicial_origin),
      last_judicial: { owner, reference },
      record: { family: 'm2', owner, capture: clone(capture) },
      record_history: recordHistory,
    };
    state.records.set(result.id, [...(state.records.get(result.id) ?? []), detail]);
  }
  state.operations.set(command.operation_id, clone(operation));
  return clone(operation);
}

function selectedHistory(state, results) {
  const history = emptyHistory();
  const collected = [new Map(), new Map(), new Map()];
  for (const result of results) {
    if (!result.previous) continue;
    const prior = state.records.get(result.previous.id).at(-1).record_history;
    [prior.records.judicial.groups, prior.records.administrative, prior.decisions].forEach(
      (rows, index) =>
        rows.forEach((row) => collected[index].set(row.origin.operation_id, clone(row))),
    );
  }
  const rows = collected.map((map) =>
    [...map.values()].sort((a, b) => a.origin.operation_id.localeCompare(b.origin.operation_id)),
  );
  [history.records.judicial.groups, history.records.administrative, history.decisions] = rows;
  return history;
}
