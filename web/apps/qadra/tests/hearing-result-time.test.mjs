import test from 'node:test';
import assert from 'node:assert/strict';
import {
  hearingResultTime,
  hearingResultTimeDraft,
  hearingResultTimeLabel,
} from '../src/lib/hearing-result-time.mjs';

const now = Date.parse('2026-09-16T12:00:00Z');
const day = { precision: 'date', date: '2026-09-16', time: '', offset: '-06:00' };

test('a declared day preserves precision and offset without inventing an event time', () => {
  assert.deepEqual(hearingResultTime(day, now), {
    precision: 'date',
    date: '2026-09-16',
    offset: '-06:00',
  });
  assert.deepEqual(hearingResultTimeDraft(hearingResultTime(day, now)), day);
  assert.match(hearingResultTimeLabel(hearingResultTime(day, now)), /sin hora.*UTC-06:00/);
  assert.deepEqual(hearingResultTimeDraft(), { precision: 'date', date: '', time: '', offset: '' });
});

test('a declared instant completes explicit minute input and preserves existing seconds', () => {
  const value = hearingResultTime({ ...day, precision: 'instant', time: '05:07:09' }, now);
  assert.deepEqual(value, { precision: 'instant', at: '2026-09-16T05:07:09-06:00' });
  assert.deepEqual(hearingResultTimeDraft(value), {
    ...day,
    precision: 'instant',
    time: '05:07:09',
  });
  assert.equal(
    hearingResultTime({ ...day, precision: 'instant', time: '06:00' }, now).at,
    '2026-09-16T06:00:00-06:00',
  );
  assert.equal(
    hearingResultTimeDraft({ precision: 'instant', at: '2026-09-16T12:00:00Z' }).offset,
    '+00:00',
  );
});

test('future checks compare the beginning of a declared day and the whole declared instant', () => {
  assert.doesNotThrow(() => hearingResultTime(day, Date.parse('2026-09-16T06:00:00Z')));
  assert.throws(() => hearingResultTime(day, Date.parse('2026-09-16T05:59:59Z')), /futuro/);
  assert.doesNotThrow(() =>
    hearingResultTime({ ...day, precision: 'instant', time: '06:00:00' }, now),
  );
  assert.throws(
    () => hearingResultTime({ ...day, precision: 'instant', time: '06:00:01' }, now),
    /futuro/,
  );
});

test('date precision requires the entire local day to fit in UTC years 1 through 9999', () => {
  const last = Date.parse('9999-12-31T23:59:59Z');
  for (const [date, offset] of [
    ['0001-01-01', '+00:01'],
    ['9999-12-31', '-00:01'],
  ])
    assert.throws(() => hearingResultTime({ ...day, date, offset }, last), /UTC|fecha/);
  for (const [date, offset] of [
    ['0001-01-01', '-14:00'],
    ['9999-12-31', '+14:00'],
  ])
    assert.doesNotThrow(() => hearingResultTime({ ...day, date, offset }, last));
  assert.doesNotThrow(() =>
    hearingResultTime(
      { ...day, precision: 'instant', date: '9999-12-31', time: '00:00:00', offset: '-14:00' },
      last,
    ),
  );
});

test('impossible calendar values and unknown or excessive offsets never normalize silently', () => {
  for (const date of ['2026-02-29', '2026-04-31', '0000-01-01', '10000-01-01', '2026-9-16'])
    assert.throws(() => hearingResultTime({ ...day, date }, now));
  for (const offset of ['-00:00', '+14:01', '-14:01', '+01:60', 'Z', '', '+1:00'])
    assert.throws(() => hearingResultTime({ ...day, offset }, now));
  assert.doesNotThrow(() =>
    hearingResultTime({ ...day, date: '2024-02-29', offset: '+05:47' }, now),
  );
});

test('wire time parsing rejects fractions, leap seconds, lowercase markers and mixed precision', () => {
  for (const value of [
    { precision: 'instant', at: '2026-09-16T10:00:00.1Z' },
    { precision: 'instant', at: '2026-09-16T10:00:60Z' },
    { precision: 'instant', at: '2026-09-16t10:00:00z' },
    { precision: 'instant', at: '2026-09-16T10:00:00-00:00' },
    { precision: 'date', date: '2026-09-16', offset: '+00:00', at: '2026-09-16T10:00:00Z' },
    { precision: 'unknown', date: '2026-09-16', offset: '+00:00' },
  ])
    assert.throws(() => hearingResultTimeDraft(value));
});

test('historical rendering is independent of the current clock and preserves distinct offsets', () => {
  const a = { precision: 'instant', at: '2099-01-01T06:01:02-06:00' };
  const b = { precision: 'instant', at: '2099-01-01T12:01:02Z' };
  assert.equal(Date.parse(a.at), Date.parse(b.at));
  assert.notDeepEqual(hearingResultTimeDraft(a), hearingResultTimeDraft(b));
  assert.match(hearingResultTimeLabel(a), /06:01:02.*UTC-06:00/);
});
