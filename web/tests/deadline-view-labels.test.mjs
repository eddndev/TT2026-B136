import test from 'node:test';
import assert from 'node:assert/strict';
import {
  deadlineBlockLabel,
  deadlineRuleLabel,
  deadlineRequirementLabel,
} from '../src/components/deadline-view-labels.mjs';

test('missing civil homologous day is visible without replacing it with month end', () => {
  const text = deadlineBlockLabel({
    kind: 'arithmetic',
    block: { kind: 'missing_homologous_day', year: 2026, month: 2, requested_day: 31 },
  });
  assert.match(text, /31/);
  assert.match(text, /2026-02/);
  assert.doesNotMatch(text, /2026-02-28/);
});
test('ordered duration distinguishes absence from an excessive supplied quantity', () => {
  assert.match(
    deadlineBlockLabel({ kind: 'rule', block: { kind: 'missing_ordered_quantity' } }),
    /concedida/,
  );
  assert.match(
    deadlineBlockLabel({
      kind: 'rule',
      block: { kind: 'ordered_quantity_exceeds_maximum', supplied: 73, maximum: 72 },
    }),
    /73.*72/,
  );
});
test('unknown applicability, a negative answer and unknown incident have distinct labels', () => {
  const labels = ['scope_unknown', 'scope_rejected', 'incident_unknown', 'unresolved_incident'].map(
    (kind) => deadlineBlockLabel({ kind }),
  );
  assert.equal(new Set(labels).size, 4);
  assert.match(
    deadlineBlockLabel({ kind: 'condition_missing', id: 'condition-reference' }),
    /condition-reference/,
  );
});
test('arithmetic labels preserve natural versus calendar days and inclusion', () => {
  const natural = deadlineRuleLabel({
    kind: 'days',
    quantity: 5,
    basis: 'natural',
    inclusion: 'after_anchor',
    final_day: 'preserve',
  });
  const calendar = deadlineRuleLabel({
    kind: 'days',
    quantity: 5,
    basis: 'calendar_countable',
    inclusion: 'on_anchor',
    final_day: 'next_countable',
  });
  assert.match(natural, /5.*naturales/);
  assert.match(natural, /siguiente/);
  assert.match(calendar, /calendario/);
  assert.notEqual(natural, calendar);
  assert.match(
    deadlineRuleLabel({ kind: 'civil_months', quantity: 1, final_day: 'preserve' }),
    /1.*mes/,
  );
  assert.match(deadlineRuleLabel({ kind: 'elapsed_hours', quantity: 24 }), /24.*horas/);
});
test('qualified trigger states the declared purpose and source family', () => {
  assert.match(
    deadlineRequirementLabel({
      kind: 'qualified',
      purpose: 'ordered_period_start',
      family: 'resolution',
    }),
    /inicio.*resoluci/i,
  );
});
