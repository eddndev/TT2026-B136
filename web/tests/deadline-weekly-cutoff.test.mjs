import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { compile } from 'svelte/compiler';
import { render } from 'svelte/server';
import { deadlineProfilesApi } from '../src/lib/deadline-profiles-api.mjs';
import { deadlineProfileDefinition } from '../src/lib/deadline-profile-values.mjs';
import { profile, id } from './fixtures/deadline-unit.mjs';

function weeklyProfile() {
  const row = profile();
  row.definition.template.rule = {
    kind: 'days',
    quantity: 1,
    inclusion: 'after_anchor',
    basis: 'natural',
    final_day: 'preserve',
  };
  row.definition.completion = {
    kind: 'weekly_civil_cutoff',
    times: ['15:00:00', '15:00:00', '15:00:00', '15:00:00', '14:00:00', null, null],
    offset_seconds: -21600,
    from: '2026-01-01',
    through: '2026-12-31',
    channel: 'Ventanilla ordinaria',
    reference_id: id(20),
  };
  row.definition.examples[0].expected.outcome = { kind: 'civil_candidate', date: '2026-01-02' };
  return row;
}

test('weekly cutoff exact profile read preserves the seven days, channel and reference', async () => {
  const row = weeklyProfile(),
    calls = [];
  const api = deadlineProfilesApi(async (path) => {
    calls.push(path);
    return structuredClone(row);
  }, id(1));
  assert.deepEqual(await api.revision(id(2), 1), row);
  assert.deepEqual(calls, [`/cases/${id(1)}/deadline-profiles/${id(2)}/revisions/1`]);
  assert.deepEqual(deadlineProfileDefinition(row.definition, id(1)), row.definition);
});

test('weekly cutoff rejects malformed weeks, unknown fields and invalid shared metadata', () => {
  const raw = weeklyProfile().definition;
  const times = raw.completion.times;
  for (const invalid of [
    { times: times.slice(0, 6) },
    { times: [...times, null] },
    { times: Array(7).fill(null) },
    { times: ['24:00:00', ...times.slice(1)] },
    { times: ['15:60:00', ...times.slice(1)] },
    { times: ['15:00:60', ...times.slice(1)] },
    { times: ['15:00', ...times.slice(1)] },
    { times: ['15:00:00.1', ...times.slice(1)] },
    { times: [150000, ...times.slice(1)] },
    { times: null },
    { time: '15:00:00' },
    { invented: true },
    { offset_seconds: -21601 },
    { through: '2025-12-31' },
    { channel: '' },
    { reference_id: id(99) },
  ]) {
    const value = structuredClone(raw);
    Object.assign(value.completion, invalid);
    assert.throws(() => deadlineProfileDefinition(value, id(1)));
  }
  const elapsed = structuredClone(raw);
  elapsed.template.rule = { kind: 'elapsed_hours', quantity: 24 };
  assert.throws(() => deadlineProfileDefinition(elapsed, id(1)));
});

async function completionMarkup(definition) {
  const path = new URL('../src/components/DeadlineProfileDetails.svelte', import.meta.url);
  const source = await readFile(path, 'utf8');
  const start = source.indexOf('<h4>Finalizaci&#243;n</h4>');
  const end = source.indexOf('<h4>Condiciones de aplicabilidad</h4>', start);
  assert.ok(start >= 0 && end > start);
  const fragment = '<script>export let d;</script>' + source.slice(start, end);
  let code = compile(fragment, { filename: path.pathname, generate: 'server' }).js.code;
  for (const match of [...code.matchAll(/from ['"]([^'"]+)['"]/g)])
    code = code.replace(match[0], `from ${JSON.stringify(import.meta.resolve(match[1]))}`);
  const component = await import(
    `data:text/javascript;base64,${Buffer.from(code).toString('base64')}`
  );
  return render(component.default, { props: { d: definition } }).body;
}

test('weekly cutoff detail shows each weekday and uncovered days with channel provenance', async () => {
  const html = await completionMarkup(weeklyProfile().definition);
  for (const [day, time] of [
    ['Lunes', '15:00:00'],
    ['Martes', '15:00:00'],
    ['Mi\u00e9rcoles', '15:00:00'],
    ['Jueves', '15:00:00'],
    ['Viernes', '14:00:00'],
    ['S\u00e1bado', 'Sin cobertura'],
    ['Domingo', 'Sin cobertura'],
  ])
    assert.match(html, new RegExp(`${day}:\\s*${time}`));
  for (const expected of [
    'Ventanilla ordinaria',
    '2026-01-01',
    '2026-12-31',
    '-21600',
    'Fuente de prueba',
  ])
    assert.ok(html.includes(expected), `missing ${expected}`);
  assert.doesNotMatch(html, /Fecha civil candidata, sin hora de corte|undefined/);
});
