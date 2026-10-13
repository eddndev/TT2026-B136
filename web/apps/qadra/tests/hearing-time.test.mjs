import test from 'node:test';
import assert from 'node:assert/strict';
import { hearingInstant, hearingTimeParts } from '../src/lib/hearing-time.mjs';

test('hearing dates accept past and future without reading the current clock', () => {
  for (const date of ['1900-01-01', '2026-09-15', '9999-12-31']) {
    assert.equal(
      hearingInstant({ date, time: '12:34:56', offset: '+00:00' }),
      `${date}T12:34:56+00:00`,
    );
  }
});

test('hearing dates preserve local date, seconds and offset across UTC day boundaries', () => {
  for (const parts of [
    { date: '2026-01-01', time: '00:10:31', offset: '+14:00' },
    { date: '2026-12-31', time: '23:50:02', offset: '-14:00' },
    { date: '2026-09-15', time: '09:20:03', offset: '-06:00' },
    { date: '2026-09-15', time: '15:20:03', offset: '+00:00' },
  ]) {
    const instant = hearingInstant(parts);
    assert.equal(instant, `${parts.date}T${parts.time}${parts.offset}`);
    assert.deepEqual(hearingTimeParts(instant), parts);
  }
});

test('hearing dates accept every complete minute offset within fourteen hours', () => {
  for (let minutes = -840; minutes <= 840; minutes++) {
    const magnitude = Math.abs(minutes);
    const hours = String(Math.floor(magnitude / 60)).padStart(2, '0');
    const rest = String(magnitude % 60).padStart(2, '0');
    const offset = `${minutes < 0 ? '-' : '+'}${hours}:${rest}`;
    const parts = { date: '2026-09-15', time: '12:34:56', offset };
    assert.deepEqual(hearingTimeParts(hearingInstant(parts)), parts);
  }
});

test('hearing dates require valid Gregorian dates without silent normalization', () => {
  for (const date of [
    '2026-02-29',
    '1900-02-29',
    '2026-04-31',
    '2026-00-15',
    '2026-13-01',
    '2026-01-00',
    '2026-1-01',
    '2026-01-1',
    ' 2026-01-01',
    '2026-01-01 ',
    '',
  ]) {
    assert.throws(() => hearingInstant({ date, time: '12:00:00', offset: '+00:00' }), Error);
  }
  assert.equal(
    hearingInstant({ date: '2000-02-29', time: '12:00:00', offset: '+00:00' }),
    '2000-02-29T12:00:00+00:00',
  );
});

test('hearing dates reject fractional seconds, leap seconds and malformed times', () => {
  for (const time of ['24:00:00', '12:60:00', '12:00:60', '12:00:00.000', '1:00:00', '', null]) {
    assert.throws(() => hearingInstant({ date: '2026-09-15', time, offset: '+00:00' }), Error);
  }
});

test('hearing dates reject unknown, missing and out of range offsets', () => {
  for (const offset of [
    '-00:00',
    '+14:01',
    '-14:01',
    '+15:00',
    '+01:60',
    '+1:00',
    '+01:00:00',
    '',
    null,
  ]) {
    assert.throws(() => hearingInstant({ date: '2026-09-15', time: '12:00:00', offset }), Error);
  }
});

test('hearing local and UTC years remain within one and 9999', () => {
  for (const parts of [
    { date: '0001-01-01', time: '00:00:00', offset: '+00:00' },
    { date: '0001-01-01', time: '00:00:00', offset: '-14:00' },
    { date: '9999-12-31', time: '23:59:59', offset: '+00:00' },
    { date: '9999-12-31', time: '23:59:59', offset: '+14:00' },
  ]) {
    assert.deepEqual(hearingTimeParts(hearingInstant(parts)), parts);
  }
  for (const parts of [
    { date: '0000-12-31', time: '23:59:59', offset: '+00:00' },
    { date: '10000-01-01', time: '00:00:00', offset: '+00:00' },
    { date: '0001-01-01', time: '00:00:00', offset: '+00:01' },
    { date: '9999-12-31', time: '23:59:59', offset: '-00:01' },
  ]) {
    assert.throws(() => hearingInstant(parts), Error);
  }
});

test('hearing reader rejects invalid RFC3339 input instead of constructing an editable draft', () => {
  for (const value of [
    '2026-02-29T12:00:00+00:00',
    '2026-09-15T12:00:60+00:00',
    '2026-09-15T12:00:00.000+00:00',
    '2026-09-15T12:00+00:00',
    '2026-09-15T12:00:00',
    '2026-09-15 12:00:00+00:00',
    '2026-09-15T12:00:00-00:00',
    '0001-01-01T00:00:00+14:00',
    '',
    null,
  ]) {
    assert.throws(() => hearingTimeParts(value), Error);
  }
});

test('hearing date errors explain which explicit temporal value needs review', () => {
  assert.throws(
    () => hearingInstant({ date: '2026-04-31', time: '12:00:00', offset: '+00:00' }),
    /fecha/i,
  );
  assert.throws(
    () => hearingInstant({ date: '2026-09-15', time: '24:00:00', offset: '+00:00' }),
    /hora/i,
  );
  assert.throws(
    () => hearingInstant({ date: '2026-09-15', time: '12:00:00', offset: '+14:01' }),
    /desfase/i,
  );
  assert.throws(() => hearingInstant(null), /fecha.*hora.*desfase/i);
});

test('minute precision from the native time control starts at an explicit zero second', () => {
  const instant = hearingInstant({ date: '2026-09-15', time: '12:34', offset: '-06:00' });
  assert.equal(instant, '2026-09-15T12:34:00-06:00');
  assert.deepEqual(hearingTimeParts(instant), {
    date: '2026-09-15',
    time: '12:34:00',
    offset: '-06:00',
  });
});

test('UTC server timestamps become an explicit editable zero offset without losing seconds', () => {
  assert.deepEqual(hearingTimeParts('2026-09-15T09:02:03Z'), {
    date: '2026-09-15',
    time: '09:02:03',
    offset: '+00:00',
  });
  assert.equal(
    hearingInstant(hearingTimeParts('0001-01-01T00:00:00Z')),
    '0001-01-01T00:00:00+00:00',
  );
});

test('hearing values reject trailing line endings without silently trimming them', () => {
  const parts = { date: '2026-09-15', time: '12:34:56', offset: '+00:00' };
  for (const ending of ['\n', '\r', '\r\n']) {
    for (const field of ['date', 'time', 'offset']) {
      assert.throws(() => hearingInstant({ ...parts, [field]: `${parts[field]}${ending}` }), Error);
    }
    assert.throws(() => hearingTimeParts(`2026-09-15T12:34:56Z${ending}`), Error);
  }
});
