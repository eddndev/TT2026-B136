import {
  factObject as object,
  factInvalid as invalid,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import {
  resourceHearingDigest as digest,
  resourceHearingUtc as utc,
} from './resource-hearing-values.mjs';
import { measureDecisionPrepared, measureDecisionRows } from './measure-decision-prepared.mjs';

function history(value, family) {
  if (family === 'g1') {
    object(value, ['groups']);
    if (!Array.isArray(value.groups) || value.groups.length >= 256) invalid();
    return;
  }
  object(value, ['records', 'decisions']);
  object(value.records, ['judicial', 'administrative']);
  object(value.records.judicial, ['groups']);
  const rows = [value.records.judicial.groups, value.records.administrative, value.decisions];
  if (
    !rows.every(Array.isArray) ||
    rows.reduce((count, entries) => count + entries.length, 0) >= 256
  )
    invalid();
}

export function measureDecisionOperation(value, caseId, decisionId = null, operationId = null) {
  object(value, [
    'family',
    'group',
    'origin',
    value.family === 'g1' ? 'measure_history' : 'record_history',
  ]);
  const { family, group } = value;
  const prepared = measureDecisionPrepared({ family, review: group?.review });
  const { review } = prepared,
    { command } = review;
  if (
    review.case_id !== caseId ||
    (decisionId !== null && command.decision_id !== decisionId) ||
    (operationId !== null && command.operation_id !== operationId)
  )
    invalid();
  object(group, [
    'family',
    'review',
    'decision',
    'measures',
    'substitutions',
    'recorded_at',
    'capture_digest',
  ]);
  if (group.family !== family) invalid();
  utc(group.recorded_at);
  digest(group.capture_digest);
  const decision = group.decision;
  object(decision, [
    'case_id',
    'operation_id',
    'decision_id',
    'actor',
    'context',
    'values',
    'support',
    'anchor',
    'recorded_at',
    'capture_digest',
  ]);
  digest(decision.capture_digest);
  if (
    decision.case_id !== caseId ||
    decision.operation_id !== command.operation_id ||
    decision.decision_id !== command.decision_id ||
    !same(decision.actor, review.actor) ||
    !same(decision.context, review.material.context) ||
    !same(decision.values, command.values) ||
    !same(decision.support, review.material.support) ||
    !same(decision.anchor, review.material.anchor) ||
    decision.recorded_at !== group.recorded_at
  )
    invalid();
  const expectedOrigin = {
    case_id: caseId,
    operation_id: command.operation_id,
    decision_id: command.decision_id,
    submission_digest: review.submission_digest,
    review_digest: review.review_digest,
    decision_digest: decision.capture_digest,
    group_digest: group.capture_digest,
  };
  if (!same(value.origin, expectedOrigin)) invalid();
  measureDecisionRows(group.measures);
  measureDecisionRows(group.substitutions);
  if (group.measures.length !== review.results.length) invalid();
  for (const [index, capture] of group.measures.entries()) {
    object(capture, [
      'family',
      'case_id',
      'result',
      'operation_id',
      'decision_id',
      'decision_digest',
      'actor',
      'recorded_at',
      'capture_digest',
    ]);
    digest(capture.capture_digest);
    if (
      capture.family !== (family === 'g1' ? 'm1' : 'm2') ||
      capture.case_id !== caseId ||
      capture.operation_id !== command.operation_id ||
      capture.decision_id !== command.decision_id ||
      capture.decision_digest !== decision.capture_digest ||
      !same(capture.actor, review.actor) ||
      capture.recorded_at !== group.recorded_at ||
      !same(capture.result, review.results[index])
    )
      invalid();
  }
  // Owning histories remain server-validated evidence, without browser rehashing.
  history(family === 'g1' ? value.measure_history : value.record_history, family);
  return value;
}
