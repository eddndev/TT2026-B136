import test from 'node:test';
import assert from 'node:assert/strict';
import {
  canFacts,
  factDenied,
  factUncertain,
  factFailure,
} from '../src/lib/procedural-fact-errors.mjs';
test('fact permissions distinguish staff reads writes and denied client access', () => {
  for (const role of ['owner', 'litigator', 'paralegal', 'client', 'unknown']) {
    assert.equal(canFacts(role, 'read'), ['owner', 'litigator', 'paralegal'].includes(role));
    assert.equal(canFacts(role, 'manage'), ['owner', 'litigator'].includes(role));
    assert.equal(canFacts(role, 'unknown'), false);
  }
  for (const failure of [{ status: 401 }, { status: 403 }, { status: 404, code: 'case_not_found' }])
    assert.equal(factDenied(failure), true);
  assert.equal(factDenied({ status: 404, code: 'procedural_fact_not_found' }), false);
});
test('uncertain writes and source failures receive specific actionable messages without retry promises', () => {
  for (const failure of [{}, { status: 500 }, { status: 503 }])
    assert.equal(factUncertain(failure), true);
  for (const status of [400, 403, 404, 409, 413, 422])
    assert.equal(factUncertain({ status }), false);
  for (const code of [
    'procedural_fact_revision_conflict',
    'procedural_fact_already_withdrawn',
    'procedural_fact_operation_conflict',
    'procedural_fact_submission_mismatch',
    'procedural_fact_support_changed',
    'procedural_fact_support_too_large',
    'procedural_fact_support_format_rejected',
    'procedural_fact_support_validation_limit',
    'procedural_fact_support_digest_mismatch',
    'invalid_declared_procedural_time',
  ]) {
    const message = factFailure({ code, message: 'fallback' });
    assert.notEqual(message, 'fallback', code);
    assert.ok(message.length > 20);
  }
  assert.match(factFailure({ status: 413 }), /512/);
});
