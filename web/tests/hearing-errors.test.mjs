import test from 'node:test';
import assert from 'node:assert/strict';
import { hearingFailure } from '../src/lib/hearing-errors.mjs';
for (const [code, expected] of [
  ['hearing_support_too_large', '16 MiB'],
  ['hearing_support_format_rejected', 'PDF o DOCX'],
  ['hearing_support_validation_limit', 'validaci\u00f3n'],
  ['hearing_support_digest_mismatch', 'Vuelve a elegir'],
]) {
  test(`${code} explains the support correction in Spanish`, () => {
    assert.ok(hearingFailure({ code, status: 422, message: 'Backend failure' }).includes(expected));
  });
}
