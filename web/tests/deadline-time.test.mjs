import test from 'node:test';
import assert from 'node:assert/strict';
import { deadlineInstant, deadlineInstantLabel } from '../src/lib/deadline-time.mjs';

test('calculated instants preserve sub-millisecond precision and non-RFC3339 offsets', () => {
  const i = { unix_seconds: -1, nanosecond: 987654321, offset_seconds: 93599 };
  assert.deepEqual(deadlineInstant(i), i);
  assert.equal(deadlineInstantLabel(i), '1970-01-02 01:59:58.987654321 / UTC+25:59:59');
  assert.equal(
    deadlineInstantLabel({ ...i, offset_seconds: -93599 }),
    '1969-12-30 22:00:00.987654321 / UTC-25:59:59',
  );
  assert.equal(
    deadlineInstantLabel({ unix_seconds: 0, nanosecond: 1, offset_seconds: 0 }),
    '1970-01-01 00:00:00.000000001 / UTC+00:00',
  );
});
test('instant bounds reject loss of precision, extra keys and invalid nanoseconds', () => {
  for (const i of [
    { unix_seconds: 0.1, nanosecond: 0, offset_seconds: 0 },
    { unix_seconds: Number.MAX_SAFE_INTEGER + 1, nanosecond: 0, offset_seconds: 0 },
    { unix_seconds: 0, nanosecond: 1000000000, offset_seconds: 0 },
    { unix_seconds: 0, nanosecond: -1, offset_seconds: 0 },
    { unix_seconds: 0, nanosecond: 0, offset_seconds: 93600 },
    { unix_seconds: 0, nanosecond: 0, offset_seconds: 0, precision: 'second' },
  ])
    assert.throws(() => deadlineInstant(i));
});
