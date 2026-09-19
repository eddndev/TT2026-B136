import {
  factObject as object,
  factInvalid as invalid,
  factUuid as parseUuid,
  factRevision as revision,
  factDigest as parseDigest,
} from './procedural-fact-primitives.mjs';
import { deadlineTrackingScope } from './deadline-tracking-observations.mjs';

function uuid(raw) {
  if (parseUuid(raw).length !== 36) invalid();
}

function digest(raw) {
  if (parseDigest(raw).length !== 64) invalid();
}

export function deadlineAuthorV2(raw) {
  if (raw?.kind === 'user') {
    object(raw, ['kind', 'id', 'email']);
    uuid(raw.id);
    const email = raw.email;
    if (
      typeof email !== 'string' ||
      email.length === 0 ||
      /^\p{White_Space}|\p{White_Space}$/u.test(email) ||
      /[\u0000-\u001f\u007f-\u009f\ud800-\udfff]/u.test(email) ||
      [...email].length > 320
    )
      invalid();
  } else {
    object(raw, ['kind', 'service', 'policy_version']);
    if (
      raw.kind !== 'technical' ||
      raw.service !== 'deadline_reevaluator' ||
      raw.policy_version !== 1
    )
      invalid();
  }
  return raw;
}

function sourceEvent(raw, caseId) {
  object(raw, [
    'sequence',
    'family',
    'source_id',
    'revision',
    'case_id',
    'hearing_id',
    'operation_id',
  ]);
  const sequence = raw.sequence;
  if (
    typeof sequence !== 'string' ||
    sequence.length === 0 ||
    sequence.length > 19 ||
    sequence[0] === '0' ||
    /[^0-9]/.test(sequence) ||
    (sequence.length === 19 && sequence > '9223372036854775807')
  )
    invalid();
  uuid(raw.source_id);
  uuid(raw.operation_id);
  revision(raw.revision);
  deadlineTrackingScope(raw.family, raw.case_id, raw.hearing_id, caseId);
}

function cause(raw, caseId) {
  if (raw?.kind === 'legacy_bootstrap') {
    object(raw, ['kind', 'job_id', 'policy_version']);
    uuid(raw.job_id);
    if (raw.policy_version !== 1) invalid();
  } else {
    object(raw, ['kind', 'job_id', 'event']);
    if (raw.kind !== 'source_event') invalid();
    uuid(raw.job_id);
    sourceEvent(raw.event, caseId);
  }
}

// The enclosing record validates action, author and predecessor continuity.
export function deadlineReceiptVersion(raw, caseId) {
  uuid(caseId);
  if (raw?.kind === 'v1') {
    object(raw, ['kind']);
    return raw;
  }
  object(raw, ['kind', 'observations_digest', 'predecessor', 'cause']);
  if (raw.kind !== 'v2') invalid();
  digest(raw.observations_digest);
  if (raw.predecessor !== null) {
    object(raw.predecessor, ['submission_digest', 'capture_digest']);
    digest(raw.predecessor.submission_digest);
    digest(raw.predecessor.capture_digest);
  }
  if (raw.cause !== null) cause(raw.cause, caseId);
  return raw;
}
