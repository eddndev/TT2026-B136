import test from 'node:test';
import assert from 'node:assert/strict';
import { factTime, factTimeLabel } from '../src/lib/procedural-fact-time.mjs';
const date = (year = 2026, month = 9, day = 16, offset_seconds = null) => ({
  precision: 'date',
  year,
  month,
  day,
  offset_seconds,
});

test('declared time retains unknown date minute second and absent offset without invention', () => {
  for (const value of [
    { precision: 'unknown' },
    date(),
    date(2026, 9, 16, 0),
    { ...date(), precision: 'minute', hour: 12, minute: 34 },
    { ...date(), precision: 'second', hour: 12, minute: 34, second: 0 },
    { ...date(9999, 12, 31, 0), precision: 'second', hour: 23, minute: 59, second: 59 },
  ])
    assert.deepEqual(factTime(value), value);
  const omitted = date();
  delete omitted.offset_seconds;
  assert.deepEqual(factTime(omitted), date());
  assert.match(factTimeLabel({ precision: 'unknown' }), /[Ss]in fecha|[Nn]o consta|[Dd]esconoc/);
  assert.match(factTimeLabel(date()), /sin hora/);
  assert.match(factTimeLabel({ ...date(), precision: 'minute', hour: 12, minute: 34 }), /12:34/);
  assert.doesNotMatch(
    factTimeLabel({ ...date(), precision: 'minute', hour: 12, minute: 34 }),
    /12:34:00/,
  );
  assert.match(factTimeLabel(date(2026, 9, 16, 0)), /UTC\+00:00/);
  assert.doesNotMatch(factTimeLabel(date()), /UTC\+00:00/);
});
test('range validation covers entire date or minute without exposing an inferred instant', () => {
  for (const value of [
    date(1, 1, 1),
    date(9999, 12, 31),
    { ...date(1, 1, 1, 60), precision: 'minute', hour: 0, minute: 1 },
    { ...date(9999, 12, 31, -60), precision: 'minute', hour: 23, minute: 58 },
  ])
    assert.deepEqual(factTime(value), value);
  for (const value of [
    date(1, 1, 1, 60),
    date(9999, 12, 31, -60),
    { ...date(1, 1, 1, 60), precision: 'minute', hour: 0, minute: 0 },
    { ...date(9999, 12, 31, -60), precision: 'minute', hour: 23, minute: 59 },
  ])
    assert.throws(() => factTime(value));
});
test('time rejects impossible dates offsets coercion missing precision and extra components', () => {
  for (const value of [
    null,
    {},
    { precision: 'unknown', offset_seconds: null },
    date(2026, 2, 29),
    date(0, 1, 1),
    date(2026, 1, 1, 1),
    date(2026, 1, 1, 50460),
    { ...date(), year: '2026' },
    { ...date(), hour: 0 },
    { ...date(), precision: 'minute', hour: 1, minute: 2, second: 0 },
    { ...date(), precision: 'second', hour: 1, minute: 2 },
    { ...date(), precision: 'second', hour: 24, minute: 0, second: 0 },
  ])
    assert.throws(() => factTime(value));
  assert.deepEqual(factTime(date(2028, 2, 29)), date(2028, 2, 29));
});
