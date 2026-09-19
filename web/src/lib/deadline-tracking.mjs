import { factObject as object, factInvalid as invalid } from './procedural-fact-primitives.mjs';
import { deadlineTrackingObservations } from './deadline-tracking-observations.mjs';
import { deadlineTrackingPolicies } from './deadline-tracking-policies.mjs';
import { deadlineTrackingAdministration } from './deadline-material.mjs';

export { deadlineAuthorV2, deadlineReceiptVersion } from './deadline-tracking-receipt.mjs';

const dependencies = ['profile', 'source', 'calendar'];
const reasons = ['source_changed', 'profile_changed', 'dependency_retired', 'policy_undetermined'];

function review(raw) {
  object(raw, ['state', 'reasons']);
  if (
    !['accepted', 'pending', 'legacy_undeclared'].includes(raw.state) ||
    !Array.isArray(raw.reasons) ||
    raw.reasons.length > 8 ||
    (raw.state === 'pending') !== raw.reasons.length > 0
  )
    invalid();
  let previous = -1;
  for (const value of raw.reasons) {
    object(value, ['dependency', 'reason']);
    const dependency = dependencies.indexOf(value.dependency);
    const reason = reasons.indexOf(value.reason);
    if (
      dependency < 0 ||
      reason < 0 ||
      (value.reason === 'source_changed' && value.dependency !== 'source') ||
      (value.reason === 'profile_changed' && value.dependency !== 'profile')
    )
      invalid();
    const order = dependency * reasons.length + reason;
    if (order <= previous) invalid();
    previous = order;
  }
}

export function deadlineTracking(raw, caseId) {
  object(raw, ['policies', 'review', 'observations', 'administration']);
  review(raw.review);
  const observations = deadlineTrackingObservations(raw.observations, caseId);
  const present = (role) => observations.entries.some((value) => value.role === role);
  deadlineTrackingPolicies(
    raw.policies,
    [true, present('source'), present('calendar')],
    raw.review.state,
    raw.review.reasons,
  );
  const source = observations.entries.find((value) => value.role === 'source');
  if (
    raw.review.state === 'accepted' &&
    source?.family === 'notification' &&
    !present('notification_parent')
  )
    invalid();
  deadlineTrackingAdministration(raw.administration, caseId);
  return raw;
}
