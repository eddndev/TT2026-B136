import test from 'node:test';
import assert from 'node:assert/strict';
import {
  agendaPeriod,
  shiftAgendaPeriod,
  agendaCivilDay,
  agendaDays,
} from '../src/lib/agenda-periods.mjs';

test('daily ranges use an exclusive next civil day across leap and year boundaries', () => {
  assert.deepEqual(agendaPeriod('day', '2028-02-28'), { from: '2028-02-28', until: '2028-02-29' });
  assert.deepEqual(agendaPeriod('day', '2028-02-29'), { from: '2028-02-29', until: '2028-03-01' });
  assert.deepEqual(agendaPeriod('day', '2026-12-31'), { from: '2026-12-31', until: '2027-01-01' });
});

test('weekly ranges start on Monday including Sunday and year changes', () => {
  for (const date of ['2026-12-28', '2026-12-31', '2027-01-03'])
    assert.deepEqual(agendaPeriod('week', date), { from: '2026-12-28', until: '2027-01-04' });
  assert.deepEqual(agendaPeriod('week', '2027-01-04'), { from: '2027-01-04', until: '2027-01-11' });
});

test('monthly ranges preserve whole calendar months rather than a fixed day count', () => {
  assert.deepEqual(agendaPeriod('month', '2028-02-29'), {
    from: '2028-02-01',
    until: '2028-03-01',
  });
  assert.deepEqual(agendaPeriod('month', '2026-12-31'), {
    from: '2026-12-01',
    until: '2027-01-01',
  });
  assert.deepEqual(agendaPeriod('month', '0001-01-10'), {
    from: '0001-01-01',
    until: '0001-02-01',
  });
});

test('period navigation moves civil anchors and clamps missing month days', () => {
  assert.equal(shiftAgendaPeriod('day', '2028-03-01', -1), '2028-02-29');
  assert.equal(shiftAgendaPeriod('week', '2026-12-30', 1), '2027-01-06');
  assert.equal(shiftAgendaPeriod('month', '2028-01-31', 1), '2028-02-29');
  assert.equal(shiftAgendaPeriod('month', '2027-03-31', -1), '2027-02-28');
  assert.equal(shiftAgendaPeriod('month', '2026-12-31', 1), '2027-01-31');
});

test('invalid civil inputs, modes and unsupported boundaries are rejected', () => {
  for (const date of [
    '2027-02-29',
    '2026-04-31',
    '2026-1-01',
    '0000-01-01',
    '10000-01-01',
    '',
    null,
  ])
    assert.throws(() => agendaPeriod('day', date));
  assert.throws(() => agendaPeriod('year', '2026-01-01'));
  assert.throws(() => agendaPeriod('day', '9999-12-31'));
  assert.throws(() => agendaPeriod('month', '9999-12-15'));
  assert.throws(() => shiftAgendaPeriod('day', '0001-01-01', -1));
  assert.throws(() => shiftAgendaPeriod('week', '2026-01-01', 0));
  assert.throws(() => shiftAgendaPeriod('month', '2026-01-01', 2));
});

test('grouping applies the explicit offset across UTC midnight without changing the instant', () => {
  assert.equal(agendaCivilDay('2026-10-01T02:00:00Z', '-06:00'), '2026-09-30');
  assert.equal(agendaCivilDay('2026-09-30T23:30:00Z', '+02:00'), '2026-10-01');
  assert.equal(agendaCivilDay('2026-10-01T00:00:00-06:00', '+00:00'), '2026-10-01');
  assert.equal(agendaCivilDay('2026-10-01T00:00:00.999999999Z', '-06:00'), '2026-09-30');
});

test('grouping rejects unavailable, malformed or unsupported instants and offsets', () => {
  for (const value of [null, '', '2026-02-30T01:00:00Z', '2026-01-01', '2026-01-01T00:00:60Z'])
    assert.throws(() => agendaCivilDay(value, '+00:00'));
  for (const offset of ['-00:00', '+14:01', '+15:00', '06:00', '-06:60'])
    assert.throws(() => agendaCivilDay('2026-10-01T00:00:00Z', offset));
  assert.throws(() => agendaCivilDay('0001-01-01T00:00:00Z', '-06:00'));
});

test('calendar days include empty dates and omit the exclusive boundary', () => {
  assert.deepEqual(agendaDays('2028-02-28', '2028-03-02'), [
    '2028-02-28',
    '2028-02-29',
    '2028-03-01',
  ]);
  assert.equal(agendaDays('2026-01-01', '2026-02-01').length, 31);
  assert.throws(() => agendaDays('2026-01-01', '2026-01-01'));
  assert.throws(() => agendaDays('2026-02-01', '2026-01-01'));
  assert.throws(() => agendaDays('2026-01-01', '2027-01-03'));
});
