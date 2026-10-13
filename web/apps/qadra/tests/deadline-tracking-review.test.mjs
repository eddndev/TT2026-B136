import test from 'node:test';
import assert from 'node:assert/strict';
import { v2Record, technicalRecord, ids, digest, instant } from './fixtures/deadline-v2-unit.mjs';
import {
  caseId,
  parseTracking,
  completeTracking,
  rejects,
  reason,
} from './deadline-tracking-support.mjs';

test('policies are explicit for present dependencies and undetermined only when allowed', () => {
  rejects(v2Record().tracking, parseTracking, [
    [
      'missing policy',
      (v) => {
        delete v.policies.source;
      },
    ],
    [
      'unknown policy',
      (v) => {
        v.policies.profile = 'Follow';
      },
    ],
    [
      'extra policy',
      (v) => {
        v.policies.parent = 'follow';
      },
    ],
    [
      'null policy',
      (v) => {
        v.policies.profile = null;
      },
    ],
    [
      'accepted undeclared profile',
      (v) => {
        v.policies.profile = 'undetermined';
      },
    ],
    [
      'declared absent source',
      (v) => {
        v.policies.source = 'fixed';
      },
    ],
    [
      'declared absent calendar',
      (v) => {
        v.policies.calendar = 'follow';
      },
    ],
    [
      'legacy declared profile',
      (v) => {
        v.review.state = 'legacy_undeclared';
      },
    ],
  ]);
  const fixed = completeTracking();
  fixed.policies = { profile: 'fixed', source: 'fixed', calendar: 'fixed' };
  assert.deepEqual(parseTracking(fixed), fixed);
});

test('pending reasons preserve canonical dependency and reason order', () => {
  const value = completeTracking();
  value.review = {
    state: 'pending',
    reasons: [
      reason('profile', 'profile_changed'),
      reason('profile', 'dependency_retired'),
      reason('source', 'source_changed'),
      reason('source', 'dependency_retired'),
      reason('calendar', 'dependency_retired'),
    ],
  };
  assert.deepEqual(parseTracking(value), value);
  rejects(value, parseTracking, [
    [
      'reordered reasons',
      (v) => {
        v.review.reasons.reverse();
      },
    ],
    [
      'duplicate reason',
      (v) => {
        v.review.reasons.splice(1, 0, structuredClone(v.review.reasons[0]));
      },
    ],
    [
      'too many reasons',
      (v) => {
        v.review.reasons = Array(9).fill(reason('profile', 'profile_changed'));
      },
    ],
    [
      'empty pending',
      (v) => {
        v.review.reasons = [];
      },
    ],
    [
      'accepted with reasons',
      (v) => {
        v.review.state = 'accepted';
      },
    ],
    [
      'legacy with reasons',
      (v) => {
        v.review.state = 'legacy_undeclared';
      },
    ],
    [
      'wrong dependency',
      (v) => {
        v.review.reasons[0].dependency = 'calendar';
      },
    ],
    [
      'fixed source changed',
      (v) => {
        v.policies.source = 'fixed';
      },
    ],
    [
      'fixed profile changed',
      (v) => {
        v.policies.profile = 'fixed';
      },
    ],
    [
      'unknown reason',
      (v) => {
        v.review.reasons[0].reason = 'accepted';
      },
    ],
    [
      'extra reason field',
      (v) => {
        v.review.reasons[0].revision = 2;
      },
    ],
  ]);
});

test('pending undeclared policies require their own reasons without inventing absent dependencies', () => {
  const value = completeTracking();
  value.policies = { profile: 'undetermined', source: 'undetermined', calendar: 'undetermined' };
  value.review = {
    state: 'pending',
    reasons: ['profile', 'source', 'calendar'].flatMap((name) => [
      reason(name, 'dependency_retired'),
      reason(name, 'policy_undetermined'),
    ]),
  };
  assert.deepEqual(parseTracking(value), value);
  rejects(value, parseTracking, [
    [
      'missing policy reason',
      (v) => {
        v.review.reasons.splice(1, 1);
      },
    ],
    [
      'policy reason with fixed',
      (v) => {
        v.policies.profile = 'fixed';
      },
    ],
  ]);
  const absent = v2Record().tracking;
  absent.review = { state: 'pending', reasons: [reason('source', 'policy_undetermined')] };
  assert.throws(() => parseTracking(absent));
});

test('captured administration supports closed cases and retains an untagged human author', () => {
  const value = completeTracking();
  value.administration = {
    kind: 'recorded',
    case_id: caseId,
    revision: 2,
    title: 'Expediente',
    reference: null,
    status: 'closed',
    values_digest: digest('f'),
    changed_at: instant(),
    changed_by: { id: ids(0), email: 'owner@example.test' },
  };
  assert.deepEqual(parseTracking(value), value);
  assert.equal(Object.hasOwn(parseTracking(value).administration.changed_by, 'kind'), false);
  rejects(value, parseTracking, [
    [
      'wrong administration case',
      (v) => {
        v.administration.case_id = ids(9);
      },
    ],
    [
      'tagged changed_by',
      (v) => {
        v.administration.changed_by.kind = 'user';
      },
    ],
    [
      'technical changed_by',
      (v) => {
        v.administration.changed_by = technicalRecord().recorded_by;
      },
    ],
    [
      'invalid captured instant',
      (v) => {
        v.administration.changed_at.nanosecond = -1;
      },
    ],
    [
      'unknown administrative status',
      (v) => {
        v.administration.status = 'deleted';
      },
    ],
  ]);
});
