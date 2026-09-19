import test from 'node:test';
import assert from 'node:assert/strict';
import { canAlerts } from '../src/lib/alerts-presentation.mjs';
import { normalizeView, viewLabels } from '../src/lib/workspace.mjs';

test('personal alerts are available to staff without granting Client access', () => {
  for (const role of ['owner', 'litigator', 'paralegal']) assert.equal(canAlerts(role), true);
  for (const role of ['client', 'other', '', null, undefined]) assert.equal(canAlerts(role), false);
});

test('alerts navigation normalizes by current role and has its own title', () => {
  for (const role of ['owner', 'litigator', 'paralegal'])
    assert.equal(normalizeView('#alerts', role), 'alerts');
  assert.equal(normalizeView('#alerts', 'client'), 'overview');
  assert.equal(viewLabels.alerts, 'Mis alertas');
});
