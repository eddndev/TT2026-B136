import test from 'node:test';
import assert from 'node:assert/strict';
import { hearingOccurrenceMatches } from './resource-activity-alert-origin.mjs';
const scenario = {
  case: { id: 'case' },
  hearingInitial: { id: 'hearing', revision: 1, receipt: { submission_digest: 'a'.repeat(64) } },
  hearing: { id: 'hearing', revision: 2, receipt: { submission_digest: 'b'.repeat(64) } },
};
const alert = (revision) => ({
  subject: { kind: 'hearing', case_id: 'case', id: 'hearing' },
  origin: { revision, evidence_digest: (revision === 1 ? 'a' : 'b').repeat(64) },
  state: { kind: 'active' },
  kind: { kind: 'upcoming', lead_hours: 48 },
});
test('an occurrence activated before a venue-only replacement retains its exact original evidence', () => {
  assert.equal(hearingOccurrenceMatches(alert(1), scenario), true);
});
test('an occurrence activated after replacement retains the exact later evidence', () => {
  assert.equal(hearingOccurrenceMatches(alert(2), scenario), true);
});
test('neither an arbitrary revision nor mismatched scope, digest or window qualifies', () => {
  for (const patch of [
    { origin: { revision: 3, evidence_digest: 'b'.repeat(64) } },
    { origin: { revision: 1, evidence_digest: 'b'.repeat(64) } },
    { subject: { kind: 'hearing', case_id: 'other', id: 'hearing' } },
    { subject: { kind: 'deadline', case_id: 'case', id: 'hearing' } },
    { state: { kind: 'resolved' } },
    { kind: { kind: 'upcoming', lead_hours: 24 } },
  ])
    assert.equal(hearingOccurrenceMatches({ ...alert(1), ...patch }, scenario), false);
});
