import test from 'node:test';
import assert from 'node:assert/strict';
import { readResourceDeadlineSubmission } from '../src/lib/resource-deadline-recovery.mjs';
import {
  resourceDeadlinePrepared,
  resourceDeadlineResult,
  clone,
} from './fixtures/resource-deadline-unit.mjs';
const missing = (code) => Object.assign(new Error('missing'), { status: 404, code });
function readers(deadline, association, calls = []) {
  return [
    {
      revision: async (...args) => {
        calls.push(['deadline', ...args]);
        if (deadline instanceof Error) throw deadline;
        return deadline;
      },
    },
    {
      revision: async (...args) => {
        calls.push(['association', ...args]);
        if (association instanceof Error) throw association;
        return { association };
      },
    },
  ];
}
test('matching first revisions remain a pair whose joint origin is unconfirmed', async () => {
  const draft = resourceDeadlinePrepared(),
    result = resourceDeadlineResult(draft),
    calls = [];
  assert.deepEqual(
    await readResourceDeadlineSubmission(
      ...readers(result.deadline, result.association, calls),
      draft,
    ),
    { state: 'paired' },
  );
  assert.deepEqual(calls, [
    ['deadline', draft.command.deadline.deadline_id, 1],
    ['association', draft.command.association_id, 1],
  ]);
});
test('both absent permits an explicit replay but never claims success for a partial pair', async () => {
  const draft = resourceDeadlinePrepared(),
    result = resourceDeadlineResult(draft);
  assert.equal(
    (
      await readResourceDeadlineSubmission(
        ...readers(missing('deadline_not_found'), missing('resource_activity_not_found')),
        draft,
      )
    ).state,
    'absent',
  );
  for (const pair of [
    [result.deadline, missing('resource_activity_not_found')],
    [missing('deadline_not_found'), result.association],
  ])
    assert.equal(
      (await readResourceDeadlineSubmission(...readers(...pair), draft)).state,
      'different',
    );
  const changed = clone(result.association);
  changed.receipt.operation_id = draft.command.association_id;
  assert.equal(
    (await readResourceDeadlineSubmission(...readers(result.deadline, changed), draft)).state,
    'different',
  );
});
test('authorization and transport errors never turn into replay permission', async () => {
  const draft = resourceDeadlinePrepared();
  for (const error of [
    Object.assign(new Error('denied'), { status: 403 }),
    missing('not_found'),
    new Error('offline'),
  ]) {
    await assert.rejects(
      readResourceDeadlineSubmission(
        ...readers(error, missing('resource_activity_not_found')),
        draft,
      ),
    );
  }
});
