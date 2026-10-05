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
} from './resource-hearing-values.mjs';
import { precautionaryHearingCommand } from './precautionary-hearing-command.mjs';
import {
  precautionaryHearingValues,
  precautionaryHearingSources,
  precautionaryHearingContext,
} from './precautionary-hearing-values.mjs';

export function precautionaryHearingPrincipal(actor) {
  object(actor, ['id', 'email', 'role']);
  uuid(actor.id);
  text(actor.email, 320, false);
  if (!['owner', 'litigator'].includes(actor.role)) invalid();
  return actor;
}

export function precautionaryHearingPrepared(review) {
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
  uuid(review.case_id);
  revision(review.result_revision);
  precautionaryHearingPrincipal(review.actor);
  digest(review.submission_digest);
  digest(review.review_digest);
  const command = precautionaryHearingCommand(review.command);
  if (!same(command, review.command) || command.case_id !== review.case_id) invalid();
  const { change } = command,
    { action } = change;
  const expectedRevision = action === 'schedule' ? 1 : change.expected_revision + 1;
  if (
    review.result_revision !== expectedRevision ||
    review.status !== (action === 'cancel' ? 'cancelled' : 'scheduled')
  )
    invalid();
  precautionaryHearingValues(review.resolved_values);
  precautionaryHearingContext(review.scheduling_context, review.case_id);
  precautionaryHearingContext(review.observed_context, review.case_id);
  if (
    action !== 'cancel' &&
    (!same(change.values, review.resolved_values) ||
      !same(change.context, review.observed_context.expectation) ||
      !same(review.scheduling_context, review.observed_context))
  )
    invalid();
  precautionaryHearingSources(review);
  return review;
}
