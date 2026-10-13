import {
  factObject as object,
  factInvalid as invalid,
  factRevision as revision,
  factText as text,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import {
  resourceHearingUuid as uuid,
  resourceHearingDigest as digest,
  resourceHearingReference as reference,
} from './resource-hearing-values.mjs';
import { factTime } from './procedural-fact-time.mjs';
import { measureKinds } from './measure-presentation.mjs';

function time(value) {
  if (value?.precision === 'unknown') {
    object(value, ['precision', 'reason']);
    value.reason = text(value.reason, 'Motivo');
  } else if (!same(factTime(value), value)) invalid();
}

function measure(value) {
  object(value, ['subject', 'kind', 'conditions', 'validity', 'supervision']);
  object(value.subject, ['id', 'revision', 'values_digest']);
  uuid(value.subject.id);
  revision(value.subject.revision);
  digest(value.subject.values_digest);
  if (!Object.hasOwn(measureKinds, value.kind)) invalid();
  value.conditions = text(value.conditions, 'Condiciones');
  object(value.validity, ['start', 'statement', 'end']);
  time(value.validity.start);
  if (value.validity.end !== null) time(value.validity.end);
  value.validity.statement = text(value.validity.statement, 'Vigencia');
  const supervision = value.supervision;
  if (supervision?.kind === 'unknown') {
    object(supervision, ['kind', 'reason']);
    supervision.reason = text(supervision.reason, 'Motivo de supervision desconocida');
  } else {
    object(supervision, ['kind', 'participant', 'statement']);
    if (supervision.kind !== 'known') invalid();
    object(supervision.participant, ['participant_id', 'revision']);
    uuid(supervision.participant.participant_id);
    revision(supervision.participant.revision);
    supervision.statement = text(supervision.statement, 'Supervision');
  }
}

function proposal(value) {
  object(value, ['id', 'values']);
  uuid(value.id);
  measure(value.values);
  return value.id;
}

export function measureEffectIds(effect) {
  if (effect.action === 'impose') return [effect.proposal.id];
  if (effect.action === 'substitute')
    return [...effect.predecessors.map((row) => row.id), ...effect.successors.map((row) => row.id)];
  return [effect.previous.id];
}

function effect(value) {
  if (value?.action === 'impose') {
    object(value, ['action', 'proposal']);
    proposal(value.proposal);
  } else if (value?.action === 'substitute') {
    object(value, ['action', 'predecessors', 'successors']);
    for (const [rows, parse] of [
      [value.predecessors, reference],
      [value.successors, proposal],
    ]) {
      if (!Array.isArray(rows) || rows.length < 1 || rows.length > 32) invalid();
      rows.forEach((row) => parse(row));
      rows.sort((left, right) => left.id.localeCompare(right.id));
    }
  } else {
    if (!['confirm', 'modify', 'revoke', 'cease'].includes(value?.action)) invalid();
    object(value, ['action', 'previous', ...(value.action === 'modify' ? ['values'] : [])]);
    reference(value.previous);
    if (value.previous.revision === 4294967295) invalid();
    if (value.action === 'modify') measure(value.values);
  }
}

function anchor(value) {
  if (value === null) return;
  if (!['initial', 'precautionary'].includes(value?.kind)) invalid();
  object(value, [
    'kind',
    'hearing_id',
    'revision',
    ...(value.kind === 'initial' ? ['values_digest', 'submission_digest'] : ['capture_digest']),
  ]);
  uuid(value.hearing_id);
  revision(value.revision);
  if (value.kind === 'initial') {
    digest(value.values_digest);
    digest(value.submission_digest);
  } else digest(value.capture_digest);
}

export function measureDecisionCommand(raw) {
  const row = structuredClone(raw);
  object(row, ['case_id', 'operation_id', 'decision_id', 'context', 'values', 'anchor', 'outcome']);
  for (const key of ['case_id', 'operation_id', 'decision_id']) uuid(row[key]);
  object(row.context, ['administration_revision', 'stage_revision', 'context_digest']);
  revision(row.context.administration_revision);
  revision(row.context.stage_revision);
  digest(row.context.context_digest);
  const values = row.values;
  object(values, ['authority', 'declared_at', 'justification', 'support', 'locator']);
  for (const key of ['authority', 'justification', 'locator']) values[key] = text(values[key], key);
  time(values.declared_at);
  object(values.support, ['document_id', 'version', 'digest']);
  uuid(values.support.document_id);
  revision(values.support.version);
  digest(values.support.digest);
  anchor(row.anchor);
  const outcome = row.outcome;
  if (outcome?.kind === 'no_measure_change') {
    object(outcome, ['kind', 'statement']);
    outcome.statement = text(outcome.statement, 'Resultado declarado');
  } else {
    object(outcome, ['kind', 'effects']);
    if (
      outcome.kind !== 'changes' ||
      !Array.isArray(outcome.effects) ||
      outcome.effects.length < 1 ||
      outcome.effects.length > 32
    )
      invalid();
    outcome.effects.forEach(effect);
    const ids = outcome.effects.flatMap(measureEffectIds);
    if (ids.length > 32 || new Set(ids).size !== ids.length) invalid();
    outcome.effects.sort((left, right) =>
      measureEffectIds(left).sort()[0].localeCompare(measureEffectIds(right).sort()[0]),
    );
  }
  return row;
}
