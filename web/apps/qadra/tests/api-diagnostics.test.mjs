import test from 'node:test';
import assert from 'node:assert/strict';
import { apiFailure } from './api-diagnostics.mjs';
const response = (status, path, payload) => ({
  status: () => status,
  url: () => `http://127.0.0.1:3000${path}`,
  request: () => ({ method: () => 'POST' }),
  json: async () => payload,
});
test('API diagnostics keep only status, code, method and an anonymized path', async () => {
  const record = await apiFailure(
    response(
      503,
      '/api/v1/cases/aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa/documents/bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb/versions/1/verify?token=secret',
      { error: { code: 'server_busy', detail: 'private details' }, access_token: 'secret' },
    ),
  );
  assert.deepEqual(record, {
    method: 'POST',
    pathname: '/api/v1/cases/:id/documents/:id/versions/1/verify',
    status: 503,
    code: 'server_busy',
  });
});
test('API diagnostics ignore successes and assets and reject unsafe codes', async () => {
  assert.equal(await apiFailure(response(200, '/api/v1/cases', {})), null);
  assert.equal(await apiFailure(response(404, '/private.js?token=secret', {})), null);
  assert.equal(
    (await apiFailure(response(500, '/api/v1/cases', { error: { code: 'unsafe\nsecret' } }))).code,
    'unknown',
  );
});

test('API diagnostics tolerate null and malformed error JSON', async () => {
  assert.equal((await apiFailure(response(500, '/api/v1/cases', null))).code, 'unknown');
  const malformed = response(503, '/api/v1/cases', null);
  malformed.json = async () => {
    throw new SyntaxError('malformed JSON');
  };
  assert.equal((await apiFailure(malformed)).code, 'unknown');
});
