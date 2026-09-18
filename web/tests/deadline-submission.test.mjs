import test from 'node:test';
import assert from 'node:assert/strict';
import { deadlineMatches, readDeadlineSubmission } from '../src/lib/deadline-submission.mjs';
import { prepared, detail, administration, id, hash, known } from './fixtures/deadline-unit.mjs';

test('register and correction accept forward administration without changing reviewed content', () => {
  for (const action of ['register', 'correct']) {
    const p = prepared(action),
      d = detail(p);
    assert.equal(deadlineMatches(d, p), true);
    d.calculation.material.administration = administration();
    d.receipt.capture_digest = hash('1');
    assert.equal(deadlineMatches(d, p), true);
    p.calculation.material.administration = administration(2);
    assert.equal(deadlineMatches(d, p), false);
    d.calculation.material.administration = administration(2);
    d.calculation.material.administration.changed_at.offset_seconds = 0;
    assert.equal(deadlineMatches(d, p), false);
  }
});
test('attention and retirement require exact captured calculation and digests', () => {
  for (const action of ['set_attention', 'retire']) {
    const p = prepared(action),
      d = detail(p);
    assert.equal(deadlineMatches(d, p), true);
    d.calculation.material.administration = administration();
    assert.equal(deadlineMatches(d, p), false);
    d.calculation = structuredClone(p.calculation);
    d.receipt.capture_digest = hash('1');
    assert.equal(deadlineMatches(d, p), false);
  }
});
test('reconciliation rejects result, profile, head, actor and receipt substitutions', () => {
  const p = prepared();
  for (const mutate of [
    (d) => {
      d.recorded_by.id = id(9);
    },
    (d) => {
      d.receipt.review_digest = hash('1');
    },
    (d) => {
      d.receipt.submission_digest = hash('1');
    },
    (d) => {
      d.receipt.operation_id = id(9);
    },
    (d) => {
      d.calculation.profile.definition_digest = hash('1');
    },
    (d) => {
      d.calculation.result.rule.quantity = 48;
    },
    (d) => {
      d.calculation.result.blocks.reverse();
      d.calculation.result.blocks.push({ kind: 'scope_unknown' });
    },
    (d) => {
      d.calculation.material.source_head = {};
    },
    (d) => {
      d.attention = { status: 'recorded' };
    },
  ]) {
    const d = detail(p);
    mutate(d);
    assert.equal(deadlineMatches(d, p), false);
  }
});
test('only exact deadline_not_found is an absent reconciliation and never retries writes', async () => {
  const p = prepared();
  let calls = 0;
  assert.deepEqual(
    await readDeadlineSubmission(
      {
        revision: async (idValue, rev) => {
          calls++;
          assert.equal(idValue, p.command.deadline_id);
          assert.equal(rev, 1);
          throw { status: 404, code: 'deadline_not_found' };
        },
      },
      p,
    ),
    { state: 'absent' },
  );
  assert.equal(calls, 1);
  await assert.rejects(() =>
    readDeadlineSubmission(
      {
        revision: async () => {
          throw { status: 404, code: 'case_not_found' };
        },
      },
      p,
    ),
  );
  assert.equal(
    (await readDeadlineSubmission({ revision: async () => detail(p) }, p)).state,
    'matched',
  );
});
