import test from 'node:test';
import assert from 'node:assert/strict';
import {
  deadlineNormalizeCommand,
  deadlineDefinition,
  deadlineAttention,
} from '../src/lib/deadline-values.mjs';
import { definition, prepared, id, known } from './fixtures/deadline-unit.mjs';

test('deadline definitions preserve absent quantities, unknown conditions and nil agreements', () => {
  const d = definition();
  d.input.selection.source = known({
    family: 'hearing_result',
    hearing_id: id(8),
    result_id: id(9),
    revision: 3,
    agreement_id: id(0),
  });
  d.input.qualification.scope_applies = { kind: 'unknown', reason: '  Por revisar  ' };
  const result = deadlineDefinition(d);
  assert.equal(result.input.ordered_quantity, null);
  assert.equal(result.input.selection.source.value.agreement_id, id(0));
  assert.deepEqual(result.input.qualification.scope_applies, {
    kind: 'unknown',
    reason: 'Por revisar',
  });
  d.input.selection.source.value.agreement_id = null;
  assert.equal(deadlineDefinition(d).input.selection.source.value.agreement_id, null);
});
test('commands normalize human text and reject foreign, missing and contradictory fields', () => {
  const c = prepared('correct').command;
  c.change.reason = '  Correccion\r\nexplicita  ';
  assert.equal(deadlineNormalizeCommand(c).change.reason, 'Correccion\nexplicita');
  for (const mutate of [
    (c) => {
      c.change.action = 'publish';
    },
    (c) => {
      c.change.expected_revision = 0;
    },
    (c) => {
      c.change.expected_revision = 4294967295;
    },
    (c) => {
      c.change.attention = { status: 'pending' };
    },
    (c) => {
      c.change.definition.input.ordered_quantity = 0;
    },
    (c) => {
      delete c.change.definition.input.calendar;
    },
    (c) => {
      c.change.definition.input.qualification.scope_applies = known('false');
    },
    (c) => {
      c.change.definition.input.qualification.conditions = [1];
    },
    (c) => {
      c.change.definition.input.selection.source = known({
        family: 'notification',
        id: id(8),
        revision: 1,
      });
    },
  ]) {
    const next = structuredClone(c);
    mutate(next);
    assert.throws(() => deadlineNormalizeCommand(next));
  }
});
test('condition IDs are unique, references remain exact, attention has explicit precision', () => {
  const d = definition(),
    condition = { id: id(8), applies: known(false), locator: 'Articulo 1' };
  d.input.qualification.conditions = [condition, condition];
  assert.throws(() => deadlineDefinition(d));
  d.input.qualification.conditions = [condition];
  d.input.selection.qualification = {
    purpose: 'ordered_period_start',
    at: { precision: 'unknown' },
    statement: 'No consta fecha',
    locator: 'Acuerdo 1',
  };
  assert.equal(deadlineDefinition(d).input.selection.qualification.at.precision, 'unknown');
  assert.deepEqual(deadlineAttention({ status: 'pending' }), { status: 'pending' });
  assert.throws(() =>
    deadlineAttention({
      status: 'recorded',
      occurred_at: { precision: 'date', year: 2026, month: 2, day: 30 },
      statement: 'Acto',
      locator: 'Folio',
    }),
  );
});
