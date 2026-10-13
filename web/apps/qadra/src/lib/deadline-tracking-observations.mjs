import {
  factObject as object,
  factInvalid as invalid,
  factUuid as parseUuid,
  factRevision as revision,
  factDigest as parseDigest,
} from './procedural-fact-primitives.mjs';

const roles = ['profile', 'source', 'calendar', 'notification_parent'];

function uuid(raw) {
  if (parseUuid(raw).length !== 36) invalid();
}

function digest(raw) {
  if (parseDigest(raw).length !== 64) invalid();
}

export function deadlineTrackingScope(family, scopedCase, hearingId, caseId) {
  uuid(caseId);
  if (scopedCase !== null) uuid(scopedCase);
  if (hearingId !== null) uuid(hearingId);
  let valid = false;
  switch (family) {
    case 'profile':
      valid = (scopedCase === null || scopedCase === caseId) && hearingId === null;
      break;
    case 'calendar':
      valid = scopedCase === null && hearingId === null;
      break;
    case 'resolution':
    case 'notification':
      valid = scopedCase === caseId && hearingId === null;
      break;
    case 'hearing_result':
      valid = scopedCase === caseId && hearingId !== null;
      break;
  }
  if (!valid) invalid();
}

function entry(raw, caseId) {
  object(raw, [
    'role',
    'family',
    'id',
    'revision',
    'case_id',
    'hearing_id',
    'parent_resolution',
    'submission_digest',
    'evidence_digest',
  ]);
  uuid(raw.id);
  revision(raw.revision);
  digest(raw.submission_digest);
  digest(raw.evidence_digest);
  deadlineTrackingScope(raw.family, raw.case_id, raw.hearing_id, caseId);
  const validFamily =
    (raw.role === 'profile' && raw.family === 'profile') ||
    (raw.role === 'calendar' && raw.family === 'calendar') ||
    (raw.role === 'notification_parent' && raw.family === 'resolution') ||
    (raw.role === 'source' &&
      ['resolution', 'notification', 'hearing_result'].includes(raw.family));
  if (!validFamily) invalid();
  if (raw.family === 'notification') {
    object(raw.parent_resolution, ['id', 'revision']);
    uuid(raw.parent_resolution.id);
    revision(raw.parent_resolution.revision);
  } else if (raw.parent_resolution !== null) invalid();
}

// Digests are format-checked; no supplied observation is a verified current head.
export function deadlineTrackingObservations(raw, caseId) {
  object(raw, ['case_id', 'entries']);
  uuid(raw.case_id);
  uuid(caseId);
  if (
    raw.case_id !== caseId ||
    !Array.isArray(raw.entries) ||
    raw.entries.length < 1 ||
    raw.entries.length > 4 ||
    raw.entries[0]?.role !== 'profile'
  )
    invalid();
  let previous = -1;
  for (const value of raw.entries) {
    entry(value, caseId);
    const order = roles.indexOf(value.role);
    if (order <= previous) invalid();
    previous = order;
  }
  const parent = raw.entries.find((value) => value.role === 'notification_parent');
  if (parent) {
    const source = raw.entries.find((value) => value.role === 'source');
    const embedded = source?.parent_resolution;
    if (!embedded || embedded.id !== parent.id || embedded.revision > parent.revision) invalid();
  }
  return raw;
}
