import test from 'node:test';
import assert from 'node:assert/strict';
import * as presentation from '../src/lib/case-reports-presentation.mjs';
import { report, pending } from './browser/case-reports-fixtures.mjs';

function finished(start, end, state = 'ready', change = {}) {
  const value = report();
  return {
    ...value,
    requested_at: start,
    updated_at: end,
    state,
    failure: state === 'failed' ? 'render_failed' : null,
    ready: state === 'ready' ? { ...value.ready, checked_at: start } : null,
    notice: { kind: state, created_at: end, read_at: null },
    ...change,
  };
}
const start = '2026-09-27T12:00:00Z';
const terminal = () => finished(start, '2026-09-27T12:01:05Z');

test('terminal duration formats observed whole seconds through days without estimating', () => {
  for (const [end, expected] of [
    ['2026-09-27T12:00:00Z', 'Menos de 1 s'],
    ['2026-09-27T12:00:00.999999999Z', 'Menos de 1 s'],
    ['2026-09-27T12:00:01Z', '1 s'],
    ['2026-09-27T12:00:59.999999999Z', '59 s'],
    ['2026-09-27T12:01:00Z', '1 min'],
    ['2026-09-27T12:01:05Z', '1 min 5 s'],
    ['2026-09-27T14:00:00Z', '2 h'],
    ['2026-09-28T13:01:01Z', '1 d 1 h 1 min 1 s'],
  ])
    assert.equal(presentation.reportDuration(finished(start, end)), expected, end);
});

test('failed reports measure time until the matching failure notice', () => {
  assert.equal(
    presentation.reportDuration(finished(start, '2026-09-27T12:00:30Z', 'failed')),
    '30 s',
  );
});

test('notice acknowledgement and later updates cannot extend a terminal duration', () => {
  for (const state of ['ready', 'failed']) {
    const value = finished(start, '2026-09-27T12:01:05Z', state);
    const original = structuredClone(value);
    assert.equal(presentation.reportDuration(value), '1 min 5 s');
    const acknowledged = {
      ...value,
      updated_at: '2027-01-01T00:00:00Z',
      notice: { ...value.notice, read_at: '2027-01-01T00:00:00Z' },
    };
    assert.equal(presentation.reportDuration(acknowledged), '1 min 5 s');
    assert.deepEqual(value, original);
  }
});

test('active revoked and unknown reports have no observed terminal duration', () => {
  for (const value of [
    pending(),
    pending({ state: 'processing', phase: 'capturing' }),
    pending({ state: 'processing', phase: 'rendering' }),
    pending({ state: 'retry_waiting', phase: 'rendering', retry_at: '2026-09-27T12:01:00Z' }),
    pending({ state: 'access_revoked', failure: 'access_revoked' }),
    report({ state: 'unknown' }),
    null,
    undefined,
    {},
  ])
    assert.equal(presentation.reportDuration(value), null);
});

test('missing incompatible or out-of-lifetime terminal notices are not durations', () => {
  const value = terminal();
  for (const change of [
    { notice: null },
    { notice: { ...value.notice, kind: 'failed' } },
    { notice: { ...value.notice, created_at: '2026-09-27T11:59:59Z' } },
    { notice: { ...value.notice, created_at: '2026-09-27T12:02:00Z' } },
    { notice: { ...value.notice, read_at: '2026-09-27T12:00:01Z' } },
    { notice: { ...value.notice, created_at: null } },
    { updated_at: '2026-09-27T12:00:00Z' },
  ])
    assert.equal(presentation.reportDuration({ ...value, ...change }), null);
  assert.equal(
    presentation.reportDuration(
      finished(start, '2026-09-27T12:00:30Z', 'failed', {
        notice: { kind: 'ready', created_at: '2026-09-27T12:00:30Z', read_at: null },
      }),
    ),
    null,
  );
});

test('invalid dates and non-UTC timestamps are excluded rather than normalized', () => {
  for (const invalid of [
    '2026-02-30T12:00:00Z',
    '2026-09-27T24:00:00Z',
    '2026-09-27T12:00:00+00:00',
    '2026-09-27',
    '2026-09-27T12:00:00.1234567891Z',
    '',
    NaN,
  ]) {
    assert.equal(presentation.reportDuration(finished(invalid, '2026-09-27T12:01:05Z')), null);
    assert.equal(presentation.reportDuration(finished(start, invalid)), null);
  }
});

test('submillisecond boundaries preserve ordering and do not round elapsed time upward', () => {
  assert.equal(
    presentation.reportDuration(finished('2026-09-27T12:00:00.000000001Z', '2026-09-27T12:00:01Z')),
    'Menos de 1 s',
  );
  assert.equal(
    presentation.reportDuration(
      finished('2026-09-27T12:00:00.000000002Z', '2026-09-27T12:00:00.000000001Z'),
    ),
    null,
  );
  assert.equal(
    presentation.reportDuration(finished('2024-02-28T12:00:00Z', '2024-03-01T12:00:00Z')),
    '2 d',
  );
});
