import assert from 'node:assert/strict';
import { deadlineReceiptVersion, deadlineTracking } from '../src/lib/deadline-tracking.mjs';
import { v2Record, technicalRecord, ids, digest } from './fixtures/deadline-v2-unit.mjs';

export const caseId = ids(1);
export const parseVersion = (value) => deadlineReceiptVersion(value, caseId);
export const parseTracking = (value) => deadlineTracking(value, caseId);
export function rejects(value, parse, changes) {
  for (const [label, mutate] of changes) {
    const next = structuredClone(value);
    mutate(next);
    const before = structuredClone(next);
    assert.throws(() => parse(next), label);
    assert.deepEqual(next, before, `${label}: rejected evidence must not be repaired`);
  }
}
export function completeTracking() {
  const value = structuredClone(v2Record().tracking);
  const entry = (role, family, id, revision, scope = caseId) => ({
    role,
    family,
    id,
    revision,
    case_id: scope,
    hearing_id: null,
    parent_resolution: null,
    submission_digest: digest('a'),
    evidence_digest: digest('b'),
  });
  value.policies = { profile: 'follow', source: 'follow', calendar: 'follow' };
  value.observations.entries.push(
    {
      ...entry('source', 'notification', ids(10), 3),
      parent_resolution: { id: ids(11), revision: 2 },
    },
    entry('calendar', 'calendar', ids(12), 1, null),
    entry('notification_parent', 'resolution', ids(11), 4),
  );
  return value;
}
export function sourceEventVersion() {
  return structuredClone(technicalRecord().receipt.version);
}
export const reason = (dependency, value) => ({ dependency, reason: value });
