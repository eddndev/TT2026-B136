import { test } from 'node:test';
import assert from 'node:assert/strict';
import { planLiveSuite } from '../../scripts/web-live-plan.mjs';

test('real audit queries provision only independent audit fixtures on the primary runner', () => {
  const file = 'audit-events.spec.mjs';
  const plans = [1, 2, 3].map((part) => planLiveSuite([file], `${part}/3`));
  assert.deepEqual(plans[2], { files: [file], fixtures: ['auditEvents'] });
  assert.deepEqual(plans[0], { files: [], fixtures: [] });
  assert.deepEqual(plans[1], { files: [], fixtures: [] });
});

test('audit and report real scenarios retain their exact union and fixture ownership', () => {
  const files = ['audit-events.spec.mjs', 'case-reports.spec.mjs'];
  const plans = [1, 2, 3].map((part) => planLiveSuite(files, `${part}/3`));
  assert.deepEqual(plans.flatMap((plan) => plan.files).sort(), files);
  assert.deepEqual(plans[2].fixtures, ['caseReports', 'auditEvents']);
});
