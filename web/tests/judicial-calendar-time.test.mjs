import test from 'node:test';
import assert from 'node:assert/strict';
import {
  civilDate,
  civilDays,
  calendarMonth,
  moveCalendarMonth,
  civilLabel,
} from '../src/lib/judicial-calendar-time.mjs';
test('civil dates accept real Gregorian extremes and leap days without clock conversion', () => {
  for (const d of ['0001-01-01', '2000-02-29', '2028-02-29', '9999-12-31'])
    assert.equal(civilDate(d), d);
  for (const d of [
    '0000-01-01',
    '10000-01-01',
    '1900-02-29',
    '2026-04-31',
    '2026-1-01',
    '2026-01-01T00:00:00Z',
    ' 2026-01-01',
  ])
    assert.throws(() => civilDate(d));
  assert.equal(civilDays('2000-02-28', '2000-03-01'), 3);
  assert.equal(civilDays('9999-12-31', '9999-12-31'), 1);
});
test('months expose bounded inclusive civil ranges and weekday without local timezone', () => {
  assert.deepEqual(calendarMonth('2000-02'), {
    from: '2000-02-01',
    through: '2000-02-29',
    length: 29,
    firstWeekday: 2,
  });
  assert.equal(calendarMonth('9999-12').through, '9999-12-31');
  assert.equal(moveCalendarMonth('0001-01', -1), null);
  assert.equal(moveCalendarMonth('9999-12', 1), null);
  assert.equal(moveCalendarMonth('2026-12', 1), '2027-01');
  assert.match(civilLabel('0001-01-01'), /1.*enero.*0001/);
  assert.throws(() => calendarMonth('2026-13'));
});
