import { test } from 'node:test';
import assert from 'node:assert/strict';
import { planLiveSuite } from '../../scripts/web-live-plan.mjs';

test('document classification runs once on the primary without extra fixture setup', () => {
  const file = 'document-classification.spec.mjs';
  const plans = [1, 2, 3].map((n) => planLiveSuite([file], `${n}/3`));
  assert.deepEqual(plans, [
    { files: [], fixtures: [] },
    { files: [], fixtures: [] },
    { files: [file], fixtures: [] },
  ]);
});

test('ordinary document workflow keeps its existing first partition', () => {
  const file = 'document-workflow.spec.mjs';
  const plans = [1, 2, 3].map((n) => planLiveSuite([file], `${n}/3`));
  assert.deepEqual(plans, [
    { files: [file], fixtures: [] },
    { files: [], fixtures: [] },
    { files: [], fixtures: [] },
  ]);
});
