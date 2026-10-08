import {
  factObject as object,
  factInvalid as invalid,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import {
  resourceHearingUuid as uuid,
  resourceHearingDigest as digest,
  resourceHearingText as text,
  resourceHearingReference as reference,
} from './resource-hearing-values.mjs';
import { precautionaryHearingPrincipal } from './precautionary-hearing-prepared.mjs';
import { precautionaryHearingContext } from './precautionary-hearing-values.mjs';
import { measureRecordValues } from './measure-record-values.mjs';
import { measureDecisionCommand, measureEffectIds } from './measure-decision-command.mjs';

export function measureDecisionRows(value) {
  if (!Array.isArray(value) || value.length > 32) invalid();
  return value;
}

function anchor(selected, material, caseId) {
  if (selected === null) {
    if (material !== null) invalid();
    return;
  }
  object(material, ['kind', selected.kind === 'initial' ? 'hearing' : 'capture']);
  if (material.kind !== selected.kind) invalid();
  if (selected.kind === 'initial') {
    const row = material.hearing;
    if (
      !row ||
      row.case_id !== caseId ||
      row.id !== selected.hearing_id ||
      row.revision !== selected.revision ||
      row.values_digest !== selected.values_digest ||
      row.receipt?.submission_digest !== selected.submission_digest
    )
      invalid();
  } else {
    const row = material.capture;
    if (
      !row ||
      row.review?.case_id !== caseId ||
      row.review?.command?.hearing_id !== selected.hearing_id ||
      row.review?.result_revision !== selected.revision ||
      row.capture_digest !== selected.capture_digest
    )
      invalid();
  }
}

function previousValues(selected, material, caseId) {
  const rows = material.predecessors.filter((row) => row?.capture?.result?.id === selected.id);
  if (rows.length !== 1) invalid();
  const capture = rows[0].capture;
  if (
    capture.case_id !== caseId ||
    capture.result.revision !== selected.revision ||
    capture.capture_digest !== selected.capture_digest
  )
    invalid();
  return capture.result.values;
}

function expectedResults(command, material) {
  const expected = new Map();
  if (command.outcome.kind === 'no_measure_change') return expected;
  for (const effect of command.outcome.effects) {
    const key = measureEffectIds(effect).sort()[0];
    const add = (id, action, previous, values) =>
      expected.set(id, {
        action,
        previous,
        values,
        revision: previous === null ? 1 : previous.revision + 1,
        effect_key: key,
      });
    if (effect.action === 'impose') {
      add(effect.proposal.id, 'impose', null, effect.proposal.values);
    } else if (effect.action === 'substitute') {
      for (const prior of effect.predecessors)
        add(prior.id, 'substitute_out', prior, previousValues(prior, material, command.case_id));
      for (const next of effect.successors) add(next.id, 'substitute_in', null, next.values);
    } else {
      const values =
        effect.action === 'modify'
          ? effect.values
          : previousValues(effect.previous, material, command.case_id);
      add(effect.previous.id, effect.action, effect.previous, values);
    }
  }
  return expected;
}

export function measureDecisionPrepared(value) {
  object(value, ['family', 'review']);
  if (!['g1', 'g2'].includes(value.family)) invalid();
  const { review } = value;
  object(review, [
    'case_id',
    'actor',
    'command',
    'material',
    'results',
    'submission_digest',
    'review_digest',
  ]);
  uuid(review.case_id);
  precautionaryHearingPrincipal(review.actor);
  digest(review.submission_digest);
  digest(review.review_digest);
  const command = measureDecisionCommand(review.command);
  if (command.case_id !== review.case_id || !same(command, review.command)) invalid();
  const material = review.material;
  object(material, ['context', 'support', 'anchor', 'predecessors', 'result_sources']);
  precautionaryHearingContext(material.context, review.case_id);
  if (!same(command.context, material.context.expectation)) invalid();
  object(material.support, ['document_id', 'version', 'digest', 'name', 'format', 'policy']);
  const support = material.support;
  if (
    !same(command.values.support, {
      document_id: support.document_id,
      version: support.version,
      digest: support.digest,
    }) ||
    !['pdf', 'docx'].includes(support.format) ||
    support.policy !== 'pdf_docx_v1'
  )
    invalid();
  text(support.name, 128, false);
  anchor(command.anchor, material.anchor, review.case_id);
  measureDecisionRows(material.predecessors);
  measureDecisionRows(material.result_sources);
  measureDecisionRows(review.results);
  const expected = expectedResults(command, material);
  if (review.results.length !== expected.size || material.result_sources.length !== expected.size)
    invalid();
  const sources = new Map();
  for (const row of material.result_sources) {
    object(row, ['id', 'sources']);
    uuid(row.id);
    if (sources.has(row.id)) invalid();
    sources.set(row.id, row.sources);
  }
  let previousId = null;
  for (const result of review.results) {
    object(result, [
      'id',
      'revision',
      'effect_key',
      'action',
      'previous',
      'values',
      'sources',
      'projection',
      ...(value.family === 'g1' ? ['origin'] : ['record_root', 'judicial_origin']),
    ]);
    uuid(result.id);
    if (previousId !== null && result.id <= previousId) invalid();
    previousId = result.id;
    const desired = expected.get(result.id);
    if (
      !desired ||
      !Object.keys(desired).every((key) => same(result[key], desired[key])) ||
      !same(sources.get(result.id), result.sources)
    )
      invalid();
    if (result.previous !== null) reference(result.previous);
    measureRecordValues(result, review.case_id);
  }
  return value;
}
