import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  civilToday,
  dayTone,
  monthCells,
  monthNavigation,
  monthOf,
  monthTitle,
  periodContains,
  shiftMonth,
  toneClasses,
  visibleMonths,
  weekdayInitials,
} from '../src/lib/month-grid.mjs';

test('weekday initials start on Monday and keep the full names', () => {
  assert.deepEqual(
    weekdayInitials.map((day) => day.initial),
    ['L', 'M', 'M', 'J', 'V', 'S', 'D'],
  );
  assert.deepEqual(
    weekdayInitials.map((day) => day.name),
    ['Lunes', 'Martes', 'Mi\u00e9rcoles', 'Jueves', 'Viernes', 'S\u00e1bado', 'Domingo'],
  );
});

test('a month always fills six Monday-first weeks', () => {
  const cells = monthCells('2026-11');
  assert.equal(cells.length, 42);
  assert.equal(cells[0].date, '2026-10-26');
  assert.equal(cells[0].inMonth, false);
  assert.equal(cells[6].date, '2026-11-01');
  assert.equal(cells[6].day, 1);
  assert.equal(cells[6].inMonth, true);
  assert.equal(cells[35].date, '2026-11-30');
  assert.equal(cells[41].date, '2026-12-06');
  assert.equal(cells[41].inMonth, false);
  assert.deepEqual(
    cells.flatMap((cell, index) => (cell.corner ? [[index, cell.corner]] : [])),
    [
      [0, 'tl'],
      [6, 'tr'],
      [35, 'bl'],
      [41, 'br'],
    ],
  );
  assert.equal(monthCells('2026-06')[0].date, '2026-06-01');
  assert.equal(monthCells('2026-06')[0].inMonth, true);
});

test('cells never leave the civil range', () => {
  const last = monthCells('9999-12');
  assert.equal(last.length, 42);
  assert.equal(last[0].date, '9999-11-29');
  assert.ok(last.slice(-9).every((cell) => cell.date === null && !cell.inMonth));
  assert.equal(last.at(-10).date, '9999-12-31');
  assert.ok(last.every((cell) => cell.date === null || cell.date <= '9999-12-31'));
  const first = monthCells('0001-01');
  assert.equal(first[0].date, '0001-01-01');
  assert.ok(first.every((cell) => cell.date === null || cell.date >= '0001-01-01'));
});

test('only the in-month cell matching today is marked', () => {
  assert.deepEqual(
    monthCells('2026-11', '2026-11-05')
      .filter((cell) => cell.isToday)
      .map((cell) => cell.date),
    ['2026-11-05'],
  );
  assert.equal(monthCells('2026-11', '2026-12-05').filter((cell) => cell.isToday).length, 0);
  assert.equal(monthCells('2026-11', '2026-10-31').filter((cell) => cell.isToday).length, 0);
  assert.equal(monthCells('2026-11', null).filter((cell) => cell.isToday).length, 0);
});

test('months shift within the civil range', () => {
  assert.equal(shiftMonth('2026-12', 1), '2027-01');
  assert.equal(shiftMonth('2026-01', -1), '2025-12');
  assert.equal(shiftMonth('0001-01', -1), null);
  assert.equal(shiftMonth('9999-12', 1), null);
  assert.deepEqual(visibleMonths('2026-11', 2), ['2026-11', '2026-12']);
  assert.deepEqual(visibleMonths('2026-11', 1), ['2026-11']);
  assert.deepEqual(visibleMonths('9999-12', 2), ['9999-12']);
  assert.throws(() => visibleMonths('2026-11', 3));
});

test('navigation stops at the bounds and the civil limits', () => {
  const coverage = { count: 1, min: '2000-02', max: '2000-03' };
  assert.deepEqual(monthNavigation('2000-02', coverage), { previous: null, next: '2000-03' });
  assert.deepEqual(monthNavigation('2000-03', coverage), { previous: '2000-02', next: null });
  assert.equal(monthNavigation('2000-02', { ...coverage, count: 2 }).next, null);
  assert.deepEqual(monthNavigation('0001-01', { count: 1, min: '9999-12', max: '9999-12' }), {
    previous: null,
    next: '0001-02',
  });
  assert.deepEqual(monthNavigation('9999-12', { count: 1 }), { previous: '9999-11', next: null });
  assert.deepEqual(monthNavigation('2026-11', { count: 2 }), {
    previous: '2026-10',
    next: '2026-12',
  });
});

test('month helpers validate and name their input', () => {
  assert.equal(monthOf('2026-01-02'), '2026-01');
  assert.throws(() => monthOf('2026-02-30'));
  assert.equal(monthTitle('2026-01'), 'enero de 2026');
  assert.equal(monthTitle('0001-12'), 'diciembre de 0001');
  assert.equal(civilToday(new Date(2026, 10, 5)), '2026-11-05');
  assert.equal(civilToday(new Date(2026, 0, 9, 23, 59)), '2026-01-09');
});

test('periods exclude their end and tones are literal classes', () => {
  assert.equal(periodContains('2026-01-02', '2026-01-02', '2026-01-03'), true);
  assert.equal(periodContains('2026-01-03', '2026-01-02', '2026-01-03'), false);
  assert.equal(periodContains('2026-01-01', '2026-01-02', '2026-01-03'), false);
  assert.deepEqual(Object.keys(toneClasses).sort(), [
    'countable',
    'excluded',
    'outside',
    'period',
    'unresolved',
  ]);
  assert.equal(toneClasses.period, 'bg-brand-50');
  assert.equal(toneClasses.outside, 'bg-slate-100 text-slate-500');
});

test('a day tone falls back to white and keeps ink unless it sets its own text', () => {
  assert.equal(dayTone(undefined), 'bg-white text-ink');
  assert.equal(dayTone('period'), 'bg-brand-50 text-ink');
  assert.equal(dayTone('outside'), 'bg-slate-100 text-slate-500');
  assert.equal(dayTone('unknown'), 'bg-white text-ink');
});
