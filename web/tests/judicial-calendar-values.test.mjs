import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  calendarDraft,
  calendarValues,
  calendarCommand,
  calendarUrl,
} from '../src/lib/judicial-calendar-values.mjs';
const vectors = JSON.parse(
  readFileSync(
    new URL('../../crates/domain/tests/fixtures/judicial_calendar_vectors.json', import.meta.url),
  ),
);
const sample = () =>
  structuredClone(vectors.find((v) => v.name === 'leap_unicode_unordered').input);
const id = '00000000-0000-0000-0000-000000000000';
test('new calendar drafts have no selected jurisdiction entities sources coverage or legal rules', () => {
  const d = calendarDraft();
  assert.equal(d.scope.jurisdiction, '');
  assert.deepEqual(d.scope.entity_codes, []);
  assert.deepEqual(d.sources, []);
  assert.equal(d.coverage.from, '');
  assert.equal(d.weekly_pattern.length, 7);
  assert.ok(
    d.weekly_pattern.every(
      (r) => r.classification === '' && r.explanation === '' && r.source_ids.length === 0,
    ),
  );
  assert.throws(() => calendarValues(d));
});
test('all independent domain vectors normalize identically without reimplementing the digest', () => {
  for (const v of vectors) assert.deepEqual(calendarValues(v.input), v.normalized, v.name);
});
test('sources require the bounded declared URL profile and preserve accepted spelling', () => {
  for (const u of ['https://Example.ORG/a?x=1#p2', 'https://example.org/%FF'])
    assert.equal(calendarUrl(` ${u} `), u);
  for (const u of [
    'http://example.org',
    'HTTPS://example.org',
    'https://example.org:443',
    'https://user@example.org',
    'https://127.0.0.1',
    'https://xn--abc.org',
    'https://example.org/%G0',
    'https://example.org/#a#b',
    'https://example.org/a\\b',
    '\thttps://example.org',
    'https://example.org/<script>',
  ])
    assert.throws(() => calendarUrl(u), u);
});
test('invalid references coverage intersections and ambiguous rules are rejected', () => {
  const invalid = [
    (v) => v.scope.entity_codes.push('01'),
    (v) => (v.scope.entity_codes = [' 01']),
    (v) => (v.sources[0].published_on = '2001-01-01'),
    (v) => (v.weekly_pattern[0].source_ids = []),
    (v) => v.weekly_pattern.pop(),
    (v) => (v.exceptions[0].from = '2000-02-29'),
    (v) => (v.coverage.through = '2004-01-01'),
    (v) => (v.sources = []),
    (v) => (v.scope.title = 'bad\u007f'),
    (v) => (v.scope.title = '\ud800'),
  ];
  for (const change of invalid) {
    const v = sample();
    change(v);
    assert.throws(() => calendarValues(v));
  }
});
test('commands preserve immutable scope require explicit reason and retire without changing values', () => {
  const v = calendarValues(sample()),
    base = { id, revision: 3, status: 'published', values: v };
  const draft = { ...calendarDraft(base), reason: ' Motivo\r\nexpreso ' };
  const options = { action: 'replace', base, calendarId: id, operationId: id };
  assert.equal(calendarCommand(draft, options).change.reason, 'Motivo\nexpreso');
  assert.equal(calendarCommand(draft, { ...options, action: 'retire' }).change.values, undefined);
  draft.scope.title = 'Other';
  assert.throws(() => calendarCommand(draft, options));
  assert.throws(() => calendarCommand(draft, { ...options, base: { ...base, status: 'retired' } }));
  assert.throws(() =>
    calendarCommand(draft, { ...options, base: { ...base, revision: 4294967295 } }),
  );
});
