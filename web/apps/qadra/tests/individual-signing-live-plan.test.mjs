import { test } from 'node:test';
import assert from 'node:assert/strict';
import { planLiveSuite } from '../../../../scripts/web-live-plan.mjs';

test('real individual signing owns one isolated fixture family in partition three', () => {
  const file = 'individual-signing-real.spec.mjs';
  const plans = [1, 2, 3].map((number) => planLiveSuite([file], `${number}/3`));
  assert.deepEqual(plans[0], { files: [], fixtures: [] });
  assert.deepEqual(plans[1], { files: [], fixtures: [] });
  assert.deepEqual(plans[2], { files: [file], fixtures: ['individualSigning'] });
});
