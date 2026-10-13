import test from 'node:test';
import assert from 'node:assert/strict';
import { deadlinePreparedValue, deadlineRecordValue } from '../src/lib/deadline-validation.mjs';
import { deadlineTracking } from '../src/lib/deadline-tracking.mjs';
import { deadlineMatches } from '../src/lib/deadline-submission.mjs';
import {
  clone,
  v2Prepared,
  v2Record,
  technicalRecord,
  timedPrepared,
  administration,
  ids,
  digest,
} from './fixtures/deadline-v2-unit.mjs';

const read = (row) => deadlineRecordValue(row, ids(1), ids(6));

function bootstrap() {
  const row = technicalRecord();
  row.receipt.version.cause = { kind: 'legacy_bootstrap', job_id: ids(0), policy_version: 1 };
  row.tracking.policies.profile = 'undetermined';
  row.tracking.review = {
    state: 'pending',
    reasons: [{ dependency: 'profile', reason: 'policy_undetermined' }],
  };
  return row;
}

function notificationParentEvent() {
  const p = timedPrepared();
  const reference = {
    family: 'notification',
    id: ids(10),
    revision: 1,
    resolution: { id: ids(11), revision: 1 },
  };
  p.definition.input.selection.source = { kind: 'known', value: reference };
  p.calculation.material.source.reference = clone(reference);
  p.calculation.material.source.href = `/api/v1/cases/${ids(1)}/resolutions/${ids(11)}/notifications/${ids(10)}/revisions/1`;
  p.calculation.material.source_head = clone(p.calculation.material.source);
  p.calculation.result.requirement.field = 'notification_practiced_at';
  p.tracking.policies.source = 'follow';
  p.tracking.observations.entries[1] = {
    role: 'source',
    family: 'notification',
    id: ids(10),
    revision: 2,
    case_id: ids(1),
    hearing_id: null,
    parent_resolution: { id: ids(11), revision: 3 },
    submission_digest: digest('1'),
    evidence_digest: digest('2'),
  };
  p.tracking.observations.entries.push({
    role: 'notification_parent',
    family: 'resolution',
    id: ids(11),
    revision: 4,
    case_id: ids(1),
    hearing_id: null,
    parent_resolution: null,
    submission_digest: digest('3'),
    evidence_digest: digest('4'),
  });
  p.tracking.review = {
    state: 'pending',
    reasons: [{ dependency: 'source', reason: 'source_changed' }],
  };
  const row = technicalRecord();
  row.definition = p.definition;
  row.calculation = p.calculation;
  row.tracking = p.tracking;
  Object.assign(row.receipt.version.cause.event, {
    family: 'resolution',
    source_id: ids(11),
    revision: 2,
  });
  return row;
}

test('technical event binds to an observed dependency and may precede the observed head', () => {
  const row = technicalRecord();
  row.receipt.version.cause.event.revision = 1;
  assert.deepEqual(read(row), row);
  for (const mutate of [
    (r) => {
      r.receipt.version.cause.event.source_id = ids(99);
    },
    (r) => {
      r.receipt.version.cause.event.revision = 3;
    },
    (r) => {
      r.receipt.version.cause.event.case_id = null;
    },
    (r) => {
      r.receipt.version.cause.event.family = 'calendar';
      r.receipt.version.cause.event.case_id = null;
    },
    (r) => {
      r.receipt.version.cause.event.family = 'hearing_result';
      r.receipt.version.cause.event.hearing_id = ids(0);
    },
  ]) {
    const next = clone(row);
    mutate(next);
    assert.throws(() => read(next));
  }
});

test('resolution event may bind to the independent notification parent without rewriting history', () => {
  const row = notificationParentEvent();
  assert.deepEqual(read(row), row);
  assert.equal(row.definition.input.selection.source.value.resolution.revision, 1);
  assert.equal(row.tracking.observations.entries[1].parent_resolution.revision, 3);
  assert.equal(row.tracking.observations.entries[2].revision, 4);
  const later = clone(row);
  later.receipt.version.cause.event.revision = 5;
  assert.throws(() => read(later));
  const other = clone(row);
  other.receipt.version.cause.event.source_id = ids(10);
  assert.throws(() => read(other));
});

test('bootstrap is pending with undeclared policies and technical actions never remain legacy', () => {
  const row = bootstrap();
  assert.deepEqual(read(row), row);
  for (const mutate of [
    (r) => {
      r.tracking.review = { state: 'legacy_undeclared', reasons: [] };
    },
    (r) => {
      r.tracking.policies.profile = 'fixed';
      r.tracking.review = { state: 'accepted', reasons: [] };
    },
    (r) => {
      r.tracking.policies.profile = 'follow';
      r.tracking.review = {
        state: 'pending',
        reasons: [{ dependency: 'profile', reason: 'profile_changed' }],
      };
    },
  ]) {
    const next = clone(row);
    mutate(next);
    assert.throws(() => read(next));
  }
  const event = technicalRecord();
  event.tracking.policies.profile = 'undetermined';
  event.tracking.review = { state: 'legacy_undeclared', reasons: [] };
  assert.throws(() => read(event));
});

test('human qualification requires one active administration in calculation and tracking', () => {
  for (const action of ['register', 'correct']) {
    for (const mutate of [
      (p) => {
        p.tracking.administration = administration(1);
      },
      (p) => {
        p.calculation.material.administration = administration(1);
        p.tracking.administration = administration(2);
      },
      (p) => {
        p.tracking.administration = administration(1);
        p.tracking.administration.status = 'closed';
      },
    ]) {
      const p = v2Prepared(action);
      mutate(p);
      assert.throws(() => deadlinePreparedValue(p));
      assert.throws(() => read(v2Record(p)));
    }
  }
});

test('qualification permits coherent parallel active administration advances', () => {
  for (const action of ['register', 'correct']) {
    const p = v2Prepared(action);
    const row = v2Record(p);
    row.calculation.material.administration = administration(2);
    row.tracking.administration = administration(2);
    row.receipt.capture_digest = digest('1');
    assert.deepEqual(read(row), row);
    assert.equal(deadlineMatches(row, p), true);
    p.calculation.material.administration = administration(2);
    p.tracking.administration = administration(2);
    assert.deepEqual(deadlinePreparedValue(p), p);
  }
});

test('attention and retirement inherit captured tracking administration and pending review', () => {
  for (const action of ['set_attention', 'retire']) {
    for (const state of ['pending', 'legacy_undeclared']) {
      const p = v2Prepared(action);
      p.tracking.administration = administration(2);
      p.tracking.administration.status = 'closed';
      p.tracking.review =
        state === 'pending'
          ? { state, reasons: [{ dependency: 'profile', reason: 'profile_changed' }] }
          : { state, reasons: [] };
      if (state === 'legacy_undeclared') p.tracking.policies.profile = 'undetermined';
      else p.tracking.observations.entries[0].revision = 2;
      assert.deepEqual(deadlinePreparedValue(p), p);
      assert.deepEqual(read(v2Record(p)), v2Record(p));
      assert.equal(deadlineMatches(v2Record(p), p), true);
    }
  }
});

test('unrevised administration cannot be closed while a captured recorded closure is readable', () => {
  const tracking = v2Record().tracking;
  tracking.administration.status = 'closed';
  assert.throws(() => deadlineTracking(tracking, ids(1)));
  tracking.administration = administration(1);
  tracking.administration.status = 'closed';
  assert.deepEqual(deadlineTracking(tracking, ids(1)), tracking);
});
