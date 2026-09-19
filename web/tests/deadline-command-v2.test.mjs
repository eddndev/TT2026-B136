import test from 'node:test';
import assert from 'node:assert/strict';
import { deadlineNormalizeCommand } from '../src/lib/deadline-values.mjs';
import { clone, v2Prepared, ids } from './fixtures/deadline-v2-unit.mjs';

test('registration and correction require all explicit tracking choices', () => {
  for (const action of ['register', 'correct']) {
    const command = v2Prepared(action).command;
    assert.deepEqual(deadlineNormalizeCommand(command), command);
    for (const mutate of [
      (c) => {
        delete c.change.tracking;
      },
      (c) => {
        c.change.tracking = null;
      },
      (c) => {
        c.change.tracking = [];
      },
      (c) => {
        delete c.change.tracking.calendar;
      },
      (c) => {
        c.change.tracking.profile = '';
      },
      (c) => {
        c.change.tracking.profile = 'undetermined';
      },
      (c) => {
        c.change.tracking.profile = 'Follow';
      },
      (c) => {
        c.change.tracking.source = true;
      },
      (c) => {
        c.change.tracking.extra = 'fixed';
      },
    ]) {
      const next = clone(command);
      mutate(next);
      assert.throws(() => deadlineNormalizeCommand(next));
    }
  }
});

test('dependency presence constrains policies without choosing intent from revisions', () => {
  const command = v2Prepared().command;
  command.change.definition.input.selection.source = {
    kind: 'known',
    value: { family: 'resolution', id: ids(0), revision: 1 },
  };
  command.change.definition.input.calendar = { id: ids(9), revision: 1 };
  assert.throws(() => deadlineNormalizeCommand(command));
  for (const policy of ['fixed', 'follow']) {
    command.change.tracking.source = policy;
    command.change.tracking.calendar = policy;
    const parsed = deadlineNormalizeCommand(command);
    assert.equal(parsed.change.tracking.source, policy);
    assert.equal(parsed.change.definition.input.selection.source.value.id, ids(0));
  }
  command.change.definition.input.selection.source = { kind: 'unknown', reason: 'Unknown' };
  assert.throws(() => deadlineNormalizeCommand(command));
  command.change.tracking.source = 'undetermined';
  command.change.definition.input.calendar = null;
  assert.throws(() => deadlineNormalizeCommand(command));
  command.change.tracking.calendar = 'undetermined';
  assert.deepEqual(deadlineNormalizeCommand(command), command);
});

test('attention and retirement prohibit supplied policies including null', () => {
  for (const action of ['set_attention', 'retire']) {
    const command = v2Prepared(action).command;
    assert.deepEqual(deadlineNormalizeCommand(command), command);
    for (const tracking of [null, {}, v2Prepared().tracking.policies]) {
      const next = clone(command);
      next.change.tracking = tracking;
      assert.throws(() => deadlineNormalizeCommand(next));
    }
  }
});

test('human requests cannot provide authors observations review or technical actions', () => {
  for (const key of [
    'author',
    'actor_id',
    'receipt_version',
    'observations',
    'review',
    'predecessor',
    'cause',
    'operational',
  ]) {
    for (const level of ['command', 'change']) {
      const command = v2Prepared().command;
      (level === 'command' ? command : command.change)[key] = null;
      assert.throws(() => deadlineNormalizeCommand(command));
    }
  }
  const command = v2Prepared('correct').command;
  command.change.action = 'reevaluate';
  assert.throws(() => deadlineNormalizeCommand(command));
});
