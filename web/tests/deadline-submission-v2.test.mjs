import test from 'node:test';
import assert from 'node:assert/strict';
import {
  deadlineRequest,
  deadlineMatches,
  readDeadlineSubmission,
} from '../src/lib/deadline-submission.mjs';
import {
  clone,
  v2Prepared,
  v2Record,
  v1Record,
  technicalRecord,
  administration,
  ids,
  digest,
} from './fixtures/deadline-v2-unit.mjs';

test('all four human actions send only their reviewed command and digest', () => {
  for (const action of ['register', 'correct', 'set_attention', 'retire']) {
    const p = v2Prepared(action);
    const body = deadlineRequest(p);
    assert.deepEqual(body, { command: p.command, expected_submission_digest: p.submission_digest });
    assert.equal(deadlineMatches(v2Record(p), p), true);
    body.command.operation_id = ids(99);
    assert.notEqual(body.command.operation_id, p.command.operation_id);
  }
});

test('reconciliation binds complete human authorship and tracking receipt context', () => {
  const p = v2Prepared('correct');
  for (const mutate of [
    (r) => {
      r.recorded_by.email = 'other@example.test';
    },
    (r) => {
      r.recorded_by.id = ids(99);
    },
    (r) => {
      r.receipt.version.predecessor.capture_digest = digest('1');
    },
    (r) => {
      r.receipt.version.predecessor.submission_digest = digest('1');
    },
    (r) => {
      r.receipt.version.observations_digest = digest('1');
    },
    (r) => {
      r.tracking.policies.profile = 'fixed';
    },
    (r) => {
      r.tracking.observations.entries[0].evidence_digest = digest('1');
    },
    (r) => {
      r.tracking.observations.entries[0].submission_digest = digest('1');
    },
    (r) => {
      r.receipt.review_digest = digest('1');
    },
    (r) => {
      r.receipt.submission_digest = digest('1');
    },
    (r) => {
      r.receipt.operation_id = ids(99);
    },
  ]) {
    const row = v2Record(p);
    mutate(row);
    assert.equal(deadlineMatches(row, p), false);
  }
  assert.equal(deadlineMatches(v1Record(), v2Prepared()), false);
  assert.equal(deadlineMatches(technicalRecord(), p), false);
});

test('qualification permits the same active administrative advance in both captures', () => {
  for (const action of ['register', 'correct']) {
    const p = v2Prepared(action);
    const row = v2Record(p);
    row.calculation.material.administration = administration(1);
    row.tracking.administration = administration(1);
    row.receipt.capture_digest = digest('1');
    assert.equal(deadlineMatches(row, p), true);
    for (const mutate of [
      (r) => {
        r.tracking.administration = clone(p.tracking.administration);
      },
      (r) => {
        r.calculation.material.administration = clone(p.calculation.material.administration);
      },
      (r) => {
        r.tracking.administration.revision = 2;
      },
      (r) => {
        r.tracking.administration.changed_by.email = 'other@example.test';
      },
      (r) => {
        r.tracking.administration.status = 'closed';
      },
    ]) {
      const next = clone(row);
      mutate(next);
      assert.equal(deadlineMatches(next, p), false);
    }
    const unchanged = v2Record(p);
    unchanged.receipt.capture_digest = digest('1');
    assert.equal(deadlineMatches(unchanged, p), false);
  }
});

test('attention and retirement preserve pending or legacy tracking without acceptance', () => {
  for (const action of ['set_attention', 'retire']) {
    for (const state of ['pending', 'legacy_undeclared']) {
      const p = v2Prepared(action);
      p.tracking.review =
        state === 'pending'
          ? { state, reasons: [{ dependency: 'profile', reason: 'profile_changed' }] }
          : { state, reasons: [] };
      if (state === 'pending') p.tracking.observations.entries[0].revision = 2;
      if (state === 'legacy_undeclared') p.tracking.policies.profile = 'undetermined';
      const row = v2Record(p);
      assert.equal(deadlineMatches(row, p), true);
      assert.equal(Object.hasOwn(deadlineRequest(p).command.change, 'tracking'), false);
      for (const mutate of [
        (r) => {
          r.tracking.review = { state: 'accepted', reasons: [] };
        },
        (r) => {
          r.tracking.administration = administration(1);
        },
        (r) => {
          r.calculation.material.administration = administration(1);
        },
        (r) => {
          r.receipt.capture_digest = digest('1');
        },
      ]) {
        const next = clone(row);
        mutate(next);
        assert.equal(deadlineMatches(next, p), false);
      }
    }
  }
});

test('uncertain submission reads the exact result once and never retries a mutation', async () => {
  const p = v2Prepared('correct');
  const calls = [];
  const api = {
    revision: async (id, revision) => {
      calls.push([id, revision]);
      return v2Record(p);
    },
    submit: async () => {
      assert.fail('A reconciliation must not write');
    },
  };
  assert.equal((await readDeadlineSubmission(api, p)).state, 'matched');
  assert.deepEqual(calls, [[p.command.deadline_id, p.result_revision]]);
  api.revision = async () => technicalRecord();
  assert.equal((await readDeadlineSubmission(api, p)).state, 'different');
  api.revision = async () => {
    throw { status: 404, code: 'deadline_not_found' };
  };
  assert.deepEqual(await readDeadlineSubmission(api, p), { state: 'absent' });
  for (const error of [
    { status: 404, code: 'case_not_found' },
    { status: 401, code: 'invalid_session' },
    { status: 500, code: 'internal_error' },
  ]) {
    api.revision = async () => {
      throw error;
    };
    await assert.rejects(() => readDeadlineSubmission(api, p));
  }
});
