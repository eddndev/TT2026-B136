import { test } from 'node:test';
import assert from 'node:assert/strict';
import { planLiveSuite } from '../../scripts/web-live-plan.mjs';

test('real case reports use only their own fixture family on the primary runner', () => {
  const file = 'case-reports.spec.mjs';
  const plans = [1, 2, 3].map((part) => planLiveSuite([file], `${part}/3`));
  assert.deepEqual(plans[2], { files: [file], fixtures: ['caseReports'] });
  assert.deepEqual(plans[0], { files: [], fixtures: [] });
  assert.deepEqual(plans[1], { files: [], fixtures: [] });
});
