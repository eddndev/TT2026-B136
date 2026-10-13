import test from 'node:test';
import assert from 'node:assert/strict';
import { agendaCivilDay } from '../src/lib/agenda-periods.mjs';

const instant = (unix_seconds, nanosecond = 0, offset_seconds = 0) => ({
  unix_seconds,
  nanosecond,
  offset_seconds,
});

test('deadline instant components use the requested civil offset across midnight', () => {
  assert.equal(agendaCivilDay(instant(1790820000), '-06:00'), '2026-09-30');
  assert.equal(agendaCivilDay(instant(1790811000), '+02:00'), '2026-10-01');
  assert.equal(agendaCivilDay(instant(1790834400), '+00:00'), '2026-10-01');
});

test('nanoseconds never round the last second into another day', () => {
  assert.equal(agendaCivilDay(instant(86399, 999999999), '+00:00'), '1970-01-01');
  assert.equal(agendaCivilDay(instant(-1, 999999999), '+00:00'), '1969-12-31');
  assert.equal(agendaCivilDay(instant(1790820000, 999999999), '-06:00'), '2026-09-30');
});

test('the instant original offset is validated but not applied to its UTC seconds', () => {
  const captured = Object.freeze(instant(1790812800, 123456789, -21600));
  assert.equal(agendaCivilDay(captured, '+00:00'), '2026-10-01');
  assert.equal(agendaCivilDay(captured, '-06:00'), '2026-09-30');
  assert.deepEqual(captured, instant(1790812800, 123456789, -21600));
});

test('instant component objects require exact integer seconds and nanoseconds', () => {
  for (const seconds of [undefined, null, '0', 0.5, NaN, Infinity, 9007199254740992])
    assert.throws(() => agendaCivilDay(instant(seconds), '+00:00'));
  for (const nanosecond of [undefined, null, '0', -1, 1000000000, 0.5, NaN, Infinity])
    assert.throws(() => agendaCivilDay({ ...instant(0), nanosecond }, '+00:00'));
  assert.throws(() => agendaCivilDay({ nanosecond: 0, offset_seconds: 0 }, '+00:00'));
  assert.throws(() => agendaCivilDay({ unix_seconds: 0, offset_seconds: 0 }, '+00:00'));
  assert.throws(() => agendaCivilDay({ ...instant(0), extra: true }, '+00:00'));
});

test('instant components retain the supported UTC and displayed year boundaries', () => {
  assert.equal(agendaCivilDay(instant(-62135596800), '+00:00'), '0001-01-01');
  assert.equal(agendaCivilDay(instant(253402300799, 999999999), '+00:00'), '9999-12-31');
  assert.throws(() => agendaCivilDay(instant(-62135596801), '+14:00'));
  assert.throws(() => agendaCivilDay(instant(253402300800), '-14:00'));
  assert.throws(() => agendaCivilDay(instant(-62135596800), '-06:00'));
  assert.throws(() => agendaCivilDay(instant(253402300799), '+14:00'));
});

test('instant objects reject invalid captured offsets and invalid query offsets', () => {
  for (const offset_seconds of [undefined, null, '0', 0.5, 93600, -93600, NaN])
    assert.throws(() => agendaCivilDay({ ...instant(0), offset_seconds }, '+00:00'));
  assert.throws(() => agendaCivilDay({ unix_seconds: 0, nanosecond: 0 }, '+00:00'));
  for (const offset of [undefined, null, '-00:00', '+14:01', '+15:00', '06:00', '-06:60'])
    assert.throws(() => agendaCivilDay(instant(0), offset));
});
