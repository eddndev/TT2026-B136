import { v2Prepared, clone, digest, ids } from './deadline-v2-unit.mjs';

export function browserDeadlinePrepared(caseId, action = 'register') {
  return JSON.parse(JSON.stringify(v2Prepared(action)).replaceAll(ids(1), caseId));
}

function profileObservation(value) {
  const profile = value.calculation.profile;
  return {
    role: 'profile',
    family: 'profile',
    id: profile.id,
    revision: profile.revision,
    case_id: profile.scope.kind === 'case' ? value.case_id : null,
    hearing_id: null,
    parent_resolution: null,
    submission_digest: profile.submission_digest,
    evidence_digest: profile.definition_digest,
  };
}

function observations(value) {
  const material = value.calculation.material;
  const entries = [profileObservation(value)];
  const source = material.source_head || material.source;
  if (source) {
    const reference = source.reference;
    entries.push({
      role: 'source',
      family: reference.family,
      id: reference.id || reference.result_id,
      revision: reference.revision,
      case_id: value.case_id,
      hearing_id: reference.hearing_id || null,
      parent_resolution: reference.family === 'notification' ? clone(reference.resolution) : null,
      submission_digest: source.submission_digest,
      evidence_digest: digest('8'),
    });
  }
  const calendar = material.calendar_head || material.calendar;
  if (calendar)
    entries.push({
      role: 'calendar',
      family: 'calendar',
      id: calendar.id,
      revision: calendar.revision,
      case_id: null,
      hearing_id: null,
      parent_resolution: null,
      submission_digest: calendar.submission_digest,
      evidence_digest: calendar.values_digest,
    });
  if (source?.reference.family === 'notification')
    entries.push({
      role: 'notification_parent',
      family: 'resolution',
      ...clone(source.reference.resolution),
      case_id: value.case_id,
      hearing_id: null,
      parent_resolution: null,
      submission_digest: digest('7'),
      evidence_digest: digest('6'),
    });
  return { case_id: value.case_id, entries };
}

// Transport fixtures preserve declared choices; fixed digests do not prove evidence validity.
export function prepareBrowserDeadline(command, current, state, caseId) {
  const action = command.change.action;
  const value = browserDeadlinePrepared(caseId, action);
  value.command = clone(command);
  value.result_revision = command.change.expected_revision + 1;
  value.definition = clone(command.change.definition || current.definition);
  value.attention = clone(command.change.attention || current?.attention || { status: 'pending' });
  value.responsible = clone(
    state.responsibles.find((row) => row.id === value.definition.responsible_id) ||
      current?.responsible,
  );
  if (current) {
    value.calculation = clone(current.calculation);
    value.receipt_version.predecessor = {
      submission_digest: current.receipt.submission_digest,
      capture_digest: current.receipt.capture_digest,
    };
  }
  if (command.change.definition) {
    const profile = state.profiles.find(
      (row) =>
        row.id === value.definition.profile.id &&
        row.revision === value.definition.profile.revision,
    );
    if (!profile) throw new Error('The fixture requires the selected exact profile.');
    value.calculation.profile = {
      id: profile.id,
      revision: profile.revision,
      algorithm: profile.algorithm,
      title: profile.definition.title,
      scope: clone(profile.scope),
      status: profile.status,
      definition_digest: profile.definition_digest,
      submission_digest: profile.receipt.submission_digest,
      href:
        profile.scope.kind === 'case'
          ? `/api/v1/cases/${caseId}/deadline-profiles/${profile.id}/revisions/${profile.revision}`
          : `/api/v1/deadline-profiles/${profile.id}/revisions/${profile.revision}`,
    };
    value.tracking = {
      policies: clone(command.change.tracking),
      review: { state: 'accepted', reasons: [] },
      observations: observations(value),
      administration: clone(value.calculation.material.administration),
    };
  } else {
    value.tracking = current.tracking
      ? clone(current.tracking)
      : {
          policies: { profile: 'undetermined', source: 'undetermined', calendar: 'undetermined' },
          review: { state: 'legacy_undeclared', reasons: [] },
          observations: observations(value),
          administration: clone(value.calculation.material.administration),
        };
  }
  return value;
}
