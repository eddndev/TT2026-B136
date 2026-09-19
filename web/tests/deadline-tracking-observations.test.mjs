import test from 'node:test';
import assert from 'node:assert/strict';
import { deadlineTracking } from '../src/lib/deadline-tracking.mjs';
import { v2Record, technicalRecord, ids } from './fixtures/deadline-v2-unit.mjs';
import { caseId, parseTracking, completeTracking, rejects } from './deadline-tracking-support.mjs';

test('tracking preserves canonical observations without replacing historical parent references', () => {
  for (const value of [v2Record().tracking, technicalRecord().tracking, completeTracking()])
    assert.deepEqual(parseTracking(value), value);
  const value = completeTracking();
  const result = parseTracking(value);
  assert.equal(result.observations.entries[1].parent_resolution.revision, 2);
  assert.equal(result.observations.entries[3].revision, 4);
  value.observations.entries[0].case_id = null;
  assert.deepEqual(parseTracking(value), value);
});

test('tracking preserves present zero UUIDs instead of converting them to absence', () => {
  const value = completeTracking();
  value.observations.case_id = ids(0);
  for (const entry of value.observations.entries) {
    entry.id = ids(0);
    if (entry.case_id !== null) entry.case_id = ids(0);
    if (entry.parent_resolution) entry.parent_resolution.id = ids(0);
  }
  assert.deepEqual(deadlineTracking(value, ids(0)), value);
});

test('tracking rejects missing, extra, malformed and cross-case observations', () => {
  for (const raw of [null, [], {}, 'accepted', 1]) assert.throws(() => parseTracking(raw));
  rejects(completeTracking(), parseTracking, [
    [
      'extra tracking field',
      (v) => {
        v.author = {};
      },
    ],
    [
      'missing administration',
      (v) => {
        delete v.administration;
      },
    ],
    [
      'array policies',
      (v) => {
        v.policies = ['follow', 'follow', 'follow'];
      },
    ],
    [
      'array review',
      (v) => {
        v.review = ['accepted', []];
      },
    ],
    [
      'array observations',
      (v) => {
        v.observations = [caseId, []];
      },
    ],
    [
      'wrong case',
      (v) => {
        v.observations.case_id = ids(9);
      },
    ],
    [
      'empty entries',
      (v) => {
        v.observations.entries = [];
      },
    ],
    [
      'missing profile',
      (v) => {
        v.observations.entries.shift();
      },
    ],
    [
      'duplicate role',
      (v) => {
        v.observations.entries[2] = structuredClone(v.observations.entries[1]);
      },
    ],
    [
      'unsorted roles',
      (v) => {
        v.observations.entries.reverse();
      },
    ],
    [
      'excess entries',
      (v) => {
        v.observations.entries.push(structuredClone(v.observations.entries[3]));
      },
    ],
    [
      'bad role family',
      (v) => {
        v.observations.entries[0].family = 'resolution';
      },
    ],
    [
      'unknown role',
      (v) => {
        v.observations.entries[0].role = 'other';
      },
    ],
    [
      'cross-case profile',
      (v) => {
        v.observations.entries[0].case_id = ids(9);
      },
    ],
    [
      'cross-case source',
      (v) => {
        v.observations.entries[1].case_id = ids(9);
      },
    ],
    [
      'scoped calendar',
      (v) => {
        v.observations.entries[2].case_id = caseId;
      },
    ],
    [
      'hearing on notification',
      (v) => {
        v.observations.entries[1].hearing_id = ids(0);
      },
    ],
    [
      'absent nullable field',
      (v) => {
        delete v.observations.entries[0].hearing_id;
      },
    ],
    [
      'extra entry field',
      (v) => {
        v.observations.entries[0].accepted = true;
      },
    ],
    [
      'malformed digest',
      (v) => {
        v.observations.entries[0].evidence_digest = 'x'.repeat(64);
      },
    ],
    [
      'zero revision',
      (v) => {
        v.observations.entries[0].revision = 0;
      },
    ],
    [
      'overflow revision',
      (v) => {
        v.observations.entries[0].revision = 4294967296;
      },
    ],
    [
      'string revision',
      (v) => {
        v.observations.entries[0].revision = '1';
      },
    ],
  ]);
});

test('notification parents retain identity and ordering while legacy absence remains explicit', () => {
  rejects(completeTracking(), parseTracking, [
    [
      'missing embedded parent',
      (v) => {
        v.observations.entries[1].parent_resolution = null;
      },
    ],
    [
      'parent on profile',
      (v) => {
        v.observations.entries[0].parent_resolution = { id: ids(11), revision: 2 };
      },
    ],
    [
      'parent zero revision',
      (v) => {
        v.observations.entries[1].parent_resolution.revision = 0;
      },
    ],
    [
      'extra parent field',
      (v) => {
        v.observations.entries[1].parent_resolution.current = true;
      },
    ],
    [
      'substituted observed parent',
      (v) => {
        v.observations.entries[3].id = ids(9);
      },
    ],
    [
      'observed parent predates embedded',
      (v) => {
        v.observations.entries[3].revision = 1;
      },
    ],
    [
      'accepted missing observed parent',
      (v) => {
        v.observations.entries.pop();
      },
    ],
  ]);
  const legacy = completeTracking();
  legacy.observations.entries.pop();
  legacy.policies = { profile: 'undetermined', source: 'undetermined', calendar: 'undetermined' };
  legacy.review = { state: 'legacy_undeclared', reasons: [] };
  assert.deepEqual(parseTracking(legacy), legacy);
  assert.equal(legacy.observations.entries.length, 3);
});

test('hearing and resolution observations use their own shape and scope', () => {
  for (const family of ['resolution', 'hearing_result']) {
    const value = completeTracking();
    value.observations.entries.pop();
    const source = value.observations.entries[1];
    Object.assign(source, {
      family,
      parent_resolution: null,
      hearing_id: family === 'hearing_result' ? ids(0) : null,
    });
    assert.deepEqual(parseTracking(value), value);
    source.hearing_id = family === 'hearing_result' ? null : ids(0);
    assert.throws(() => parseTracking(value));
  }
});
