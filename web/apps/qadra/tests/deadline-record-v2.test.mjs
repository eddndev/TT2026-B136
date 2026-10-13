import test from 'node:test';
import assert from 'node:assert/strict';
import {
  deadlinePreparedValue,
  deadlineRecordValue,
  deadlineListRow,
} from '../src/lib/deadline-validation.mjs';
import {
  clone,
  v2Prepared,
  v2Record,
  v1Record,
  technicalRecord,
  timedPrepared,
  ids,
  digest,
  instant,
  notChecked,
  summary,
} from './fixtures/deadline-v2-unit.mjs';

const read = (row) => deadlineRecordValue(row, ids(1), ids(6));
const current = (due = null) => ({
  freshness: 'current',
  checked_at: instant(),
  changed_dependencies: [],
  due_at: clone(due),
});

test('reads preserve legacy V1 and complete human or technical V2 evidence', () => {
  for (const row of [v1Record(), v2Record(), technicalRecord()]) {
    assert.deepEqual(read(row), row);
    assert.deepEqual(deadlineRecordValue(row, ids(1), ids(6), row.revision), row);
  }
  assert.equal(v1Record().tracking, null);
  assert.equal(technicalRecord().calculation.profile.revision, 1);
  assert.equal(technicalRecord().tracking.observations.entries[0].revision, 2);
});

test('V2 draft is human and echoes policies only for qualification actions', () => {
  for (const action of ['register', 'correct', 'set_attention', 'retire']) {
    const p = v2Prepared(action);
    assert.deepEqual(deadlinePreparedValue(p), p);
    assert.equal(
      Object.hasOwn(p.command.change, 'tracking'),
      ['register', 'correct'].includes(action),
    );
  }
  for (const mutate of [
    (p) => {
      p.actor_id = ids(9);
    },
    (p) => {
      p.author = technicalRecord().recorded_by;
    },
    (p) => {
      p.receipt_version = { kind: 'v1' };
    },
    (p) => {
      p.receipt_version.cause = technicalRecord().receipt.version.cause;
    },
    (p) => {
      p.tracking = null;
    },
    (p) => {
      p.command.change.tracking.profile = 'fixed';
    },
    (p) => {
      p.receipt_version.predecessor = { submission_digest: digest(), capture_digest: digest() };
    },
  ]) {
    const p = v2Prepared();
    mutate(p);
    assert.throws(() => deadlinePreparedValue(p));
  }
});

test('record context rejects incomplete V2 and contradictory author action or predecessor', () => {
  for (const mutate of [
    (r) => {
      delete r.receipt.version;
    },
    (r) => {
      delete r.tracking;
    },
    (r) => {
      r.tracking = null;
    },
    (r) => {
      r.receipt.version = { kind: 'v1' };
    },
    (r) => {
      r.recorded_by = technicalRecord().recorded_by;
    },
    (r) => {
      r.receipt.version.cause = technicalRecord().receipt.version.cause;
    },
    (r) => {
      r.receipt.version.predecessor = { submission_digest: digest(), capture_digest: digest() };
    },
    (r) => {
      delete r.operational;
    },
  ]) {
    const row = v2Record();
    mutate(row);
    assert.throws(() => read(row));
  }
  for (const mutate of [
    (r) => {
      r.recorded_by = v2Record().recorded_by;
    },
    (r) => {
      r.receipt.version.cause = null;
    },
    (r) => {
      r.receipt.version.predecessor = null;
    },
  ]) {
    const row = technicalRecord();
    mutate(row);
    assert.throws(() => read(row));
  }
  const legacy = v1Record();
  legacy.recorded_by = technicalRecord().recorded_by;
  assert.throws(() => read(legacy));
});

test('tracking roots and presence agree with selection without replacing historical calculation', () => {
  for (const mutate of [
    (r) => {
      r.tracking.observations.entries[0].id = ids(99);
    },
    (r) => {
      r.tracking.observations.entries[0].case_id = null;
    },
    (r) => {
      r.tracking.observations.entries[0].revision = 2;
    },
    (r) => {
      r.tracking.policies.source = 'fixed';
      r.tracking.observations.entries.push({
        role: 'source',
        family: 'resolution',
        id: ids(8),
        revision: 1,
        case_id: ids(1),
        hearing_id: null,
        parent_resolution: null,
        submission_digest: digest('1'),
        evidence_digest: digest('2'),
      });
    },
  ]) {
    const row = v2Record();
    mutate(row);
    assert.throws(() => read(row));
  }
  const row = v2Record(timedPrepared());
  const original = clone(row.calculation);
  row.tracking.observations.entries[1].revision = 2;
  row.tracking.observations.entries[1].submission_digest = digest('6');
  row.tracking.observations.entries[1].evidence_digest = digest('7');
  assert.deepEqual(read(row), row);
  assert.deepEqual(row.calculation, original);
  row.tracking.observations.entries[1].id = ids(99);
  assert.throws(() => read(row));
});

test('operational freshness is server evidence separate from accepted historical calculation', () => {
  const row = v2Record(timedPrepared());
  const historical = clone(row.calculation);
  assert.deepEqual(read(row), row);
  assert.equal(row.operational.due_at, null);
  row.operational = current(row.calculation.result.due_at);
  assert.deepEqual(read(row), row);
  row.operational = {
    freshness: 'changed',
    checked_at: instant(),
    changed_dependencies: ['source'],
    due_at: null,
  };
  assert.deepEqual(read(row), row);
  assert.deepEqual(row.calculation, historical);
  assert.equal(row.tracking.review.state, 'accepted');
});

test('Fixed accepts either authoritative current or changed without deciding source retirement locally', () => {
  const row = v2Record(timedPrepared());
  assert.equal(row.tracking.policies.source, 'fixed');
  row.operational = current(row.calculation.result.due_at);
  assert.deepEqual(read(row), row);
  row.operational = {
    freshness: 'changed',
    checked_at: instant(),
    changed_dependencies: ['source'],
    due_at: null,
  };
  assert.deepEqual(read(row), row);
  assert.equal(row.calculation.material.source.status, 'recorded');
});

test('operational contradictions are rejected without manufacturing a replacement date', () => {
  const due = timedPrepared().calculation.result.due_at;
  for (const operational of [
    { ...notChecked(), due_at: due },
    { ...notChecked(), checked_at: instant() },
    { ...current(due), checked_at: null },
    { ...current(due), changed_dependencies: ['source'] },
    { freshness: 'changed', checked_at: instant(), changed_dependencies: [], due_at: null },
    { freshness: 'changed', checked_at: instant(), changed_dependencies: ['source'], due_at: due },
    {
      freshness: 'changed',
      checked_at: instant(),
      changed_dependencies: ['source', 'source'],
      due_at: null,
    },
    {
      freshness: 'changed',
      checked_at: instant(),
      changed_dependencies: ['calendar', 'profile'],
      due_at: null,
    },
    {
      freshness: 'changed',
      checked_at: instant(),
      changed_dependencies: ['notification_parent'],
      due_at: null,
    },
    { ...current(due), freshness: 'completed' },
    { ...current(due), due_at: { ...due, nanosecond: 1 } },
    { ...notChecked(), inferred: true },
  ]) {
    const row = v2Record(timedPrepared());
    row.operational = operational;
    assert.throws(() => read(row));
  }
  const blocked = v2Record();
  blocked.operational = current(due);
  assert.throws(() => read(blocked));
  const pending = technicalRecord();
  pending.operational = current(due);
  assert.throws(() => read(pending));
  pending.operational = current();
  assert.deepEqual(read(pending), pending);
});

test('exact historical reads never claim operational currentness', () => {
  const row = v2Record(timedPrepared());
  row.operational = current(row.calculation.result.due_at);
  assert.throws(() => deadlineRecordValue(row, ids(1), ids(6), 1));
  row.operational = notChecked();
  assert.deepEqual(deadlineRecordValue(row, ids(1), ids(6), 1), row);
});

test('list distinguishes calculation blocking review and server freshness', () => {
  const timed = v2Record(timedPrepared());
  for (const row of [
    summary(v1Record()),
    summary(v2Record()),
    summary(technicalRecord()),
    summary(timed),
  ])
    assert.doesNotThrow(() => deadlineListRow(row, ids(1)));
  const row = summary(timed);
  assert.equal(row.calculation_blocked, false);
  assert.equal(row.operational.due_at, null);
  row.operational = current(row.calculation_due_at);
  assert.doesNotThrow(() => deadlineListRow(row, ids(1)));
  for (const mutate of [
    (r) => {
      r.calculation_blocked = true;
    },
    (r) => {
      r.review_state = 'pending';
    },
    (r) => {
      r.status = 'retired';
    },
    (r) => {
      r.receipt_kind = 'v1';
    },
    (r) => {
      r.due_at = r.calculation_due_at;
    },
    (r) => {
      r.blocked = false;
    },
  ]) {
    const next = clone(row);
    mutate(next);
    assert.throws(() => deadlineListRow(next, ids(1)));
  }
});
