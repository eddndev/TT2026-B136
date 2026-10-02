import test from 'node:test';
import assert from 'node:assert/strict';
import { uploadOutcomeUncertain } from '../src/lib/document-upload-outcome.mjs';

test('the explicit unavailable admission validator response confirms upload rejection', () => {
  assert.equal(uploadOutcomeUncertain({ status: 503, code: 'document_validator_unavailable' }), false);
});

test('unknown server and network failures preserve uncertainty even with unrelated error text', () => {
  for (const failure of [
    {}, { status: 500 }, { status: 502 }, { status: 503 }, { status: 504 },
    { status: 503, code: 'storage_unavailable' },
    { status: 500, code: 'document_validator_unavailable' },
    { code: 'document_validator_unavailable' },
  ]) assert.equal(uploadOutcomeUncertain(failure), true);
});

test('explicit client rejections leave the retained upload available for a new decision', () => {
  for (const status of [400, 401, 403, 404, 409, 413, 415, 422, 429])
    assert.equal(uploadOutcomeUncertain({ status }), false);
});
