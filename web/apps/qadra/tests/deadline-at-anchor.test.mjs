import test from 'node:test';
import assert from 'node:assert/strict';
import { deadlineProfilesApi } from '../src/lib/deadline-profiles-api.mjs';
import { deadlineRule } from '../src/lib/deadline-rule.mjs';
import { deadlineResult } from '../src/lib/deadline-result.mjs';
import { deadlineTrace } from '../src/lib/deadline-trace.mjs';
import { deadlineInstantLabel } from '../src/lib/deadline-time.mjs';
import { deadlineRuleLabel } from '../src/components/deadline-view-labels.mjs';
import { componentMarkup } from './deadline-editor-render.mjs';
import { definition, profile, id } from './fixtures/deadline-unit.mjs';

const at = { unix_seconds: 1767247207, nanosecond: 0, offset_seconds: -21600 };
const anchor = {
  precision: 'second',
  year: 2026,
  month: 1,
  day: 1,
  hour: 0,
  minute: 0,
  second: 7,
  offset_seconds: -21600,
};
const requirement = { kind: 'qualified', purpose: 'hearing_end', family: 'hearing_result' };

function anchorProfile() {
  const row = profile();
  row.definition.title = 'Fin declarado de audiencia';
  row.definition.trigger = { ...requirement };
  row.definition.template.rule = { kind: 'at_anchor' };
  row.definition.examples[0].anchor = { ...anchor };
  row.definition.examples[0].expected.outcome = { kind: 'instant_candidate', instant: { ...at } };
  return row;
}
function result() {
  return {
    requirement: { ...requirement },
    trigger_outcome: { kind: 'extracted', at: { ...anchor } },
    rule: { kind: 'at_anchor' },
    arithmetic: {
      rule: { kind: 'at_anchor' },
      anchor: { ...anchor },
      outcome: { kind: 'instant_candidate', instant: { ...at } },
      trace: [{ kind: 'at_anchor', instant: { ...at } }],
    },
    due_at: { ...at, offset_seconds: 0 },
    blocks: [],
  };
}

test('at anchor profile exact read preserves the hearing end without a quantity', async () => {
  const row = anchorProfile(),
    calls = [];
  const api = deadlineProfilesApi(async (path) => {
    calls.push(path);
    return structuredClone(row);
  }, id(1));
  assert.deepEqual(await api.revision(id(2), 1), row);
  assert.deepEqual(calls, [`/cases/${id(1)}/deadline-profiles/${id(2)}/revisions/1`]);
  assert.deepEqual(deadlineRule({ kind: 'at_anchor' }), { kind: 'at_anchor' });
});

test('at anchor rejects quantities, ordered units and profiles outside qualified hearing end', async () => {
  for (const raw of [
    { kind: 'at_anchor', quantity: 0 },
    { kind: 'at_anchor', quantity: 1 },
    { kind: 'at_anchor', final_day: 'preserve' },
  ])
    assert.throws(() => deadlineRule(raw));
  assert.throws(() => deadlineRule({ kind: 'at_anchor' }, true));
  for (const change of [
    (d) => {
      d.template = { kind: 'ordered', unit: { kind: 'at_anchor' }, maximum: null };
    },
    (d) => {
      d.trigger = { kind: 'source_field', field: 'hearing_session_event_time' };
    },
    (d) => {
      d.trigger.purpose = 'ordered_period_start';
    },
    (d) => {
      d.trigger.family = 'resolution';
    },
    (d) => {
      d.trigger.family = 'notification';
    },
    (d) => {
      d.completion = { kind: 'civil_candidate_only' };
    },
  ]) {
    const row = anchorProfile();
    change(row.definition);
    await assert.rejects(() => deadlineProfilesApi(async () => row, id(1)).revision(id(2), 1));
  }
});

test('at anchor result and trace retain exact instant components and original offset', () => {
  const value = result();
  assert.deepEqual(deadlineResult(value), value);
  assert.equal(value.arithmetic.outcome.instant.offset_seconds, -21600);
  assert.equal(value.due_at.offset_seconds, 0);
  const step = {
    kind: 'at_anchor',
    instant: { ...at, nanosecond: 123456789, offset_seconds: -21600 },
  };
  assert.deepEqual(deadlineTrace(step), step);
  for (const invalid of [
    { ...step, quantity: 1 },
    { kind: 'at_anchor' },
    { ...step, instant: { ...at, nanosecond: 1000000000 } },
  ])
    assert.throws(() => deadlineTrace(invalid));
});

test('at anchor blocked results preserve missing precision and offset without a due instant', () => {
  const { second, ...minute } = anchor;
  for (const [block, declared] of [
    [{ kind: 'unknown_anchor' }, { precision: 'unknown' }],
    [
      { kind: 'insufficient_precision', observed: 'minute' },
      { ...minute, precision: 'minute' },
    ],
    [{ kind: 'missing_offset' }, { ...anchor, offset_seconds: null }],
  ]) {
    const value = result();
    value.trigger_outcome.at = declared;
    value.arithmetic.anchor = declared;
    value.arithmetic.outcome = { kind: 'blocked', block };
    value.arithmetic.trace = [];
    value.due_at = null;
    value.blocks = [{ kind: 'arithmetic', block }];
    assert.deepEqual(deadlineResult(value), value);
  }
});

test('at anchor labels and trace show the declared hearing end without a duration', async () => {
  const label = deadlineRuleLabel({ kind: 'at_anchor' });
  assert.match(label, /fin declarado de audiencia/i);
  assert.doesNotMatch(label, /cantidad|horas|d[i\u00ed]as|meses/i);
  const html = await componentMarkup('DeadlineTrace', { trace: result().arithmetic.trace });
  assert.match(html, /Fin declarado de audiencia/);
  assert.ok(html.includes(deadlineInstantLabel(at)));
  assert.doesNotMatch(html, /Cantidad:|Conteo|Adici/);
});

test('at anchor editor describes the end and does not ask for an ordered quantity', async () => {
  const html = await componentMarkup('DeadlineFields', {
    value: definition(),
    profile: anchorProfile(),
    caseId: id(1),
    api: { deadlineProfiles: () => ({ dispose() {} }) },
  });
  assert.match(html, /Fin declarado de audiencia/);
  assert.doesNotMatch(html, /Duraci[o\u00f3]n fija:|Cantidad ordenada|undefined|at_anchor/);
});
