import test from 'node:test';
import assert from 'node:assert/strict';
import { deadlineResult } from '../src/lib/deadline-result.mjs';
import { prepared, instant, id } from './fixtures/deadline-unit.mjs';

test('monthly failure preserves missing homologous day and full trace', () => {
  const result = prepared().calculation.result;
  result.trigger_outcome = {
    kind: 'extracted',
    at: { precision: 'date', year: 2026, month: 1, day: 31, offset_seconds: null },
  };
  result.rule = { kind: 'civil_months', quantity: 1, final_day: 'preserve' };
  const block = { kind: 'missing_homologous_day', year: 2026, month: 2, requested_day: 31 };
  result.arithmetic = {
    rule: result.rule,
    anchor: result.trigger_outcome.at,
    outcome: { kind: 'blocked', block },
    trace: [
      {
        kind: 'civil_months',
        anchor: '2026-01-31',
        quantity: 1,
        target_year: 2026,
        target_month: 2,
        requested_day: 31,
        candidate: null,
      },
    ],
  };
  result.blocks = [{ kind: 'arithmetic', block }];
  assert.deepEqual(deadlineResult(result), result);
  result.arithmetic.trace[0].candidate = '2026-02-31';
  assert.throws(() => deadlineResult(result));
});
test('calendar trace preserves unresolved versus outside coverage and source identifiers', () => {
  const result = prepared().calculation.result;
  const at = { precision: 'date', year: 2026, month: 1, day: 1, offset_seconds: null };
  const block = { kind: 'outside_calendar_coverage', date: '2026-01-02' };
  result.trigger_outcome = { kind: 'extracted', at };
  result.rule = {
    kind: 'days',
    quantity: 2,
    inclusion: 'on_anchor',
    basis: 'calendar_countable',
    final_day: 'preserve',
  };
  result.arithmetic = {
    rule: result.rule,
    anchor: at,
    outcome: { kind: 'blocked', block },
    trace: [
      {
        kind: 'counted_days',
        count: {
          first_included: '2026-01-01',
          quantity: 2,
          accumulated: 1,
          outcome: { kind: 'outside_coverage', date: '2026-01-02' },
          trace: [
            {
              day: {
                date: '2026-01-01',
                origin: { kind: 'weekly_pattern', weekday: 4 },
                classification: 'countable',
                explanation: 'Regla',
                source_ids: [id(1)],
              },
              accumulated: 1,
            },
            {
              day: {
                date: '2026-01-02',
                origin: null,
                classification: null,
                explanation: null,
                source_ids: [],
              },
              accumulated: 1,
            },
          ],
        },
      },
    ],
  };
  result.blocks = [{ kind: 'arithmetic', block }];
  assert.deepEqual(deadlineResult(result), result);
  result.arithmetic.trace[0].count.trace[1].day.classification = 'unresolved';
  assert.throws(() => deadlineResult(result));
});
