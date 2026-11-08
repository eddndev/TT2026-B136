import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  calendarCoverageMonths,
  calendarDayDescriber,
  calendarDayOrigin,
  calendarDayRow,
  calendarLegend,
} from '../src/lib/judicial-calendar-days.mjs';

// Pure projections behind CalendarDays: day names, tints and markers for the
// month grid, list rows and the coverage bounds of the month navigation.
const excluded = {
  date: '2000-02-29',
  state: 'excluded',
  origin: 'exception',
  source_ids: ['s'],
  explanation: 'Excepcion declarada',
};
const outside = {
  date: '2000-02-26',
  state: 'outside_coverage',
  origin: null,
  source_ids: [],
  explanation: null,
};
const weekly = { ...excluded, date: '2000-02-28', state: 'countable', origin: 'weekly_pattern' };

test('a classified day keeps its exact name, tint, symbol and state attribute', () => {
  const describe = calendarDayDescriber([excluded, outside, weekly]);
  assert.deepEqual(describe('2000-02-29'), {
    label: '29 de febrero de 2000: Excluido',
    tone: 'excluded',
    marker: { text: 'E' },
    attrs: { 'data-state': 'excluded' },
  });
  assert.deepEqual(describe('2000-02-26'), {
    label: '26 de febrero de 2000: Fuera de cobertura',
    tone: 'outside',
    marker: { text: '--' },
    attrs: { 'data-state': 'outside_coverage' },
  });
  assert.equal(describe('2000-02-28').marker.text, 'C');
});

test('a day without a loaded row is only named by its date and cannot be chosen', () => {
  assert.deepEqual(calendarDayDescriber([])('2000-02-29'), {
    label: '29 de febrero de 2000',
    attrs: { disabled: true },
  });
});

test('the legend lists every state with its symbol, label and tint', () => {
  assert.deepEqual(
    calendarLegend.map(({ state, symbol, label, tone }) => [state, symbol, label, tone]),
    [
      ['countable', 'C', 'Computable', 'countable'],
      ['excluded', 'E', 'Excluido', 'excluded'],
      ['unresolved', '?', 'Sin resolver', 'unresolved'],
      ['outside_coverage', '--', 'Fuera de cobertura', 'outside'],
    ],
  );
});

test('the origin text says where the classification comes from', () => {
  assert.equal(calendarDayOrigin(excluded), 'Excepci\u00f3n declarada');
  assert.equal(calendarDayOrigin(weekly), 'Patr\u00f3n semanal declarado');
  assert.equal(
    calendarDayOrigin(outside),
    'Esta revisi\u00f3n no cubre la fecha; no se aplica una regla por defecto.',
  );
});

test('a list row covers the whole civil day and keeps the grid name', () => {
  assert.deepEqual(calendarDayRow(excluded, '2000-02-29'), {
    key: '2000-02-29',
    value: excluded,
    label: '29 de febrero de 2000: Excluido',
    pressed: true,
    attrs: { 'data-state': 'excluded' },
    date: '2000-02-29',
    dateText: '29 de febrero de 2000',
    badge: 'E',
    title: 'Excluido',
    meta: ['Excepci\u00f3n declarada'],
    timeText: 'Todo el d\u00eda',
  });
  assert.equal(calendarDayRow(outside, null).pressed, false);
});

test('coverage bounds the month navigation by its first and last months', () => {
  assert.deepEqual(calendarCoverageMonths({ from: '2000-02-27', through: '2000-03-04' }), {
    min: '2000-02',
    max: '2000-03',
  });
});
