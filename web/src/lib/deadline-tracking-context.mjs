import { factInvalid as invalid, factSame as same } from './procedural-fact-primitives.mjs';
import { deadlineTracking } from './deadline-tracking.mjs';

export function deadlineTrackingContext(raw, definition, calculation, caseId, version, action) {
  if (version.kind === 'v1') {
    if (raw !== null) invalid();
    return 'legacy_undeclared';
  }
  deadlineTracking(raw, caseId);
  const state = raw.review.state;
  if (
    ['register', 'correct'].includes(action) &&
    (state !== 'accepted' ||
      raw.administration.status !== 'active' ||
      !same(raw.administration, calculation.material.administration))
  )
    invalid();
  if (action === 'reevaluate') technicalContext(raw, version.cause);
  const find = (role) => raw.observations.entries.find((entry) => entry.role === role);
  const selectedRevision = (entry, selected, policy) => {
    if (
      entry.revision < selected ||
      (state === 'accepted' && policy === 'follow' && entry.revision !== selected)
    )
      invalid();
  };
  const profile = find('profile');
  const expectedScope = calculation.profile.scope.kind === 'case' ? caseId : null;
  if (profile.id !== definition.profile.id || profile.case_id !== expectedScope) invalid();
  selectedRevision(profile, definition.profile.revision, raw.policies.profile);
  const source = find('source');
  const declared = definition.input.selection.source;
  if (declared.kind === 'unknown') {
    if (source !== undefined) invalid();
  } else {
    const selected = declared.value;
    const hearing = selected.family === 'hearing_result';
    if (
      !source ||
      source.family !== selected.family ||
      source.id !== (hearing ? selected.result_id : selected.id) ||
      source.hearing_id !== (hearing ? selected.hearing_id : null) ||
      (source.parent_resolution?.id ?? null) !==
        (selected.family === 'notification' ? selected.resolution.id : null)
    )
      invalid();
    selectedRevision(source, selected.revision, raw.policies.source);
  }
  const calendar = find('calendar');
  const selected = definition.input.calendar;
  if (selected === null) {
    if (calendar !== undefined) invalid();
  } else {
    if (!calendar || calendar.id !== selected.id) invalid();
    selectedRevision(calendar, selected.revision, raw.policies.calendar);
  }
  return state;
}

function technicalContext(raw, cause) {
  if (raw.review.state === 'legacy_undeclared') invalid();
  if (cause.kind === 'legacy_bootstrap') {
    if (
      raw.review.state !== 'pending' ||
      ['profile', 'source', 'calendar'].some((name) => raw.policies[name] !== 'undetermined')
    )
      invalid();
    return;
  }
  const event = cause.event;
  if (
    !raw.observations.entries.some(
      (entry) =>
        entry.family === event.family &&
        entry.id === event.source_id &&
        entry.case_id === event.case_id &&
        entry.hearing_id === event.hearing_id &&
        entry.revision >= event.revision,
    )
  )
    invalid();
}
