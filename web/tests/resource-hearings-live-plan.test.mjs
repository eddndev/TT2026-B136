import { test } from 'node:test';
import assert from 'node:assert/strict';
import { planLiveSuite } from '../../scripts/web-live-plan.mjs';

test('own resource hearings reuse only the resource activities fixture in partition three', () => {
  const file = 'resource-hearings-real.spec.mjs';
  const plans = [1, 2, 3].map((n) => planLiveSuite([file], `${n}/3`));
  assert.deepEqual(plans[2], { files: [file], fixtures: ['resourceActivities'] });
  assert.deepEqual(plans[0].files, []);
  assert.deepEqual(plans[1].files, []);
});
