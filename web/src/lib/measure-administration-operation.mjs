import {
  factObject as object,
  factInvalid as invalid,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import {
  resourceHearingDigest as digest,
  resourceHearingUtc as utc,
} from './resource-hearing-values.mjs';
import { measureAdministrationPrepared } from './measure-administration-prepared.mjs';

function history(value) {
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

const reference = (capture) => ({
  id: capture.result.id,
  revision: capture.result.revision,
  capture_digest: capture.capture_digest,
});

export function measureAdministrationOperation(value, caseId, operationId = null) {
  object(value, ['capture', 'origin', 'record_history']);
  const { capture } = value;
  object(capture, [
    'family',
    'review',
    'records',
    'replacement_link',
    'recorded_at',
    'capture_digest',
  ]);
  const review = measureAdministrationPrepared(capture.review),
    { command } = review;
  if (
    capture.family !== 'a1' ||
    review.case_id !== caseId ||
    (operationId !== null && command.operation_id !== operationId)
  )
    invalid();
  utc(capture.recorded_at);
  digest(capture.capture_digest);
  if (
    !same(value.origin, {
      case_id: caseId,
      operation_id: command.operation_id,
      submission_digest: review.submission_digest,
      review_digest: review.review_digest,
      capture_digest: capture.capture_digest,
    })
  )
    invalid();
  const expected = [
    review.result,
    ...(review.replacement === null ? [] : [review.replacement]),
  ].sort((left, right) => left.id.localeCompare(right.id));
  if (!Array.isArray(capture.records) || capture.records.length !== expected.length) invalid();
  for (const [index, record] of capture.records.entries()) {
    object(record, [
      'family',
      'case_id',
      'operation_id',
      'result',
      'actor',
      'context',
      'support',
      'review_digest',
      'recorded_at',
      'capture_digest',
    ]);
    digest(record.capture_digest);
    if (
      record.family !== 'c1' ||
      record.case_id !== caseId ||
      record.operation_id !== command.operation_id ||
      record.review_digest !== review.review_digest ||
      !same(record.actor, review.actor) ||
      !same(record.context, review.context) ||
      !same(record.support, review.support) ||
      !same(record.result, expected[index]) ||
      record.recorded_at !== capture.recorded_at
    )
      invalid();
  }
  if (review.replacement === null) {
    if (capture.replacement_link !== null) invalid();
  } else {
    const original = capture.records.find((row) => row.result.id === review.result.id);
    const replacement = capture.records.find((row) => row.result.id === review.replacement.id);
    if (
      !same(capture.replacement_link, {
        entered_in_error: reference(original),
        replacement: reference(replacement),
      })
    )
      invalid();
  }
  // The authorized server validates owning histories and their cryptographic commitments.
  history(value.record_history);
  return value;
}
