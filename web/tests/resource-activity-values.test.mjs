import { test } from 'node:test';
import assert from 'node:assert/strict';
import { resourceActivityCommand } from '../src/lib/resource-activity-values.mjs';

const id = (n) => `00000000-0000-0000-0000-${String(n).padStart(12, '0')}`;
const digest = (n) => n.repeat(64);
function link() {
  return {
    case_id: id(1),
    resource_id: id(2),
    association_id: id(3),
    operation_id: id(4),
    expected_resource_revision: 5,
    change: {
      action: 'link',
      expected_revision: 0,
      resource: { id: id(2), revision: 1, capture_digest: digest('a') },
      act: { id: id(5), revision: 1, resource_revision: 2, capture_digest: digest('b') },
      target: { kind: 'hearing', id: id(6), revision: 1, submission_digest: digest('c') },
    },
  };
}

test('link retains historical resource and act revisions separately from the expected head', () => {
  const input = link(),
    before = structuredClone(input);
  assert.deepEqual(resourceActivityCommand(input), before);
  assert.deepEqual(input, before);
});
test('a calculated deadline uses its exact capture digest and does not accept a manual due date', () => {
  const input = link();
  input.change.act = null;
  input.change.target = { kind: 'deadline', id: id(6), revision: 3, capture_digest: digest('d') };
  assert.deepEqual(resourceActivityCommand(input), input);
  input.change.target.due_at = '2026-09-19T18:00:00Z';
  assert.throws(() => resourceActivityCommand(input));
});
test('selection rejects a different resource identity and a revision newer than the expected head', () => {
  for (const mutate of [
    (c) => {
      c.change.resource.id = id(9);
    },
    (c) => {
      c.change.resource.revision = 6;
    },
    (c) => {
      c.change.act.resource_revision = 6;
    },
    (c) => {
      delete c.change.act;
    },
    (c) => {
      c.change.expected_revision = 1;
    },
    (c) => {
      c.expected_resource_revision = 0;
    },
  ]) {
    const input = link();
    mutate(input);
    assert.throws(() => resourceActivityCommand(input));
  }
});
test('target families cannot exchange receipt fields or accept unknown identifiers', () => {
  for (const mutate of [
    (c) => {
      c.change.target.kind = 'notification';
    },
    (c) => {
      c.change.target.capture_digest = digest('a');
    },
    (c) => {
      c.change.target.submission_digest = 'A'.repeat(64);
    },
    (c) => {
      c.operation_id = 'invalid';
    },
    (c) => {
      c.change.target.revision = 0;
    },
  ]) {
    const input = link();
    mutate(input);
    assert.throws(() => resourceActivityCommand(input));
  }
});
test('unlink declares an organizational reason without replacing captured targets', () => {
  const input = link();
  input.change = {
    action: 'unlink',
    expected_revision: 1,
    reason: 'Relacion registrada por error',
  };
  assert.deepEqual(resourceActivityCommand(input), input);
  input.change.target = link().change.target;
  assert.throws(() => resourceActivityCommand(input));
  delete input.change.target;
  input.change.reason = ' ';
  assert.throws(() => resourceActivityCommand(input));
});
