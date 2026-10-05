import {
  factObject as object,
  factInvalid as invalid,
  factRevision as revision,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import {
  resourceHearingUuid as uuid,
  resourceHearingDigest as digest,
  resourceHearingText as text,
  resourceHearingUtc as utc,
} from './resource-hearing-values.mjs';
import {
  precautionaryHearingValues,
  precautionaryHearingSources,
  precautionaryHearingContext,
} from './precautionary-hearing-values.mjs';

function capture(value, caseId, hearingId, expectedRevision) {
  object(value, ['review', 'recorded_at', 'capture_digest']);
  digest(value.capture_digest);
  utc(value.recorded_at);
  const review = value.review;
  object(review, [
    'case_id',
    'actor',
    'command',
    'resolved_values',
    'result_revision',
    'status',
    'scheduling_context',
    'observed_context',
    'sources',
    'participants',
    'submission_digest',
    'review_digest',
  ]);
  object(review.actor, ['id', 'email', 'role']);
  uuid(review.actor.id);
  text(review.actor.email, 320, false);
  if (!['owner', 'litigator'].includes(review.actor.role)) invalid();
  digest(review.submission_digest);
  digest(review.review_digest);
  const command = review.command;
  object(command, ['case_id', 'operation_id', 'hearing_id', 'change']);
  uuid(command.operation_id);
  if (
    review.case_id !== caseId ||
    command.case_id !== caseId ||
    command.hearing_id !== hearingId ||
    review.result_revision !== expectedRevision
  )
    invalid();
  const change = command.change,
    action = change?.action;
  if (!['schedule', 'replace', 'cancel'].includes(action)) invalid();
  object(change, [
    'action',
    ...(action === 'schedule' ? [] : ['expected_revision', 'expected_capture_digest', 'reason']),
    ...(action === 'cancel' ? [] : ['context', 'values']),
  ]);
  if (action === 'schedule') {
    if (expectedRevision !== 1) invalid();
  } else {
    if (change.expected_revision !== expectedRevision - 1 || expectedRevision === 1) invalid();
    digest(change.expected_capture_digest);
    text(change.reason);
  }
  if (review.status !== (action === 'cancel' ? 'cancelled' : 'scheduled')) invalid();
  precautionaryHearingValues(review.resolved_values);
  precautionaryHearingContext(review.scheduling_context, caseId);
  precautionaryHearingContext(review.observed_context, caseId);
  if (
    action !== 'cancel' &&
    (!same(change.values, review.resolved_values) ||
      !same(change.context, review.observed_context.expectation) ||
      !same(review.scheduling_context, review.observed_context))
  )
    invalid();
  precautionaryHearingSources(review);
  return value;
}

function origin(value) {
  return {
    case_id: value.review.case_id,
    hearing_id: value.review.command.hearing_id,
    operation_id: value.review.command.operation_id,
    revision: 1,
    submission_digest: value.review.submission_digest,
    review_digest: value.review.review_digest,
    capture_digest: value.capture_digest,
  };
}

export function precautionaryHearingRecord(value, caseId, hearingId, selectedRevision) {
  uuid(caseId);
  uuid(hearingId);
  revision(selectedRevision);
  object(value, ['capture', 'history']);
  object(value.history, ['origin', 'captures', 'record_history']);
  const rows = value.history.captures;
  if (!Array.isArray(rows) || rows.length !== selectedRevision || rows.length > 256) invalid();
  const operations = new Set();
  for (let index = 0; index < rows.length; index++) {
    const current = capture(rows[index], caseId, hearingId, index + 1);
    const operation = current.review.command.operation_id;
    if (operations.has(operation)) invalid();
    operations.add(operation);
    if (index === 0) continue;
    const previous = rows[index - 1],
      change = current.review.command.change;
    if (
      previous.review.status !== 'scheduled' ||
      change.expected_capture_digest !== previous.capture_digest ||
      utc(current.recorded_at) < utc(previous.recorded_at)
    )
      invalid();
    if (change.action === 'cancel') {
      for (const field of ['resolved_values', 'scheduling_context', 'sources', 'participants']) {
        if (!same(current.review[field], previous.review[field]))
          invalid('La cancelacion no conserva la programacion y sus fuentes historicas.');
      }
    }
  }
  if (!same(value.history.origin, origin(rows[0])) || !same(value.capture, rows.at(-1)))
    invalid('La lectura no conserva el origen y la captura exacta de su historial.');
  // Mixed decision evidence remains opaque; the server verifies its complete ancestry.
  const evidence = value.history.record_history;
  object(evidence, ['records', 'decisions']);
  object(evidence.records, ['judicial', 'administrative']);
  object(evidence.records.judicial, ['groups']);
  if (
    ![evidence.records.judicial.groups, evidence.records.administrative, evidence.decisions].every(
      Array.isArray,
    )
  )
    invalid();
  return value;
}

export function precautionaryHearingRecordOverview(value) {
  const { capture: selected } = value,
    review = selected.review,
    values = review.resolved_values;
  return {
    case_id: review.case_id,
    id: review.command.hearing_id,
    revision: review.result_revision,
    purpose: values.purpose,
    scheduled_at: values.scheduled_at,
    modality: values.modality,
    status: review.status,
    participant_count: values.participants.length,
    capture_digest: selected.capture_digest,
  };
}
